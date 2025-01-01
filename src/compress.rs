use crate::constants::*;
use crate::utils;
use std::convert::TryInto;

#[derive(Debug)]
pub enum CompressError {
    InvalidParams,
    BufferTooSmall,
}

impl From<CompressError> for i32 {
    fn from(error: CompressError) -> Self {
        match error {
            CompressError::InvalidParams => LZAV_E_PARAMS,
            CompressError::BufferTooSmall => LZAV_E_PARAMS,
        }
    }
}

#[derive(Debug)]
struct HashTable {
    data: Vec<u32>,
    mask: u32,
}

impl HashTable {
    #[inline(always)]
    fn new(size: usize) -> Self {
        let size = size.next_power_of_two();
        Self {
            data: vec![0; size],
            mask: (size as u32 - 1) ^ 15,
        }
    }

    #[inline(always)]
    fn get_entry(&mut self, hash: u32) -> &mut [u32] {
        let offset = (hash & self.mask) as usize;
        if offset + 4 > self.data.len() {
            &mut self.data[0..4]  // Fallback to start of buffer if out of bounds
        } else {
            &mut self.data[offset..offset + 4]
        }
    }

    #[inline(always)]
    fn update_entry(&mut self, offset: usize, value: u32, pos: u32) {
        if offset + 2 <= self.data.len() {
            self.data[offset] = value;
            self.data[offset + 1] = pos;
        }
    }
}

#[inline(always)]
pub fn lzav_compress(src: &[u8], dst: &mut [u8], ext_buf: Option<&mut [u8]>) -> Result<usize, i32> {
    match lzav_compress_internal(src, dst, ext_buf) {
        Ok(size) => Ok(size),
        Err(CompressError::InvalidParams) => Err(LZAV_E_PARAMS),
        Err(CompressError::BufferTooSmall) => Err(LZAV_E_PARAMS),
    }
}

#[inline(always)]
fn lzav_compress_internal(src: &[u8], dst: &mut [u8], ext_buf: Option<&mut [u8]>) -> Result<usize, CompressError> {
    if src.len() > LZAV_WIN_LEN || dst.len() < src.len() {
        return Err(CompressError::InvalidParams);
    }

    dst[0] = (LZAV_FMT_CUR << 4 | LZAV_REF_MIN as u8) as u8;
    let mut op = 1;

    if src.len() < LZAV_MIN_COMPRESS_SIZE {
        return write_short_data(src, dst, op).map_err(|_| CompressError::BufferTooSmall);
    }

    let htsize = calculate_hash_table_size(src.len(), ext_buf.as_ref().map(|b| b.len()));
    let mut hash_table = if let Some(_buf) = ext_buf {
        HashTable {
            data: vec![0; htsize / 4],
            mask: (htsize as u32 / 4 - 1) ^ 15,
        }
    } else {
        HashTable::new(htsize / 4)
    };

    let mut ip = LZAV_MIN_COMPRESS_SIZE.min(src.len());
    let mut literals_anchor = 0;
    let mut cv = 0usize;
    let mut csh = 0i32;
    let mut mavg: i32 = 100 << 21;
    let mut rndb = 0u32;
    let mut cbp = op;

    let mut initv = [0u32; 2];
    if ip < src.len() - 9 {
        initv[0] = u32::from_le_bytes(src[0..4].try_into().unwrap());
        initv[1] = 16;
    }

    while ip < src.len() - LZAV_LIT_FIN {
        let iw1 = u32::from_le_bytes(src[ip..ip + 4].try_into().unwrap());
        let iw2 = u16::from_le_bytes(src[ip + 4..ip + 6].try_into().unwrap());

        let seed1 = 0x243F6A88u32.wrapping_sub(iw1);
        let hm = (seed1 as u64).wrapping_mul((0x85A308D3u32.wrapping_sub(iw2 as u32)) as u64);
        let hval = (hm.wrapping_shr(32).wrapping_add(hm)) as u32;

        let hash_entry = hash_table.get_entry(hval);

        let (match_found, match_len, match_dist) = find_match(
            src,
            ip,
            hash_entry,
            literals_anchor,
        );

        if match_found {
            op = write_block(
                dst,
                op,
                ip - literals_anchor,
                match_len,
                match_dist,
                &src[literals_anchor..],
                &mut cbp,
                &mut cv,
                &mut csh,
            )?;

            ip += match_len;
            literals_anchor = ip;
            mavg = ((mavg as i64 * 127 + ((match_len as i64) << 21)) >> 7) as i32;
            rndb ^= 1;
            continue;
        }

        hash_table.update_entry((hval & hash_table.mask) as usize, iw1, ip as u32);

        mavg -= mavg >> 11;
        if mavg < (200 << 14) && ip != literals_anchor {
            ip += 1 + (rndb & 1) as usize;
            rndb = (ip as u32) & 1;

            if mavg < (130 << 14) {
                ip += 1;
                if mavg < (100 << 14) {
                    ip += 100 - (mavg >> 14) as usize;
                }
            }
        }
        ip += 1;
    }

    write_final_block(dst, op, &src[literals_anchor..], src.len() - literals_anchor).map_err(|_| CompressError::BufferTooSmall)
}

#[inline]
fn calculate_hash_table_size(srclen: usize, ext_size: Option<usize>) -> usize {
    match ext_size {
        Some(size) if size >= srclen * 4 => size.next_power_of_two(),
        _ => (1 << 10).min(srclen.next_power_of_two())
    }
}

#[inline]
fn write_short_data(src: &[u8], dst: &mut [u8], mut op: usize) -> Result<usize, i32> {
    dst[op] = src.len() as u8;
    op += 1;
    dst[op..op + src.len()].copy_from_slice(src);
    op += src.len();

    if src.len() < LZAV_LIT_FIN {
        dst[op..op + LZAV_LIT_FIN - src.len()].fill(0);
        op += LZAV_LIT_FIN - src.len();
    }

    Ok(op)
}

#[inline(always)]
fn find_match(
    src: &[u8],
    ip: usize,
    hash_entry: &[u32],
    literals_anchor: usize,
) -> (bool, usize, usize) {
    let mut best_match = (false, 0, 0);
    
    for chunk in hash_entry.chunks_exact(2) {
        if let [_value, pos] = chunk {
            let pos = *pos as usize;
            if pos < ip {
                let dist = ip - pos;
                if dist <= LZAV_WIN_LEN && ip + dist <= src.len() {
                    let match_len = utils::lzav_match_len(
                        &src[ip..],
                        &src[pos..],
                        src.len().saturating_sub(ip).min(dist)
                    );
                    if match_len > best_match.1 {
                        best_match = (true, match_len, dist);
                    }
                }
            }
        }
    }

    if best_match.1 >= LZAV_REF_MIN {
        let mut back_len = 0;
        if ip > literals_anchor {
            let max_back = std::cmp::min(ip - literals_anchor, best_match.2);
            if ip >= max_back && ip - best_match.2 >= max_back {
                back_len = utils::lzav_match_len(
                    &src[ip - max_back..ip],
                    &src[ip - best_match.2 - max_back..ip - best_match.2],
                    max_back
                );
            }
        }
        (true, best_match.1 + back_len, best_match.2)
    } else {
        (false, 0, 0)
    }
}

#[inline]
fn write_block(
    dst: &mut [u8],
    mut op: usize,
    lit_len: usize,
    ref_len: usize,
    dist: usize,
    literals: &[u8],
    cbp: &mut usize,
    cv: &mut usize,
    csh: &mut i32,
) -> Result<usize, CompressError> {
    if lit_len > 0 {
        let ncv = (*cv & 3) << 6;
        *cv >>= 2;

        if lit_len < 16 {
            dst[op] = ncv as u8 | lit_len as u8;
            op += 1;
            dst[op..op + lit_len].copy_from_slice(&literals[..lit_len]);
            op += lit_len;
        } else {
            let lcw = lit_len - 16;
            dst[op] = ncv as u8;
            op += 1;
            
            op = write_varint(dst, lcw, op);

            dst[op..op + lit_len].copy_from_slice(&literals[..lit_len]);
            op += lit_len;
        }
    }

    let ref_len_adj = ref_len - LZAV_REF_MIN;
    let bt = 1 + (dist > (1 << 10) - 1) as usize + (dist > (1 << 18) - 1) as usize;
    
    if ref_len_adj < 16 {
        let header = ((dist << 6) | (bt << 4) | ref_len_adj) as u32;
        dst[op..op + 4].copy_from_slice(&header.to_le_bytes());
        op += bt;
        dst[op] = ((header >> (bt * 8)) & 0xFF) as u8;
        op += 1;
    } else {
        let header = ((dist << 6) | (bt << 4)) as u32;
        dst[op..op + 4].copy_from_slice(&header.to_le_bytes());
        op += bt;
        dst[op] = 0;
        op += 1;
        
        if ref_len_adj < 16 + 255 {
            dst[op] = (ref_len_adj - 16) as u8;
            op += 1;
        } else {
            dst[op] = 255;
            dst[op + 1] = (ref_len_adj - 16 - 255) as u8;
            op += 2;
        }
    }

    *cv = if bt == 3 { dist >> 21 } else { 0 };
    *csh = if bt == 3 { 3 } else { 0 };
    *cbp = op - 1;

    Ok(op)
}

#[inline(always)]
fn write_varint(dst: &mut [u8], mut value: usize, mut pos: usize) -> usize {
    while value > 127 {
        dst[pos] = (0x80 | (value & 0x7F)) as u8;
        value >>= 7;
        pos += 1;
    }
    dst[pos] = value as u8;
    pos + 1
}

#[inline]
fn write_final_block(dst: &mut [u8], mut op: usize, literals: &[u8], lit_len: usize) -> Result<usize, i32> {
    if lit_len < 16 {
        dst[op] = lit_len as u8;
        op += 1;
    } else {
        dst[op] = 0;
        op += 1;
        
        let lcw = lit_len - 16;
        let mut temp_lcw = lcw;
        while temp_lcw > 127 {
            dst[op] = (0x80 | (temp_lcw & 0x7F)) as u8;
            temp_lcw >>= 7;
            op += 1;
        }
        dst[op] = temp_lcw as u8;
        op += 1;
    }

    dst[op..op + lit_len].copy_from_slice(literals);
    op += lit_len;

    Ok(op)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_small() {
        let src = b"Hello World!";
        let mut dst = vec![0u8; src.len() * 2];
        let res = lzav_compress(src, &mut dst, None).unwrap();
        assert!(res > 0);
    }

    #[test]
    fn test_compression_limits() {
        let src = vec![0u8; LZAV_WIN_LEN + 1];
        let mut dst = vec![0u8; src.len() * 2];
        assert!(lzav_compress(&src, &mut dst, None).is_err());
    }

    #[test]
    fn test_compression_empty() {
        let src = &[];
        let mut dst = vec![0u8; LZAV_MIN_COMPRESS_SIZE];
        let res = lzav_compress(src, &mut dst, None);
        assert!(res.is_ok());
    }

    #[test]
    fn test_compression_min_size() {
        let src = vec![0u8; LZAV_MIN_COMPRESS_SIZE];
        let mut dst = vec![0u8; LZAV_MIN_COMPRESS_SIZE * 2];
        let res = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(res >= LZAV_MIN_COMPRESS_SIZE);
    }

    #[test]
    fn test_compression_max_size() {
        let src = vec![0u8; LZAV_WIN_LEN];
        let mut dst = vec![0u8; LZAV_WIN_LEN];
        let res = lzav_compress(&src, &mut dst, None);
        assert!(res.is_ok());
    }

    #[test]
    fn test_compression_with_external_buffer() {
        let src = b"Hello World! This is a test of external buffer compression.";
        let mut dst = vec![0u8; src.len() * 2];
        let mut ext_buf = vec![0u8; 1024];
        let res = lzav_compress(src, &mut dst, Some(&mut ext_buf)).unwrap();
        assert!(res > 0 && res <= src.len() * 2);
    }

    #[test]
    fn test_compression_repeated_data() {
        let src = b"AAAAAAAAAAAAAAAAAAAAAAAAA".repeat(10);
        let mut dst = vec![0u8; src.len()];
        let res = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(res < src.len(), "Compressed size should be smaller for repeated data");
    }

    #[test]
    fn test_compression_random_data() {
        let src: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let mut dst = vec![0u8; src.len() * 2];
        let res = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(res > 0);
    }

    #[test]
    fn test_compression_error_cases() {
        // Test source too large
        let src = vec![0u8; LZAV_WIN_LEN + 1];
        let mut dst = vec![0u8; LZAV_WIN_LEN + 1];
        assert!(lzav_compress(&src, &mut dst, None).is_err());

        // Test destination too small
        let src = vec![0u8; 100];
        let mut dst = vec![0u8; 50];
        assert!(lzav_compress(&src, &mut dst, None).is_err());
    }

    #[test]
    fn test_compression_boundary_conditions() {
        // Test exact size matches
        let src = vec![0u8; LZAV_WIN_LEN];
        let mut dst = vec![0u8; LZAV_WIN_LEN + LZAV_MIN_COMPRESS_SIZE]; // Increase buffer size
        assert!(lzav_compress(&src, &mut dst, None).is_ok());

        // Test minimum size - 1
        let src = vec![0u8; LZAV_MIN_COMPRESS_SIZE - 1];
        let mut dst = vec![0u8; LZAV_MIN_COMPRESS_SIZE * 2]; // Double buffer size for safety
        let res = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(res >= src.len());
    }

    #[test]
    fn test_compression_mixed_content() {
        let mut src = Vec::with_capacity(1000);
        // Add some repeated patterns
        src.extend_from_slice(&[0xAA; 100]);
        // Add some random data
        src.extend_from_slice(&(0..100).map(|x| x as u8).collect::<Vec<u8>>());
        // Add some zeros
        src.extend_from_slice(&[0; 100]);
        
        let mut dst = vec![0u8; src.len() * 2];
        let res = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(res > 0 && res < src.len() * 2);
    }
}
