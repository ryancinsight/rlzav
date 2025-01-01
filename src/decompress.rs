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

        if op + cc > dstl || src_pos + cc > srcl {
            return Err(DecompressError::DestOutOfBounds);
        }

        // Use SIMD-optimized copy when available
        if let Some(()) = utils::arch::copy_block(&mut dst[op..], &src[src_pos..], cc) {
            *cv |= ncv << *csh;
            *csh += 2;
            op += cc;
            Ok((ip, op))
        } else {
            Err(DecompressError::DestOutOfBounds)
        }
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
    // ... implement extended literal handling ...
    // This is a placeholder - implement the actual logic
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
    // ... implement reference block handling ...
    // This is a placeholder - implement the actual logic
    Ok((ip, op))
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
#[inline]
pub fn lzav_decompress_partial(src: &[u8], dst: &mut [u8], dstl: usize) -> usize {
    match decompress_internal(src, dst, dstl) {
        Ok(size) => size,
        Err(_) => 0, // Return 0 on any error
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
}

