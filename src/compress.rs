use crate::constants::*;
use crate::utils;
use std::convert::TryInto;

struct HashTable {
    data: Vec<u32>,
    mask: u32,
}

impl HashTable {
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
        &mut self.data[offset..offset + 4]
    }

    #[inline(always)]
    fn update_entry(&mut self, offset: usize, value: u32, pos: u32) {
        if offset + 2 <= self.data.len() {
            self.data[offset] = value;
            self.data[offset + 1] = pos;
        }
    }
}

#[inline]
pub fn lzav_compress(src: &[u8], dst: &mut [u8], ext_buf: Option<&mut [u8]>) -> Result<usize, i32> {
    if src.len() > LZAV_WIN_LEN || dst.len() < src.len() {
        return Err(LZAV_E_PARAMS);
    }

    dst[0] = (LZAV_FMT_CUR << 4 | LZAV_REF_MIN as u8) as u8;
    let mut op = 1;

    if src.len() < LZAV_MIN_COMPRESS_SIZE {
        return write_short_data(src, dst, op);
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

    write_final_block(dst, op, &src[literals_anchor..], src.len() - literals_anchor)
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
    let mut best_len = 0;
    let mut best_dist = 0;
    
    for i in (0..4).step_by(2) {
        if i + 2 <= hash_entry.len() {
            let pos = hash_entry[i + 1] as usize;
            
            if pos < ip {
                let dist = ip - pos;
                if dist <= LZAV_WIN_LEN && ip + dist <= src.len() {
                    let match_len = utils::lzav_match_len(
                        &src[ip..],
                        &src[pos..],
                        std::cmp::min(src.len() - ip, dist)
                    );
                    if match_len > best_len {
                        best_len = match_len;
                        best_dist = dist;
                    }
                }
            }
        }
    }

    if best_len >= LZAV_REF_MIN {
        let mut back_len = 0;
        if ip > literals_anchor {
            let max_back = std::cmp::min(ip - literals_anchor, best_dist);
            if ip >= max_back && ip - best_dist >= max_back {
                back_len = utils::lzav_match_len(
                    &src[ip - max_back..ip],
                    &src[ip - best_dist - max_back..ip - best_dist],
                    max_back
                );
            }
        }
        (true, best_len + back_len, best_dist)
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
) -> Result<usize, i32> {
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
            
            let mut temp_lcw = lcw;
            while temp_lcw > 127 {
                dst[op] = (0x80 | (temp_lcw & 0x7F)) as u8;
                temp_lcw >>= 7;
                op += 1;
            }
            dst[op] = temp_lcw as u8;
            op += 1;

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
}
