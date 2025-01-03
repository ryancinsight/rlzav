use std::env;
use std::fs;
use std::path::Path;
use std::io::{self, BufReader, BufWriter, Read, Write, Seek, SeekFrom};

// Constants for safety limits and buffer sizes
const MAX_PATH_LENGTH: u32 = 1024;
const BUFFER_SIZE: usize = 1024 * 1024; // 1MB chunks
const MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024; // 1GB limit

use rlzav::compress_default;
use rlzav::decompress;
use rlzav::errors::{LZAV_E_PARAMS, LZAV_E_SRCOOB, LZAV_E_DSTOOB, LZAV_E_REFOOB, LZAV_E_DSTLEN, LZAV_E_UNKFMT};

fn print_help() {
    println!("RLZAV Compression Utility");
    println!("\nUSAGE:");
    println!("  rlzav <command> [options]");
    println!("\nCOMMANDS:");
    println!("  help                     Show this help message");
    println!("  compress <input> <out>   Compress a file or folder");
    println!("  decompress <in> <out>    Decompress an archive");
    println!("\nEXAMPLES:");
    println!("  # Compress a single file:");
    println!("  rlzav compress myfile.txt archive.lzav");
    println!("\n  # Compress a folder:");
    println!("  rlzav compress myfolder archive.lzav");
    println!("\n  # Decompress to a folder:");
    println!("  rlzav decompress archive.lzav output_folder");
    println!("\n  # Decompress to a single file:");
    println!("  rlzav decompress archive.lzav output.txt");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "help" | "-h" | "--help" => {
            print_help();
        },
        "compress" => {
            if args.len() != 4 {
                eprintln!("Usage: rlzav compress <file/folder> <output_file>");
                eprintln!("Try 'rlzav help' for more information");
                std::process::exit(1);
            }
            let input_folder = &args[2];
            let output_file = &args[3];

            if let Err(e) = compress_folder(input_folder, output_file) {
                eprintln!("Compression failed: {}", e);
                std::process::exit(1);
            }
        },
        "decompress" => {
            if args.len() != 4 {
                eprintln!("Usage: rlzav decompress <archive_file> <output>");
                eprintln!("Try 'rlzav help' for more information");
                std::process::exit(1);
            }
            let archive_file = &args[2];
            let output_folder = &args[3];

            if let Err(e) = decompress_archive(archive_file, output_folder) {
                eprintln!("Decompression failed: {}", e);
                std::process::exit(1);
            }
        },
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Try 'rlzav help' for more information");
            std::process::exit(1);
        }
    }
}

fn compress_folder(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(input);
    let file = fs::File::create(output)?;
    let mut archive = BufWriter::new(file);

    if path.is_file() {
        let metadata = fs::metadata(path)?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err("File too large".into());
        }
        let file_name = path.file_name()
            .ok_or("Invalid file name")?
            .to_string_lossy()
            .into_owned();
        compress_single_file(&mut archive, path, &file_name)?;
    } else {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let metadata = fs::metadata(&path)?;
                if metadata.len() > MAX_FILE_SIZE {
                    eprintln!("Skipping large file: {}", path.display());
                    continue;
                }
                let relative_path = path.file_name()
                    .ok_or("Invalid file name")?
                    .to_string_lossy()
                    .into_owned();
                compress_single_file(&mut archive, &path, &relative_path)?;
            }
        }
    }
    archive.flush()?;
    Ok(())
}

fn compress_single_file(archive: &mut BufWriter<fs::File>, path: &Path, store_path: &str) 
    -> Result<(), Box<dyn std::error::Error>> 
{
    let path_bytes = store_path.as_bytes();
    if path_bytes.len() > MAX_PATH_LENGTH as usize {
        return Err("Path too long".into());
    }

    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let file_size = reader.get_ref().metadata()?.len();
    
    let path_len = path_bytes.len() as u32;
    archive.write_all(&path_len.to_le_bytes())?;
    archive.write_all(path_bytes)?;
    archive.write_all(&(file_size as u32).to_le_bytes())?;

    // Stream compression in chunks
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut compressed_size = 0u32;
    let compressed_size_pos = archive.seek(SeekFrom::Current(0))?;
    archive.write_all(&[0u8; 4])?; // Placeholder for compressed size

    eprintln!("Compressing file: {}", path.display());
    
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 { break; }
        
        let chunk = &buffer[..bytes_read];
        let mut compressed = vec![0u8; rlzav::compress_bound(bytes_read as i32) as usize];
        let compressed_len = rlzav::compress_default(chunk, &mut compressed);
        compressed.truncate(compressed_len as usize);
        
        compressed_size += compressed_len as u32;
        archive.write_all(&compressed)?;
    }

    // Go back and write the actual compressed size
    let current_pos = archive.seek(SeekFrom::Current(0))?;
    archive.seek(SeekFrom::Start(compressed_size_pos))?;
    archive.write_all(&compressed_size.to_le_bytes())?;
    archive.seek(SeekFrom::Start(current_pos))?;

    eprintln!("Saved compressed file: {} ({} bytes -> {} bytes)", 
             path.display(), file_size, compressed_size);
    Ok(())
}

fn decompress_archive(archive: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Starting decompression of archive: {}", archive);
    let file = fs::File::open(archive)?;
    let metadata = file.metadata()?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err("Archive too large".into());
    }

    let mut reader = BufReader::new(file);
    let output_path = Path::new(output);
    let is_dir = output_path.extension().is_none() || 
                 output_path.to_str().map_or(false, |s| s.ends_with('/'));

    while reader.seek(SeekFrom::Current(0))? < metadata.len() {
        let mut path_len_bytes = [0u8; 4];
        reader.read_exact(&mut path_len_bytes)?;
        let path_len = u32::from_le_bytes(path_len_bytes);

        if path_len > MAX_PATH_LENGTH {
            return Err("Invalid path length in archive".into());
        }

        let mut path_bytes = vec![0u8; path_len as usize];
        reader.read_exact(&mut path_bytes)?;
        let path = String::from_utf8(path_bytes)?;

        let mut original_len_bytes = [0u8; 4];
        reader.read_exact(&mut original_len_bytes)?;
        let original_len = u32::from_le_bytes(original_len_bytes);

        let mut compressed_len_bytes = [0u8; 4];
        reader.read_exact(&mut compressed_len_bytes)?;
        let compressed_len = u32::from_le_bytes(compressed_len_bytes);

        if compressed_len > MAX_FILE_SIZE as u32 || original_len > MAX_FILE_SIZE as u32 {
            return Err("File in archive too large".into());
        }

        eprintln!("Extracting: {} ({} bytes compressed)", path, compressed_len);

        let final_path = if is_dir {
            output_path.join(&path)
        } else {
            output_path.to_path_buf()
        };

        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut compressed = vec![0u8; compressed_len as usize];
        reader.read_exact(&mut compressed)?;

        let mut decompressed = vec![0u8; original_len as usize];
        let result = rlzav::decompress(&compressed, &mut decompressed);

        if result < 0 {
            return Err(format!("Decompression failed for {}: {}", path, result).into());
        }

        let mut output_file = BufWriter::new(fs::File::create(&final_path)?);
        output_file.write_all(&decompressed[..result as usize])?;
        output_file.flush()?;

        eprintln!("Extracted: {} ({} bytes)", final_path.display(), result);
    }

    eprintln!("Decompression completed successfully.");
    Ok(())
}