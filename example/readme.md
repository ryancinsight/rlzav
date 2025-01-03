# LZAV

A Rust implementation of the LZAV compression algorithm, providing fast and efficient data compression with SIMD optimizations.

[![Crates.io](https://img.shields.io/crates/v/lzav.svg)](https://crates.io/crates/lzav)
[![Documentation](https://docs.rs/lzav/badge.svg)](https://docs.rs/lzav)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- Fast compression and decompression
- SIMD optimizations for x86_64 architectures
- Zero-copy operations
- Streaming support
- Safe Rust implementation
- Configurable compression levels
- Optional external buffer support

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
lzav = "0.1.0"
```

## Usage

Basic compression:

```rust
use lzav::{lzav_compress, lzav_decompress};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"Hello, World!";
    let mut compressed = vec![0u8; data.len() * 2];
    let mut decompressed = vec![0u8; data.len()];

    // Compress
    let compressed_size = lzav_compress(data, &mut compressed, None)?;
    compressed.truncate(compressed_size);

    // Decompress
    let decompressed_size = lzav_decompress(
        &compressed, 
        &mut decompressed, 
        data.len()
    )?;

    assert_eq!(&data[..], &decompressed[..decompressed_size]);
    Ok(())
}
```

With external buffer (for better performance):

```rust
use lzav::lzav_compress;

let data = b"Hello, World!";
let mut compressed = vec![0u8; data.len() * 2];
let mut ext_buf = vec![0u8; 1024];

let compressed_size = lzav_compress(
    data, 
    &mut compressed, 
    Some(&mut ext_buf)
)?;
```

## Performance

The LZAV algorithm is designed for:

- Fast compression and decompression
- Good compression ratios for typical data
- Efficient memory usage
- SIMD optimization where available

## API Documentation

### Main Functions

- `lzav_compress`: Compresses data using the LZAV algorithm
- `lzav_decompress`: Decompresses LZAV-compressed data
- `lzav_decompress_partial`: Performs partial decompression for streaming

### Options

- External buffer support for improved performance
- Configurable compression parameters
- Format version selection

## Building

```bash
cargo build --release
```

With SIMD optimizations (recommended for x86_64):

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Testing

```bash
cargo test
```

## Features

- `format1`: Enable support for format version 1 (optional)
- Default features include SIMD optimizations for supported platforms

## Requirements

- Rust 1.51 or later
- No external dependencies
- Optional: x86_64 CPU with AVX2 support for SIMD optimizations

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

Based on the original LZAV compression algorithm.