use std::env;
use std::fs;
use std::io;
use lzav::{lzav_compress, lzav_compress_bound, lzav_decompress};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <c|d> <input_file> <output_file>", args[0]);
        eprintln!("  c: compress");
        eprintln!("  d: decompress");
        std::process::exit(1);
    }

    let mode = &args[1];
    let input_path = &args[2];
    let output_path = &args[3];

    let input = fs::read(input_path)?;
    
    match mode.as_str() {
        "c" => {
            let bound = lzav_compress_bound(input.len() as i32) as usize;
            let mut compressed = vec![0u8; bound];
            
            match lzav_compress(&input, &mut compressed, None) {
                Ok(compressed_size) => {
                    compressed.truncate(compressed_size);
                    fs::write(output_path, compressed)?;
                    println!("Compressed {} -> {} bytes", input.len(), compressed_size);
                }
                Err(e) => {
                    eprintln!("Compression error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "d" => {
            let mut decompressed = vec![0u8; input.len() * 4];
            let len = decompressed.len();
            
            match lzav_decompress(&input, &mut decompressed, len) {
                Ok(decompressed_size) => {
                    decompressed.truncate(decompressed_size);
                    fs::write(output_path, decompressed)?;
                    println!("Decompressed {} -> {} bytes", input.len(), decompressed_size);
                }
                Err(e) => {
                    eprintln!("Decompression error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Invalid mode. Use 'c' for compress or 'd' for decompress");
            std::process::exit(1);
        }
    }

    Ok(())
}
