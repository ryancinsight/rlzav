//! LZAV compression library
//! 
//! This is a Rust implementation of the LZAV compression algorithm.
//! Original C implementation: https://github.com/avaneev/lzav
//!
//! Note that compression algorithm and its output on the same source data may
//! differ between LZAV versions, and may differ between big- and little-endian
//! systems. However, the decompression of a compressed data produced by any
//! prior compressor version will remain possible.

pub mod constants;
pub mod compress;
pub mod decompress;
pub mod error;
mod utils;

// Add test module
#[cfg(test)]
pub mod tests;

pub use compress::lzav_compress;
pub use decompress::{lzav_decompress, lzav_decompress_partial};
pub use error::LzavError;

// Add optimization hint macros
#[macro_export]
macro_rules! likely {
    ($x:expr) => {
        if cfg!(all(target_arch = "x86_64", target_feature = "sse2")) {
            #[allow(unused_unsafe)]
            unsafe { std::intrinsics::likely($x) }
        } else {
            $x
        }
    };
}

#[macro_export]
macro_rules! unlikely {
    ($x:expr) => {
        if cfg!(all(target_arch = "x86_64", target_feature = "sse2")) {
            #[allow(unused_unsafe)]
            unsafe { std::intrinsics::unlikely($x) }
        } else {
            $x
        }
    };
}

/// Returns the required buffer size for LZAV compression.
///
/// This function helps allocate a sufficiently large destination buffer for compression.
#[inline]
pub fn lzav_compress_bound(srcl: i32) -> i32 {
    if srcl <= 0 {
        return 16;
    }
    let k = 16 + 127 + 1;
    let l2 = srcl / (k + 6);
    (srcl - l2 * 6 + k - 1) / k * 2 - l2 + srcl + 16
}

/// Returns the required buffer size for higher-ratio LZAV compression.
///
/// Note that the higher-ratio compression is much slower than the standard compression.
#[inline]
pub fn lzav_compress_bound_hi(srcl: i32) -> i32 {
    if srcl <= 0 {
        return 16;
    }
    let l2 = srcl / (16 + 5);
    (srcl - l2 * 5 + 15) / 16 * 2 - l2 + srcl + 16
}

/// Default compression function without external buffer option.
///
/// This is a convenience wrapper around `lzav_compress` that uses default settings.
#[inline]
pub fn lzav_compress_default(src: &[u8], dst: &mut [u8]) -> Result<usize, i32> {
    lzav_compress(src, dst, None)
}
