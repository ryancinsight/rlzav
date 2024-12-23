use crate::constants::*;
use std::arch::x86_64::*;

// Add HashTable struct
#[derive(Default)]
struct HashTable {
    data: Vec<u8>,
    mask: usize,
}

// Implement methods for HashTable
impl HashTable {
    fn new(size: usize) -> Self {
        let size = size.next_power_of_two();
        Self {
            data: vec![0; size],
            mask: (size - 1) ^ 15,
        }
    }

    #[inline(always)]
    unsafe fn get_entry_unchecked(&mut self, hash: u32) -> &mut [u8] {
        let offset = (hash & self.mask as u32) as usize;
        std::slice::from_raw_parts_mut(
            self.data.as_mut_ptr().add(offset),
            16
        )
    }
}

#[inline(always)]
pub fn lzav_decompress(src: &[u8], dst: &mut [u8], expected_len: usize) -> Result<usize, i32> {
    if src.is_empty() || dst.is_empty() || expected_len > dst.len() {
        return Err(LZAV_E_PARAMS);
    }

    if expected_len > LZAV_WIN_LEN {
        return Err(LZAV_E_PARAMS);
    }

    if src.len() < 2 {
        return Err(LZAV_E_SRCOOB);
    }

    let fmt = src[0] >> 4;
    if fmt < LZAV_FMT_MIN {
        return Err(LZAV_E_UNKFMT);
    }

    let mut ip = 1;  // Input pointer
    let mut op = 0;  // Output pointer

    while ip < src.len() && op < expected_len {
        let bh = src[ip];
        ip += 1;

        if (bh & 0x30) == 0 {
            // Literal block
            let mut cc = (bh & 15) as usize;
            if cc == 0 {
                if ip >= src.len() {
                    return Err(LZAV_E_SRCOOB);
                }
                cc = 16 + src[ip] as usize;
                ip += 1;
            }
            
            if ip + cc > src.len() || op + cc > expected_len {
                return Err(LZAV_E_SRCOOB);
            }

            #[cfg(target_arch = "x86_64")]
            unsafe {
                // Use SIMD for larger copies
                if cc >= 32 {
                    let mut copied = 0;
                    while copied + 32 <= cc {
                        let src_ptr = src[ip + copied..].as_ptr() as *const __m256i;
                        let dst_ptr = dst[op + copied..].as_mut_ptr() as *mut __m256i;
                        _mm256_storeu_si256(dst_ptr, _mm256_loadu_si256(src_ptr));
                        copied += 32;
                    }
                    // Copy remaining bytes
                    if copied < cc {
                        dst[op + copied..op + cc].copy_from_slice(&src[ip + copied..ip + cc]);
                    }
                } else if cc >= 16 {
                    let src_ptr = src[ip..].as_ptr() as *const __m128i;
                    let dst_ptr = dst[op..].as_mut_ptr() as *mut __m128i;
                    _mm_storeu_si128(dst_ptr, _mm_loadu_si128(src_ptr));
                    if cc > 16 {
                        dst[op + 16..op + cc].copy_from_slice(&src[ip + 16..ip + cc]);
                    }
                } else {
                    dst[op..op + cc].copy_from_slice(&src[ip..ip + cc]);
                }
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                dst[op..op + cc].copy_from_slice(&src[ip..ip + cc]);
            }

            ip += cc;
            op += cc;
            continue;
        }

        // Match block
        let bt = ((bh >> 4) & 3) as usize;
        if bt == 0 {
            return Err(LZAV_E_UNKFMT);
        }

        let mut next_ip = ip + bt;
        let mut dist = ((bh as usize) >> 6) as usize;
        for i in 0..bt {
            dist = (dist << 8) | (src[ip + i] as usize);
        }

        let mut match_len = (bh & 15) as usize;
        if match_len == 0 {
            if next_ip >= src.len() {
                return Err(LZAV_E_SRCOOB);
            }
            match_len = 16 + src[next_ip] as usize;
            next_ip += 1;
        }
        match_len += LZAV_REF_MIN;

        if dist == 0 || dist > op || op + match_len > expected_len {
            return Err(LZAV_E_REFOOB);
        }

        let match_src = op - dist;
        
        #[cfg(target_arch = "x86_64")]
        unsafe {
            if match_len >= 32 && dist >= 32 {
                let mut copied = 0;
                while copied + 32 <= match_len {
                    let src_ptr = dst[match_src + copied..].as_ptr() as *const __m256i;
                    let dst_ptr = dst[op + copied..].as_mut_ptr() as *mut __m256i;
                    _mm256_storeu_si256(dst_ptr, _mm256_loadu_si256(src_ptr));
                    copied += 32;
                }
                if copied < match_len {
                    for i in copied..match_len {
                        dst[op + i] = dst[match_src + i];
                    }
                }
            } else if match_len >= 16 && dist >= 16 {
                let src_ptr = dst[match_src..].as_ptr() as *const __m128i;
                let dst_ptr = dst[op..].as_mut_ptr() as *mut __m128i;
                _mm_storeu_si128(dst_ptr, _mm_loadu_si128(src_ptr));
                for i in 16..match_len {
                    dst[op + i] = dst[match_src + i];
                }
            } else {
                for i in 0..match_len {
                    dst[op + i] = dst[match_src + i];
                }
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            for i in 0..match_len {
                dst[op + i] = dst[match_src + i];
            }
        }

        ip = next_ip;
        op += match_len;
    }

    if op != expected_len {
        return Err(LZAV_E_DSTLEN);
    }

    Ok(op)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompress_partial() {
        let src = [0x21, 0x04, 1, 2, 3, 4];
        let mut dst = [0u8; 10];
        let written = lzav_decompress(&src, &mut dst, 4).unwrap();
        assert_eq!(written, 4);
        assert_eq!(&dst[..4], &[1, 2, 3, 4]);
    }
}

#[repr(C)]
struct HashEntry {
    value: u64,
    position: u64,
}

#[cfg(target_feature = "avx2")]
unsafe fn copy_block(dst: &mut [u8], src: &[u8], len: usize) {
    // AVX2-optimized copy
}