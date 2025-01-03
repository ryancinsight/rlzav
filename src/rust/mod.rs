pub mod lzav;

use crate::lzav::LzavError;

/// Compress data using the Rust backend and return `i32` for compatibility.
pub fn compress_default(src: &[u8], dst: &mut [u8]) -> i32 {
    match lzav::lzav_compress_default(src, dst) {
        Ok(size) => size as i32,
        Err(e) => e as i32,
    }
}

/// Get the compression bound using the Rust backend and return `i32`.
pub fn compress_bound(srcl: i32) -> i32 {
    lzav::lzav_compress_bound(srcl as usize) as i32
}

/// Decompress data using the Rust backend and return `i32` for compatibility.
pub fn decompress(src: &[u8], dst: &mut [u8]) -> i32 {
    match lzav::lzav_decompress(src, dst) {
        Ok(size) => size as i32,
        Err(e) => e as i32,
    }
}

/// Decompress partial data using the Rust backend and return `i32`.
pub fn decompress_partial(src: &[u8], dst: &mut [u8]) -> i32 {
    lzav::lzav_decompress_partial(src, dst) as i32
}
