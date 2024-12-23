use crate::constants::*;

#[inline(always)]
fn lzav_match_len(p1: &[u8], p2: &[u8], ml: usize) -> usize {
    let p1s = p1.as_ptr();
    let p1e = unsafe { p1s.add(ml) };
    let mut p1p = p1s;
    let p2p = p2.as_ptr();

    #[cfg(target_arch = "x86_64")]
    unsafe {
        // Process 8 bytes at a time using 64-bit operations
        while p1p.add(7) < p1e {
            let v1: u64 = std::ptr::read_unaligned(p1p as *const u64);
            let v2: u64 = std::ptr::read_unaligned(p2p.add(p1p.offset_from(p1s) as usize) as *const u64);
            let vd = v1 ^ v2;

            if vd != 0 {
                #[cfg(target_feature = "bmi1")]
                {
                    return p1p.offset_from(p1s) as usize + (vd.trailing_zeros() as usize >> 3);
                }
                #[cfg(not(target_feature = "bmi1"))]
                {
                    let m = 0x0101010101010101u64;
                    let offset = ((((vd ^ (vd - 1)) & (m - 1)) * m) >> 56) as usize;
                    return p1p.offset_from(p1s) as usize + offset;
                }
            }
            p1p = p1p.add(8);
        }

        // Handle remaining bytes with 32-bit operations
        while p1p.add(3) < p1e {
            let v1: u32 = std::ptr::read_unaligned(p1p as *const u32);
            let v2: u32 = std::ptr::read_unaligned(p2p.add(p1p.offset_from(p1s) as usize) as *const u32);
            let vd = v1 ^ v2;

            if vd != 0 {
                let offset = unsafe { p1p.offset_from(p1s) } as usize;
                return offset + (vd.trailing_zeros() as usize >> 3);
            }
            p1p = p1p.add(4);
        }
    }

    // Handle remaining bytes
    while p1p < p1e {
        if unsafe { *p1p != *p2p.add(p1p.offset_from(p1s) as usize) } {
            return unsafe { p1p.offset_from(p1s) } as usize;
        }
        p1p = unsafe { p1p.add(1) };
    }

    ml
}

struct HashTable {
    data: Vec<u8>,
    mask: usize,
}

impl HashTable {
    fn new(size: usize) -> Self {
        // Use next power of two for efficient modulo operations
        let size = size.next_power_of_two();
        Self {
            data: vec![0; size],
            // Use bitwise operations for faster modulo
            mask: (size - 1) ^ 15,
        }
    }

    #[inline(always)]
    fn get_entry(&mut self, hash: u32) -> &mut [u8] {
        // Use bitwise AND instead of modulo for faster indexing
        let offset = (hash & self.mask as u32) as usize;
        &mut self.data[offset..offset + 16]
    }

    #[inline(always)]
    fn update_entry(&mut self, hash: u32, value: u64) {
        let offset = (hash & self.mask as u32) as usize;
        
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::*;
            let value_reg = _mm_set_epi64x(0, value as i64);
            _mm_storeu_si128(
                self.data.as_mut_ptr().add(offset) as *mut __m128i,
                value_reg
            );
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            self.data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }
    }
}

pub struct Compressor<const WIN_LEN: usize = { 1 << 23 }, const REF_MIN: usize = 6> {
    hash_table: HashTable,
    literals_anchor: usize,
    current_pos: usize,
}

impl<const WIN_LEN: usize, const REF_MIN: usize> Compressor<WIN_LEN, REF_MIN> {
    // Implementation with compile-time constants
}

pub fn lzav_compress(
    src: &[u8],
    dst: &mut [u8],
    ext_buf: Option<&mut [u8]>,
) -> Result<usize, i32> {
    if src.is_empty() || dst.is_empty() || src.len() > dst.len() {
        return Err(LZAV_E_PARAMS);
    }

    let srcl = src.len();
    
    // Ensure minimum size
    if srcl < LZAV_MIN_COMPRESS_SIZE {
        // Write format byte
        dst[0] = LZAV_FMT_CUR << 4;
        let mut op = 1;
        
        // Write length and data
        dst[op] = srcl as u8;
        op += 1;
        dst[op..op.wrapping_add(srcl)].copy_from_slice(src);
        op += srcl;
        
        return Ok(op);
    }

    // Limit maximum input size to prevent hangs
    if srcl > LZAV_WIN_LEN {
        return Err(LZAV_E_PARAMS);
    }

    // More efficient hash table size calculation
    let htsize = (1 << 10).min(srcl.next_power_of_two());
    
    let mut hash_table = if let Some(buf) = ext_buf {
        HashTable {
            data: if buf.len() >= htsize { buf.to_vec() } else { vec![0; htsize] },
            mask: (htsize - 1) ^ 15,
        }
    } else {
        HashTable::new(htsize)
    };

    let mut ip = 16.min(srcl); // Prevent overflow
    let mut literals_anchor = 0;
    let mut cv = 0usize;
    let mut csh = 0i32;
    let mut mavg = 100 << 21;
    let mut rndb = 0;

    // Add maximum iteration guard
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 1_000_000; // Reasonable limit

    // Write prefix byte
    dst[0] = (LZAV_FMT_CUR << 4 | LZAV_REF_MIN as u8) as u8;
    let mut op = 1;

    if srcl < 16 {
        // Handle very short data
        dst[op] = srcl as u8;
        op += 1;
        dst[op..op + srcl].copy_from_slice(&src[..srcl]);
        op += srcl;

        if srcl < LZAV_LIT_FIN {
            dst[op..op.wrapping_add(LZAV_LIT_FIN).wrapping_sub(srcl)].fill(0);
            op += LZAV_LIT_FIN.wrapping_sub(srcl);
        }
        return Ok(op);
    }

    // Main compression loop
    while ip < srcl.saturating_sub(LZAV_LIT_FIN) {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            return Err(LZAV_E_PARAMS);
        }
        // Hash calculation using komihash-style construct
        let mut iw1: u32 = 0;
        let mut iw2: u16 = 0;
        unsafe {
            std::ptr::copy_nonoverlapping(
                src.as_ptr().add(ip),
                &mut iw1 as *mut u32 as *mut u8,
                4,
            );
            std::ptr::copy_nonoverlapping(
                src.as_ptr().add(ip + 4),
                &mut iw2 as *mut u16 as *mut u8,
                2,
            );
        }

        let seed1 = 0x243F6A88 ^ iw1;
        let hm = (seed1 as u64).wrapping_mul((0x85A308D3 ^ iw2 as u32) as u64);
        let hval = (hm ^ (hm >> 32)) as u32;

        // Hash table access
        let hp = hash_table.get_entry(hval);
        let _ipo = ip;

        // Find match in hash table
        let mut best_match_len = 0;
        let mut best_match_dist = 0;
        let mut _best_match_pos = 0;

        // Check both hash table entries
        for i in (0..16).step_by(8) {
            let mut hw1: u32 = 0;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    hp.as_ptr().add(i),
                    &mut hw1 as *mut u32 as *mut u8,
                    4,
                );
            }

            if hw1 == iw1 {
                let pos = u32::from_le_bytes(hp[i + 4..i + 8].try_into().unwrap()) as usize;
                if pos < ip {
                    let dist = ip - pos;
                    if dist <= LZAV_WIN_LEN {
                        let match_len = lzav_match_len(
                            &src[ip..],
                            &src[pos..],
                            std::cmp::min(srcl - ip, dist),
                        );
                        if match_len > best_match_len {
                            best_match_len = match_len;
                            best_match_dist = dist;
                            _best_match_pos = pos;
                        }
                    }
                }
            }
        }

        // Update hash table
        let entry_pos = if rndb != 0 { 8 } else { 0 };
        unsafe {
            std::ptr::copy_nonoverlapping(
                &iw1 as *const u32 as *const u8,
                hash_table.data.as_mut_ptr().add(entry_pos),
                4,
            );
            std::ptr::copy_nonoverlapping(
                &(ip as u32).to_le_bytes() as *const [u8; 4] as *const u8,
                hash_table.data.as_mut_ptr().add(entry_pos + 4),
                4,
            );
        }

        // Process match or literal
        if best_match_len >= LZAV_REF_MIN {
            // Write literals if any
            let lit_len = ip - literals_anchor;
            if lit_len > 0 {
                op = write_literals(dst, op, &src[literals_anchor..ip], lit_len, &mut cv, &mut csh)?;
            }

            // Write match reference
            op = write_match_reference(
                dst,
                op,
                best_match_dist,
                best_match_len,
                &mut cv,
                &mut csh,
            )?;

            ip += best_match_len;
            literals_anchor = ip;

            // Update statistics
            mavg = (mavg * 127 + ((best_match_len as i32) << 21)) >> 7;
            rndb ^= 1;
        } else {
            ip += 1;
        }
    }

    // Write remaining literals
    let remaining_len = srcl - literals_anchor;
    if remaining_len > 0 {
        op = write_literals(
            dst,
            op,
            &src[literals_anchor..srcl],
            remaining_len,
            &mut cv,
            &mut csh,
        )?;
    }

    Ok(op)
}

// Helper function to write literals block
#[inline(always)]
fn write_literals(
    dst: &mut [u8],
    mut op: usize,
    literals: &[u8],
    len: usize,
    cv: &mut usize,
    csh: &mut i32,
) -> Result<usize, i32> {
    // Fast path for small literals (most common case)
    if len < 16 {
        // Write header and data in one operation using a small stack buffer
        let mut buf = [0u8; 17];  // 1 byte header + max 16 bytes data
        buf[0] = ((*cv << 6) | len) as u8;
        buf[1..=len].copy_from_slice(&literals[..len]);
        dst[op..op + len + 1].copy_from_slice(&buf[..len + 1]);
        op += len + 1;
    } else {
        // Extended literal block
        dst[op] = (*cv << 6) as u8;
        op += 1;
        
        // Optimize length encoding using a lookup table for common cases
        const LEN_ENCODINGS: [(u8, u8); 4] = [
            (16, 0),      // len == 16
            (17, 1),      // len == 17
            (18, 2),      // len == 18
            (19, 3),      // len == 19
        ];
        
        let len_remaining = len - 16;
        if len_remaining < LEN_ENCODINGS.len() {
            // Fast path for common small lengths
            dst[op] = LEN_ENCODINGS[len_remaining].1;
            op += 1;
        } else {
            // Optimize length encoding for larger values
            let mut len_remaining = len_remaining;
            if len_remaining < 128 {
                // Fast path for medium lengths
                dst[op] = len_remaining as u8;
                op += 1;
            } else {
                // Use a small stack buffer for length bytes
                let mut len_bytes = [0u8; 4];  // Max 4 length bytes needed
                let mut len_byte_count = 0;
                
                while len_remaining > 127 {
                    len_bytes[len_byte_count] = 0x80 | (len_remaining as u8);
                    len_remaining >>= 7;
                    len_byte_count += 1;
                }
                len_bytes[len_byte_count] = len_remaining as u8;
                len_byte_count += 1;
                
                // Write all length bytes at once
                dst[op..op + len_byte_count].copy_from_slice(&len_bytes[..len_byte_count]);
                op += len_byte_count;
            }
        }

        // Use SIMD for large copies when available
        #[cfg(target_arch = "x86_64")]
        {
            if len >= 32 {
                use std::arch::x86_64::*;
                let mut written = 0;
                while written + 32 <= len {
                    unsafe {
                        let src_ptr = literals[written..].as_ptr() as *const __m256i;
                        let dst_ptr = dst[op + written..].as_mut_ptr() as *mut __m256i;
                        _mm256_storeu_si256(dst_ptr, _mm256_loadu_si256(src_ptr));
                    }
                    written += 32;
                }
                // Copy remaining bytes
                if written < len {
                    dst[op + written..op + len].copy_from_slice(&literals[written..]);
                }
            } else {
                dst[op..op + len].copy_from_slice(literals);
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            dst[op..op + len].copy_from_slice(literals);
        }
        
        op += len;
    }

    // Clear state
    *cv = 0;
    *csh = 0;
    Ok(op)
}

// Helper function to write match reference block
#[inline(always)]
fn write_match_reference(
    dst: &mut [u8],
    mut op: usize,
    dist: usize,
    len: usize,
    cv: &mut usize,
    csh: &mut i32,
) -> Result<usize, i32> {
    let len_adj = len - LZAV_REF_MIN;
    let bt = 1 + (dist > (1 << 10) - 1) as usize + (dist > (1 << 18) - 1) as usize;
    
    // Use a stack buffer for block header to minimize memory operations
    let mut header_buf = [0u8; 5];  // Max header size is 4 bytes + length byte
    
    // Write block header
    let mut header = ((dist << 6) | (bt << 4)) as u32;
    if len_adj < 16 {
        // Fast path for short matches
        header |= len_adj as u32;
        header_buf[..bt + 1].copy_from_slice(&header.to_le_bytes()[..bt + 1]);
        dst[op..op + bt + 1].copy_from_slice(&header_buf[..bt + 1]);
        op += bt + 1;
    } else {
        // Extended match length
        header_buf[..bt + 1].copy_from_slice(&header.to_le_bytes()[..bt + 1]);
        dst[op..op + bt + 1].copy_from_slice(&header_buf[..bt + 1]);
        op += bt + 1;
        
        // Optimize length encoding for common cases
        const LEN_ENCODINGS: [(usize, u8); 4] = [
            (16, 0),      // len_adj == 16
            (271, 255),   // len_adj == 271 (16 + 255)
            (270, 254),   // len_adj == 270
            (269, 253),   // len_adj == 269
        ];
        
        if let Some(&(_, encoded)) = LEN_ENCODINGS.iter().find(|&&(l, _)| l == len_adj) {
            dst[op] = encoded;
            op += 1;
        } else if len_adj < 16 + 255 {
            // Common case: length fits in one byte
            dst[op] = (len_adj - 16) as u8;
            op += 1;
        } else {
            // Two-byte length encoding
            dst[op] = 255;
            dst[op + 1] = (len_adj - 16 - 255) as u8;
            op += 2;
        }
    }

    // Update carry values using lookup table
    static OFFSET_CARRY_SHIFTS: [i32; 4] = [0, 0, 0, 3];
    *cv = if bt == 3 { dist >> 21 } else { 0 };
    *csh = OFFSET_CARRY_SHIFTS[bt];

    Ok(op)
}

// Add a new helper function for hash calculation
#[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
fn calculate_hash(data: &[u8], pos: usize) -> u32 {
    unsafe {
        use std::arch::x86_64::*;
        let data_ptr = data[pos..].as_ptr();
        let value = _mm_loadu_si128(data_ptr as *const __m128i);
        let extracted = _pext_u64(_mm_cvtsi128_si64(value) as u64, 0x0F0F0F0F0F0F0F0F);
        extracted as u32
    }
}

// Add a new helper function for match finding
#[inline(always)]
fn find_best_match(
    src: &[u8],
    ip: usize,
    hash_entry: &[u8],
    srcl: usize,
) -> (usize, usize, usize) {
    let mut best_match_len = 0;
    let mut best_match_dist = 0;
    let mut best_match_pos = 0;

    // Check both hash table entries using SIMD when possible
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::*;
        let curr_data = _mm_loadu_si128(src[ip..].as_ptr() as *const __m128i);
        
        for i in (0..16).step_by(8) {
            let entry_data = _mm_loadu_si128(hash_entry[i..].as_ptr() as *const __m128i);
            let mask = _mm_movemask_epi8(_mm_cmpeq_epi8(curr_data, entry_data));
            
            if mask != 0 {
                let pos = u32::from_le_bytes(hash_entry[i + 4..i + 8].try_into().unwrap()) as usize;
                if pos < ip {
                    let dist = ip - pos;
                    if dist <= LZAV_WIN_LEN {
                        let match_len = lzav_match_len(
                            &src[ip..],
                            &src[pos..],
                            std::cmp::min(srcl - ip, dist),
                        );
                        if match_len > best_match_len {
                            best_match_len = match_len;
                            best_match_dist = dist;
                            best_match_pos = pos;
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        for i in (0..16).step_by(8) {
            let mut hw1: u32 = 0;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    hash_entry[i..].as_ptr(),
                    &mut hw1 as *mut u32 as *mut u8,
                    4,
                );
            }
            
            let curr_val = unsafe {
                let mut val: u32 = 0;
                std::ptr::copy_nonoverlapping(
                    src[ip..].as_ptr(),
                    &mut val as *mut u32 as *mut u8,
                    4,
                );
                val
            };

            if hw1 == curr_val {
                let pos = u32::from_le_bytes(hash_entry[i + 4..i + 8].try_into().unwrap()) as usize;
                if pos < ip {
                    let dist = ip - pos;
                    if dist <= LZAV_WIN_LEN {
                        let match_len = lzav_match_len(
                            &src[ip..],
                            &src[pos..],
                            std::cmp::min(srcl - ip, dist),
                        );
                        if match_len > best_match_len {
                            best_match_len = match_len;
                            best_match_dist = dist;
                            best_match_pos = pos;
                        }
                    }
                }
            }
        }
    }

    (best_match_len, best_match_dist, best_match_pos)
}
#[inline(always)]
fn write_block(dst: &mut [u8], op: &mut usize, literals: &[u8], len: usize) -> Result<(), i32> {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::*;
        if len >= 32 {
            let mut written = 0;
            while written + 32 <= len {
                let src_ptr = literals[written..].as_ptr() as *const __m256i;
                let dst_ptr = dst[*op + written..].as_mut_ptr() as *mut __m256i;
                _mm256_storeu_si256(dst_ptr, _mm256_loadu_si256(src_ptr));
                written += 32;
            }
            // Copy remaining bytes
            if written < len {
                dst[*op + written..*op + len].copy_from_slice(&literals[written..]);
            }
        } else {
            dst[*op..*op + len].copy_from_slice(literals);
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        dst[*op..*op + len].copy_from_slice(literals);
    }
    *op += len;
    Ok(())
}

#[repr(align(64))]  // Cache line alignment
struct AlignedBuffer {
    data: Vec<u8>,
}

impl AlignedBuffer {
    fn new(size: usize) -> Self {
        let mut vec = Vec::with_capacity(size);
        vec.resize(size, 0);
        Self { data: vec }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lzav_decompress;

    #[test]
    fn test_compress_short_data() {
        let src = [1, 2, 3];
        let mut dst = [0u8; 16];
        let compressed_size = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(compressed_size > 0);
        
        let mut decompressed = [0u8; 3];
        let decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, 3).unwrap();
        assert_eq!(decompressed_size, 3);
        assert_eq!(&decompressed[..3], &src);
    }

    #[test]
    fn test_compress_longer_data() {
        let src = [0x01, 0x02, 0x03, 0x01, 0x02, 0x03];
        let mut dst = [0u8; 32];
        let compressed_size = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(compressed_size > 0);
    }
}

