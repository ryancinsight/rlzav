mod lzav;
use lzav::*;

/// Compress data using the Rust SWAR-based backend and return `i32` for compatibility.
pub fn compress_default(src: &[u8], dst: &mut [u8]) -> i32 {
    let mut compressor = SWARCompressor::new();
    
    if src.is_empty() || dst.is_empty() {
        return -1; // LZAV_E_PARAMS
    }
    
    if dst.len() < compress_bound(src.len() as i32) as usize {
        return -4; // LZAV_E_DSTLEN
    }

    let compressed = compressor.compress(src);
    if compressed.data.len() > dst.len() {
        return -4; // LZAV_E_DSTLEN
    }

    dst[..compressed.data.len()].copy_from_slice(&compressed.data);
    compressed.data.len() as i32
}

/// Get the compression bound for SWAR-based compression and return `i32`.
pub fn compress_bound(srcl: i32) -> i32 {
    if srcl <= 0 {
        return 16;
    }
    // Add overhead for compression metadata and worst-case scenario
    (srcl as usize + (srcl as usize / 8) + 16) as i32
}

/// Decompress data using the SWAR-based backend and return `i32` for compatibility.
pub fn decompress(src: &[u8], dst: &mut [u8]) -> i32 {
    let compressor = SWARCompressor::new();
    
    if src.is_empty() || dst.is_empty() {
        return -1; // LZAV_E_PARAMS
    }

    // First decompress to get actual size
    let size_check = compressor.decompress_size(src);
    if size_check > dst.len() {
        return -4; // LZAV_E_DSTLEN
    }

    // Create CompressedData structure from input with correct size
    let compressed = CompressedData {
        metadata: FileMetadata {
            original_size: size_check as u32,
            checksum: compressor.calculate_initial_checksum(src),
        },
        data: src.to_vec(),
    };

    match compressor.decompress(&compressed) {
        decompressed if decompressed.len() <= dst.len() => {
            dst[..decompressed.len()].copy_from_slice(&decompressed);
            decompressed.len() as i32
        },
        _ => -4, // LZAV_E_DSTLEN
    }
}

/// Decompress partial data using the SWAR-based backend and return `i32`.
pub fn decompress_partial(src: &[u8], dst: &mut [u8]) -> i32 {
    // For now, partial decompression is same as full decompression
    decompress(src, dst)
}
