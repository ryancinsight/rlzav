use crate::constants::*;
use crate::utils::{self};

#[derive(Debug)]
pub enum DecompressError {
    InvalidParams,
    UnknownFormat,
    SourceOutOfBounds,
    DestOutOfBounds,
    ReferenceOutOfBounds,
    DestLengthMismatch,
}

impl From<DecompressError> for i32 {
    fn from(err: DecompressError) -> i32 {
        match err {
            DecompressError::InvalidParams => LZAV_E_PARAMS,
            DecompressError::UnknownFormat => LZAV_E_UNKFMT,
            DecompressError::SourceOutOfBounds => LZAV_E_SRCOOB,
            DecompressError::DestOutOfBounds => LZAV_E_DSTOOB,
            DecompressError::ReferenceOutOfBounds => LZAV_E_REFOOB,
            DecompressError::DestLengthMismatch => LZAV_E_DSTLEN,
        }
    }
}

#[inline(always)]
pub fn lzav_decompress(src: &[u8], dst: &mut [u8], dstl: usize) -> Result<usize, i32> {
    match decompress_internal(src, dst, dstl) {
        Ok(size) => Ok(size),
        Err(e) => Err(e.into()),
    }
}

#[inline(always)]
fn decompress_internal(src: &[u8], dst: &mut [u8], dstl: usize) -> Result<usize, DecompressError> {
    if src.is_empty() {
        return if dstl == 0 { 
            Ok(0) 
        } else { 
            Err(DecompressError::InvalidParams) 
        };
    }

    if dst.is_empty() || dstl == 0 {
        return Err(DecompressError::InvalidParams);
    }

    let fmt = src[0] >> 4;
    match fmt {
        2 => decompress_fmt2(src, dst, src.len(), dstl),
        #[cfg(feature = "format1")]
        1 => decompress_fmt1(src, dst, src.len(), dstl),
        _ => Err(DecompressError::UnknownFormat)
    }
}

#[inline(always)]
fn decompress_fmt2(src: &[u8], dst: &mut [u8], srcl: usize, dstl: usize) -> Result<usize, DecompressError> {
    if srcl < 6 {
        return Err(DecompressError::SourceOutOfBounds);
    }

    let mut ip = 1;
    let mut op = 0;
    let mref1 = (src[0] & 15) as usize - 1;
    let mut cv = 0;
    let mut csh = 0;

    while ip < srcl - 6 {
        let bh = src[ip] as usize;
        
        if (bh & 0x30) == 0 {
            let (new_ip, new_op) = handle_literal_block(
                src, dst, ip, op, srcl, dstl, bh, &mut cv, &mut csh
            )?;
            ip = new_ip;
            op = new_op;
            continue;
        }

        let (new_ip, new_op) = handle_reference_block(
            src, dst, ip, op, srcl, dstl, bh, mref1, &mut cv, &mut csh
        )?;
        ip = new_ip;
        op = new_op;
    }

    if op != dstl {
        return Err(DecompressError::DestLengthMismatch);
    }

    Ok(op)
}

#[inline(always)]
fn handle_literal_block(
    src: &[u8],
    dst: &mut [u8],
    mut ip: usize,
    mut op: usize,
    srcl: usize,
    dstl: usize,
    bh: usize,
    cv: &mut usize,
    csh: &mut i32,
) -> Result<(usize, usize), DecompressError> {
    let ncv = bh >> 6;
    ip += 1;
    let mut cc = bh & 15;

    if cc != 0 {
        // Direct length encoding
        let src_pos = ip;
        ip += cc;

        // Improved bounds checking
        if src_pos + cc > srcl {
            return Err(DecompressError::SourceOutOfBounds);
        }
        if op + cc > dstl {
            return Err(DecompressError::DestOutOfBounds);
        }

        dst[op..op + cc].copy_from_slice(&src[src_pos..src_pos + cc]);
        *cv |= ncv << *csh;
        *csh += 2;
        op += cc;
        Ok((ip, op))
    } else {
        handle_extended_literal(src, dst, ip, op, srcl, dstl, ncv, cv, csh)
    }
}

#[inline(always)]
fn handle_extended_literal(
    src: &[u8],
    dst: &mut [u8],
    mut ip: usize,
    mut op: usize,
    srcl: usize,
    dstl: usize,
    ncv: usize,
    cv: &mut usize,
    csh: &mut i32,
) -> Result<(usize, usize), DecompressError> {
    if ip >= srcl {
        return Err(DecompressError::SourceOutOfBounds);
    }

    let mut cc = src[ip] as usize;
    ip += 1;

    if cc & 0x80 != 0 {
        cc &= 0x7F;
        let mut shift = 7;
        while shift < 28 {
            if ip >= srcl {
                return Err(DecompressError::SourceOutOfBounds);
            }
            let byte = src[ip] as usize;
            ip += 1;
            cc |= (byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
    }

    cc += 16;
    let src_pos = ip;
    ip += cc;

    // Improved bounds checking
    if src_pos + cc > srcl {
        return Err(DecompressError::SourceOutOfBounds);
    }
    if op + cc > dstl {
        return Err(DecompressError::DestOutOfBounds);
    }

    dst[op..op + cc].copy_from_slice(&src[src_pos..src_pos + cc]);
    *cv |= ncv << *csh;
    *csh += 2;
    op += cc;
    Ok((ip, op))
}

#[inline(always)]
fn handle_reference_block(
    src: &[u8],
    dst: &mut [u8],
    mut ip: usize,
    mut op: usize,
    srcl: usize,
    dstl: usize,
    bh: usize,
    mref1: usize,
    cv: &mut usize,
    csh: &mut i32,
) -> Result<(usize, usize), DecompressError> {
    if ip + 1 >= srcl {
        return Err(DecompressError::SourceOutOfBounds);
    }

    // Combine operations to reduce register pressure
    let ncv = bh >> 6;
    let copy_len = ((bh >> 4) & 3) + 2 + if (bh & 8) != 0 { mref1 } else { 0 };
    
    // Extract reference offset in one operation
    let oref = ((bh & 7) << 8) | src[ip + 1] as usize;
    if oref == 0 {
        return Err(DecompressError::ReferenceOutOfBounds);
    }

    // Bounds checking with single comparison
    let ref_pos = op.checked_sub(oref)
        .ok_or(DecompressError::ReferenceOutOfBounds)?;
    if ref_pos + copy_len > op || op + copy_len > dstl {
        return Err(DecompressError::DestOutOfBounds);
    }

    if op < ref_pos {
        let (left, right) = dst.split_at_mut(ref_pos);
        left[op..op + copy_len].copy_from_slice(&right[..copy_len]);
    } else {
        let (left, right) = dst.split_at_mut(op);
        right[..copy_len].copy_from_slice(&left[ref_pos..ref_pos + copy_len]);
    }

    // Update state
    *cv |= ncv << *csh;
    *csh += 2;
    Ok((ip + 2, op + copy_len))
}

#[cfg(feature = "format1")]
fn decompress_fmt1(src: &[u8], dst: &mut [u8], srcl: usize, dstl: usize) -> Result<usize, i32> {
    // Format 1 decompression implementation
    // This is optional and can be enabled via the "format1" feature
    unimplemented!("Format 1 decompression not implemented");
}

/// Decompresses data partially, useful for recovery or streaming decompression.
/// 
/// This function can be used to decompress only an initial segment of a larger data block.
/// Unlike the main decompression function, this one always returns a non-negative value
/// and does not propagate error codes.
#[inline(always)]
pub fn lzav_decompress_partial(src: &[u8], dst: &mut [u8], dstl: usize) -> usize {
    // Early return for invalid inputs with a single check
    if src.is_empty() || dst.is_empty() || dstl == 0 || src.get(0).map_or(true, |&b| b >> 4 != 2) {
        return 0;
    }

    // Pre-check buffer lengths and get first byte
    let Some(&bh) = src.get(1) else { return 0 };
    let bh = bh as usize;

    // Fast path for simple literal block
    if (bh & 0x30) == 0 {
        let cc = bh & 15;
        if cc > 0 {
            let available = src.len().saturating_sub(2);
            let copy_len = cc.min(available).min(dstl);
            if copy_len > 0 {
                dst[..copy_len].copy_from_slice(&src[2..2 + copy_len]);
                return copy_len;
            }
        }
    }

    // Fallback to full decompression with size tracking
    match decompress_fmt2(src, dst, src.len(), dstl) {
        Ok(size) => size,
        Err(_) => dst.iter().position(|&x| x == 0).unwrap_or(dstl)
    }
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

    #[test]
    fn test_decompress_boundary_conditions() {
        // Test empty input
        let mut dst = vec![0u8; 1];
        assert!(lzav_decompress(&[], &mut dst, 0).is_ok());
        assert!(lzav_decompress(&[], &mut dst, 1).is_err());

        // Test minimal valid input
        let src = [LZAV_FMT_CUR << 4 | LZAV_REF_MIN as u8, 0];
        let mut dst = vec![0u8; 1];
        assert!(lzav_decompress(&src, &mut dst, 0).is_err());
    }

    #[test]
    fn test_decompress_partial() {
        // Test partial decompression of valid data
        let mut src = vec![LZAV_FMT_CUR << 4 | LZAV_REF_MIN as u8];
        src.extend_from_slice(&[5, b'H', b'e', b'l', b'l', b'o']);
        let mut dst = vec![0u8; 3];
        let decompressed = lzav_decompress_partial(&src, &mut dst, 3);
        assert_eq!(decompressed, 3);
        assert_eq!(&dst[..3], b"Hel");
    }

    #[test]
    fn test_decompress_error_conditions() {
        let mut dst = vec![0u8; 16];

        // Test invalid format version
        let src = [0xFF; 16];
        assert!(matches!(
            lzav_decompress(&src, &mut dst, 16).unwrap_err(),
            LZAV_E_UNKFMT
        ));

        // Test source OOB
        let src = [LZAV_FMT_CUR << 4 | LZAV_REF_MIN as u8, 20];
        assert!(matches!(
            lzav_decompress(&src, &mut dst, 16).unwrap_err(),
            LZAV_E_SRCOOB
        ));

        // Test destination OOB with proper error code
        let mut src = vec![LZAV_FMT_CUR << 4 | LZAV_REF_MIN as u8];
        src.extend_from_slice(&[4, b'H', b'e', b'l', b'l']);
        assert!(matches!(
            lzav_decompress(&src, &mut dst, 32).unwrap_err(),
            LZAV_E_DSTLEN
        ));
    }

    #[test]
    fn test_decompress_reference_blocks() {
        // Create compressed data with reference blocks
        let original = b"ABCABCABCABC".to_vec();
        let mut compressed = vec![0u8; original.len() * 2];
        let compressed_size = super::super::compress::lzav_compress(
            &original,
            &mut compressed,
            None
        ).unwrap();
        compressed.truncate(compressed_size);

        // Test decompression
        let mut decompressed = vec![0u8; original.len()];
        let size = lzav_decompress(&compressed, &mut decompressed, original.len()).unwrap();
        assert_eq!(size, original.len());
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decompress_large_literals() {
        // Test decompression of data with large literal blocks
        let original: Vec<u8> = (0..255).collect();
        let mut compressed = vec![0u8; original.len() * 2];
        let compressed_size = super::super::compress::lzav_compress(
            &original,
            &mut compressed,
            None
        ).unwrap();
        compressed.truncate(compressed_size);

        let mut decompressed = vec![0u8; original.len()];
        let size = lzav_decompress(&compressed, &mut decompressed, original.len()).unwrap();
        assert_eq!(size, original.len());
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decompress_alignment() {
        // Test decompression with different memory alignments
        let original = b"Testing alignment with different offsets".to_vec();
        let mut compressed = vec![0u8; original.len() * 2];
        let compressed_size = super::super::compress::lzav_compress(
            &original,
            &mut compressed,
            None
        ).unwrap();
        compressed.truncate(compressed_size);

        // Test with different destination buffer alignments
        for offset in 0..8 {
            let mut decompressed = vec![0u8; original.len() + offset];
            let size = lzav_decompress(
                &compressed,
                &mut decompressed[offset..],
                original.len()
            ).unwrap();
            assert_eq!(size, original.len());
            assert_eq!(&decompressed[offset..offset + original.len()], &original);
        }
    }

    #[test]
    fn test_decompress_edge_cases() {
        // Test minimum reference length
        let src = vec![0u8; LZAV_REF_MIN + 1];
        let mut compressed = vec![0u8; src.len() * 2];
        let compressed_size = super::super::compress::lzav_compress(
            &src,
            &mut compressed,
            None
        ).unwrap();
        compressed.truncate(compressed_size);

        let mut decompressed = vec![0u8; src.len()];
        let size = lzav_decompress(&compressed, &mut decompressed, src.len()).unwrap();
        assert_eq!(size, src.len());
        assert_eq!(decompressed, src);
    }

    #[test]
    fn test_decompress_with_carry() {
        // Test decompression with offset carry values
        let original = b"ABCDEFABCDEFABCDEFABCDEF".to_vec();
        let mut compressed = vec![0u8; original.len() * 2];
        let compressed_size = super::super::compress::lzav_compress(
            &original,
            &mut compressed,
            None
        ).unwrap();
        compressed.truncate(compressed_size);

        let mut decompressed = vec![0u8; original.len()];
        let size = lzav_decompress(&compressed, &mut decompressed, original.len()).unwrap();
        assert_eq!(size, original.len());
        assert_eq!(decompressed, original);
    }
}

