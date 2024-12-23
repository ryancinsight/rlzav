//! LZAV compression library
//! 
//! This is a Rust implementation of the LZAV compression algorithm.
//! Original C implementation: https://github.com/avaneev/lzav
//!
//! # Examples
//!
//! ```
//! use lzav::{lzav_compress, lzav_decompress, lzav_compress_bound};
//!
//! // Example data to compress
//! let data = b"Hello, world! This is a test string that might compress well due to repetition.";
//!
//! // Calculate required buffer size and create output buffer
//! let bound = lzav_compress_bound(data.len() as i32) as usize;
//! let mut compressed = vec![0u8; bound];
//!
//! // Compress the data
//! let compressed_size = lzav_compress(data, &mut compressed, None).unwrap();
//! compressed.truncate(compressed_size);
//!
//! // Create buffer for decompressed data
//! let mut decompressed = vec![0u8; data.len()];
//!
//! // Decompress the data
//! let decompressed_size = lzav_decompress(&compressed, &mut decompressed, data.len()).unwrap();
//! assert_eq!(decompressed_size, data.len());
//! assert_eq!(&decompressed, data);
//! ```
#[cfg(test)]
mod tests;

pub mod constants;
pub mod compress;
pub mod decompress;

pub use compress::lzav_compress;
pub use decompress::lzav_decompress;

/// Returns the required buffer size for compression
#[inline]
pub fn lzav_compress_bound(srcl: i32) -> i32 {
    if srcl <= 0 {
        return 16;
    }

    let k = 16 + 127 + 1;
    let l2 = srcl / (k + 6);

    (srcl - l2 * 6 + k - 1) / k * 2 - l2 + srcl + 16
}

/// Returns the required buffer size for higher-ratio compression
#[inline]
pub fn lzav_compress_bound_hi(srcl: i32) -> i32 {
    if srcl <= 0 {
        return 16;
    }

    let l2 = srcl / (16 + 5);

    (srcl - l2 * 5 + 15) / 16 * 2 - l2 + srcl + 16
}