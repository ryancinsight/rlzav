#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// Shared modules between implementations
pub mod errors;

// Implementation-specific modules
#[cfg(feature = "c-backend")]
pub mod c;
#[cfg(feature = "rust-backend")]
pub mod rust;

// Re-export the active implementation
#[cfg(feature = "c-backend")]
pub use crate::c::*;

#[cfg(feature = "rust-backend")]
pub use crate::rust::*;

// Re-export compression functions
#[cfg(feature = "rust-backend")]
pub use crate::rust::{
    compress_default,
    compress_bound,
};

#[cfg(feature = "c-backend")]
pub use crate::c::compress_bound;

// Re-export decompression function
#[cfg(feature = "rust-backend")]
pub use crate::rust::decompress;

#[cfg(feature = "c-backend")]
pub use crate::c::decompress;

// Common interface that both implementations must provide
pub trait Lzav {
    fn compress_default(src: &[u8], dst: &mut [u8]) -> i32;
    fn compress_bound(srcl: i32) -> i32;
    fn decompress(src: &[u8], dst: &mut [u8]) -> i32;
    fn decompress_partial(src: &[u8], dst: &mut [u8]) -> i32;
}

#[cfg(feature = "c-backend")]
pub fn init() {
    // Initialize C backend error codes
    use crate::errors::*;
    unsafe {
        LZAV_ERR_CODES[0].set(c::c_get_lzav_e_params()).unwrap();
        LZAV_ERR_CODES[1].set(c::c_get_lzav_e_srcoob()).unwrap();
        LZAV_ERR_CODES[2].set(c::c_get_lzav_e_dstoob()).unwrap();
        LZAV_ERR_CODES[3].set(c::c_get_lzav_e_refoob()).unwrap();
        LZAV_ERR_CODES[4].set(c::c_get_lzav_e_dstlen()).unwrap();
        LZAV_ERR_CODES[5].set(c::c_get_lzav_e_unkfmt()).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    fn verify_roundtrip(original: &[u8]) {

        let mut compressed = vec![0u8; compress_bound(original.len() as i32) as usize];

        let mut decompressed = vec![0u8; original.len()];

        let compressed_len = compress_default(original, &mut compressed);
        assert!(compressed_len > 0, "Compression failed");

        println!("\nCompression Results:");
        println!("Original size: {} bytes", original.len());
        println!("Compressed size: {} bytes", compressed_len);
        println!("Compression ratio: {:.2}%", 
            (compressed_len as f64 / original.len() as f64) * 100.0);

        let decompressed_len = decompress(&compressed[..compressed_len as usize], &mut decompressed);

        {
            // Cast `i32` to `usize` for slice indexing
            let decompressed_length = decompressed_len as usize;
            assert_eq!(decompressed_length, original.len(),
                "Decompressed length does not match original length");
            assert_eq!(&decompressed[..decompressed_length], original, "Decompressed data does not match original");
        }

        // Print first few bytes for verification
        println!("\nData sample (first 16 bytes or less):");
        println!("Original:    {:02x?}", &original[..original.len().min(16)]);
        println!("Decompressed:{:02x?}", &decompressed[..decompressed.len().min(16)]);
        
        if original.len() > 16 {
            println!("...");
        }
    }

    #[test]
    fn test_compress_decompress() {
        let original = b"Hello, World!";
        verify_roundtrip(original);
    }

    #[test]
    fn test_compress_bound() {
        {
            assert!(compress_bound(100) > 0);
            assert_eq!(compress_bound(0), 16);
            assert!(compress_bound(1) > 0);
        }
    }

    #[test]
    fn test_roundtrip_random_data() {
        let mut rng = rand::thread_rng();
        let size = 1024;
        let original: Vec<u8> = (0..size).map(|_| rng.gen()).collect();
        verify_roundtrip(&original);
    }

    #[test]
    fn test_html_compression() {
        let data = br#"<!DOCTYPE html>
<html>
<head>
    <title>Test Page</title>
</head>
<body>
    <div class="container">
        <h1>Welcome</h1>
        <p>This is a test page with repeating content.</p>
        <div class="container">
            <h1>Welcome</h1>
            <p>This is a test page with repeating content.</p>
        </div>
    </div>
</body>
</html>"#;
        verify_roundtrip(data);
    }

    #[test]
    fn test_json_compression() {
        let data = br#"{"key":"value","array":[1,2,3],"nested":{"hello":"world","array":[1,2,3,4,5]},"repeated":{"hello":"world","array":[1,2,3,4,5]}}"#;
        verify_roundtrip(data);
    }

    #[test]
    fn test_markdown_compression() {
        let data = br#"# Title

Some text with **bold** formatting.

## Section 1
- List item 1
- List item 2
- List item 3

## Section 2
Same content repeated:
- List item 1
- List item 2
- List item 3
"#;
        verify_roundtrip(data);
    }
}
