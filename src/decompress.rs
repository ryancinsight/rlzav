use crate::constants::*;
use crate::utils::{self, AlignedBuffer};

#[inline]
pub fn lzav_decompress(src: &[u8], dst: &mut [u8], dstl: usize) -> Result<usize, i32> {
    if src.is_empty() {
        return if dstl == 0 { Ok(0) } else { Err(LZAV_E_PARAMS) };
    }

    if dst.is_empty() || dstl == 0 {
        return Err(LZAV_E_PARAMS);
    }

    let fmt = src[0] >> 4;
    match fmt {
        2 => {
            let mut written = 0;
            let res = decompress_fmt2(src, dst, src.len(), dstl, &mut written)?;
            Ok(res)
        }
        #[cfg(feature = "format1")]
        1 => decompress_fmt1(src, dst, src.len(), dstl),
        _ => Err(LZAV_E_UNKFMT)
    }
}

/// Decompresses data partially, useful for recovery or streaming decompression.
///
/// This function can be used to decompress only an initial segment of a larger data block.
/// Unlike the main decompression function, this one always returns a non-negative value
/// and does not propagate error codes.
#[inline]
pub fn lzav_decompress_partial(src: &[u8], dst: &mut [u8], dstl: usize) -> usize {
    if src.is_empty() || dst.is_empty() || dstl == 0 {
        return 0;
    }

    let mut written = 0; // Move declaration to outer scope
    let fmt = src[0] >> 4;
    if fmt == 2 {
        if let Ok(size) = decompress_fmt2(src, dst, src.len(), dstl, &mut written) {
            size
        } else {
            written
        }
    } else {
        written
    }
}

#[inline]
fn decompress_fmt2(src: &[u8], dst: &mut [u8], srcl: usize, dstl: usize, pwl: &mut usize) -> Result<usize, i32> {
    if srcl < 6 {
        return Err(LZAV_E_SRCOOB);
    }

    let mut ip = 1; // Skip format byte
    let mut op = 0;
    let mref1 = (src[0] & 15) as usize - 1;
    let mut bh = src[ip] as usize;
    let mut cv = 0;
    let mut csh = 0;

    // Load first block header
    if ip >= srcl - 6 {
        return Err(LZAV_E_SRCOOB);
    }
    bh = src[ip] as usize;

    while ip < srcl - 6 {
        if (bh & 0x30) == 0 {
            // Literal block
            let mut ncv = bh >> 6;
            ip += 1;
            let mut cc = bh & 15;

            if cc != 0 {
                // Direct length encoding
                ncv <<= csh;
                let src_pos = ip;
                ip += cc;

                if op + cc <= dstl && src_pos + cc <= srcl {
                    cv |= ncv;
                    csh += 2;
                    bh = src[ip] as usize;
                    // Use safe copy_block that returns Option
                    if utils::arch::copy_block(&mut dst[op..], &src[src_pos..], cc).is_none() {
                        return Err(LZAV_E_DSTOOB);
                    }
                    op += cc;
                    continue;
                }
            } else {
                // Extended length encoding
                let mut lcw = src[ip] as usize;
                ncv <<= csh;
                ip += 1;
                cc = lcw & 0x7F;
                let mut sh = 7;

                while (lcw & 0x80) != 0 && sh < 28 {
                    lcw = src[ip] as usize;
                    ip += 1;
                    cc |= (lcw & 0x7F) << sh;
                    sh += 7;
                }

                cc += 16;
                let src_pos = ip;
                ip += cc;

                if op + cc <= dstl && src_pos + cc <= srcl {
                    cv |= ncv;
                    csh += 2;
                    bh = src[ip] as usize;
                    utils::arch::copy_block(&mut dst[op..], &src[src_pos..], cc);
                    op += cc;
                    continue;
                }
            }

            return if ip >= srcl {
                *pwl = op;
                Err(LZAV_E_SRCOOB)
            } else if op + cc > dstl {
                dst[op..dstl].copy_from_slice(&src[ip..ip + (dstl - op)]);
                Err(LZAV_E_DSTOOB)
            } else {
                dst[op..op + cc].copy_from_slice(&src[ip..ip + cc]);
                op += cc;
                cv |= ncv;
                csh += 2;
                bh = src[ip] as usize;
                Ok(op)
            };
        }

        // Reference block
        let bt = (bh >> 4) & 3;
        ip += 1;
        let bt8 = bt << 3;

        // Load 32-bit value with endianness correction
        let mut bv = u32::from_le_bytes(src[ip..ip + 4].try_into().unwrap());
        ip += bt;
        let o = bv & ((1 << bt8) - 1);
        bv >>= bt8;

        // Reference offset calculation
        let d = match bt {
            0 => ((bh >> 6 | (o as usize) << 2) << csh) | cv,
            1 => ((bh >> 6 | ((o as usize) & 0x3FF) << 2) << csh) | cv,
            2 => ((bh >> 6 | ((o as usize) & 0x3FFFF) << 2) << csh) | cv,
            _ => ((bh >> 6 | ((o as usize) & 0x1FFFFF) << 2) << csh) | cv,
        };

        if d > op {
            eprintln!("Error: Back-reference out of bounds. d: {}, op: {}", d, op); // Debug information
            *pwl = op;
            return Err(LZAV_E_REFOOB);
        }

        let src_pos = op - d;
        let mut cc = bh & 15;

        if cc != 0 {
            cc += mref1;
            bh = bv as usize & 0xFF;
        } else {
            bh = bv as usize & 0xFF;
            if bh == 255 {
                cc = 16 + mref1 + 255 + src[ip + 1] as usize;
                bh = src[ip + 2] as usize;
                ip += 2;
            } else {
                cc = 16 + mref1 + bh;
                ip += 1;
                bh = src[ip] as usize;
            }
        }

        if op + cc > dstl {
            // Perform partial copy up to dstl
            let copy_len = dstl - op;
            for i in 0..copy_len {
                dst[op + i] = dst[src_pos + i];
            }
            return Err(LZAV_E_DSTOOB);
        }

        // Copy reference data
        for i in 0..cc {
            dst[op + i] = dst[src_pos + i];
        }
        op += cc;

        // Update carry values
        cv = if bt == 3 { (o >> 21) as usize } else { 0 };
        csh = if bt == 3 { 3 } else { 0 };
    }

    if op != dstl {
        *pwl = op;
        return Err(LZAV_E_DSTLEN);
    }

    Ok(op)
}

#[cfg(feature = "format1")]
fn decompress_fmt1(src: &[u8], dst: &mut [u8], srcl: usize, dstl: usize) -> Result<usize, i32> {
    // Format 1 decompression implementation
    // This is optional and can be enabled via the "format1" feature
    unimplemented!("Format 1 decompression not implemented");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompress_empty() {
        let mut dst = [0u8; 16];
        assert!(lzav_decompress(&[], &mut dst, 0).is_ok());
        assert!(lzav_decompress(&[], &mut dst, 1).is_err());
    }

    #[test]
    fn test_decompress_invalid_format() {
        let src = [0xFF; 16]; // Invalid format byte
        let mut dst = [0u8; 16];
        assert!(lzav_decompress(&src, &mut dst, 16).is_err());
    }

    #[test]
    fn test_decompress_truncated() {
        let src = [0x20, 0x01]; // Valid format but truncated
        let mut dst = [0u8; 16];
        assert!(lzav_decompress(&src, &mut dst, 16).is_err());
    }
}

