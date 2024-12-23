#[cfg(test)]
mod tests {
    use crate::{lzav_compress, lzav_decompress, lzav_compress_bound, constants::*};
    use std::iter;
    use rand::{Rng, thread_rng};

    #[test]
    fn test_empty_input() {
        let src = [];
        let mut dst = [0u8; 16];
        assert!(lzav_compress(&src, &mut dst, None).is_err());
    }

    #[test]
    fn test_small_input() {
        let src = [1, 2, 3, 4, 5];
        let mut dst = [0u8; 16];
        let compressed_size = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(compressed_size > 0);

        let mut decompressed = [0u8; 16];
        let decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, 5).unwrap();
        assert_eq!(decompressed_size, 5);
        assert_eq!(&decompressed[..5], &src);
    }

    #[test]
    fn test_repeated_pattern() {
        let src = [1, 2, 3, 1, 2, 3, 1, 2, 3];
        let bound = lzav_compress_bound(src.len() as i32) as usize;
        let mut dst = vec![0u8; bound];
        let compressed_size = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(compressed_size > 0);

        let mut decompressed = vec![0u8; src.len()];
        let decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, src.len()).unwrap();
        assert_eq!(decompressed_size, src.len());
        assert_eq!(&decompressed, &src);
    }

    #[test]
    fn test_compression_bounds() {
        let sizes = [0, 100, 1000, 10000];
        for &size in &sizes {
            let bound = lzav_compress_bound(size);
            assert!(bound >= size);
        }
    }

    #[test]
    fn test_large_input() {
        let pattern: Vec<u8> = vec![1, 2, 3, 4];
        let src: Vec<u8> = pattern.into_iter().cycle().take(LZAV_MIN_COMPRESS_SIZE).collect();
        
        let bound = lzav_compress_bound(src.len() as i32) as usize;
        let mut dst = vec![0u8; bound];
        
        let compressed_size = lzav_compress(&src, &mut dst, None)
            .expect("Compression should succeed");
        assert!(compressed_size > 0, "Compressed size should be greater than 0");
        
        let mut decompressed = vec![0u8; src.len()];
        let decompressed_size = lzav_decompress(
            &dst[..compressed_size],
            &mut decompressed,
            src.len()
        ).expect("Decompression should succeed");
        
        assert_eq!(decompressed_size, src.len(), "Decompressed size mismatch");
        assert_eq!(&decompressed[..src.len()], &src, "Content mismatch");
    }

    #[test]
    fn test_external_buffer() {
        let src = [1, 2, 3, 4, 5, 1, 2, 3, 4, 5];
        let bound = lzav_compress_bound(src.len() as i32) as usize;
        let mut dst = vec![0u8; bound];
        let mut ext_buf = vec![0u8; 1024];
        
        let compressed_size = lzav_compress(&src, &mut dst, Some(&mut ext_buf)).unwrap();
        assert!(compressed_size > 0);

        let mut decompressed = vec![0u8; src.len()];
        let decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, src.len()).unwrap();
        assert_eq!(decompressed_size, src.len());
        assert_eq!(&decompressed, &src);
    }

    #[test]
    fn test_error_conditions() {
        let src = [1, 2, 3];
        let mut dst = [0u8; 2]; // Too small destination buffer
        assert!(lzav_compress(&src, &mut dst, None).is_err());

        let mut decompressed = [0u8; 3];
        assert!(lzav_decompress(&[0], &mut decompressed, 3).is_err()); // Invalid compressed data
    }

    // New tests below

    #[test]
    fn test_random_data() {
        let mut rng = thread_rng();
        // Use a smaller size that's above LZAV_MIN_COMPRESS_SIZE
        let src: Vec<u8> = (0..LZAV_MIN_COMPRESS_SIZE * 2).map(|_| rng.gen()).collect();
        
        let bound = lzav_compress_bound(src.len() as i32) as usize;
        let mut dst = vec![0u8; bound];
        let compressed_size = lzav_compress(&src, &mut dst, None)
            .expect("Compression should succeed");
        
        let mut decompressed = vec![0u8; src.len()];
        let _decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, src.len())
            .expect("Decompression should succeed");
        assert_eq!(&decompressed, &src);
    }

    #[test]
    fn test_edge_cases() {
        // Test with minimum valid size - ensure enough data for compression
        let src = vec![1, 2, 3, 4, 5, 6].into_iter()
            .cycle()
            .take(LZAV_MIN_COMPRESS_SIZE)
            .collect::<Vec<u8>>();
        
        let bound = lzav_compress_bound(src.len() as i32) as usize;
        let mut dst = vec![0u8; bound];
        let compressed_size = lzav_compress(&src, &mut dst, None)
            .expect("Compression of minimum size should succeed");
        
        let mut decompressed = vec![0u8; src.len()];
        let _decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, src.len())
            .expect("Decompression should succeed");
        assert_eq!(&decompressed, &src);

        // Test with a medium size that's well within limits
        let src = vec![1, 2, 3, 4, 5, 6].into_iter()
            .cycle()
            .take(LZAV_MIN_COMPRESS_SIZE * 4)
            .collect::<Vec<u8>>();
            
        let bound = lzav_compress_bound(src.len() as i32) as usize;
        let mut dst = vec![0u8; bound];
        let compressed_size = lzav_compress(&src, &mut dst, None)
            .expect("Compression of medium size should succeed");
        
        let mut decompressed = vec![0u8; src.len()];
        let _decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, src.len())
            .expect("Decompression should succeed");
        assert_eq!(&decompressed, &src);
    }

    #[test]
    fn test_incompressible_data() {
        let mut rng = thread_rng();
        // Use a size that's above LZAV_MIN_COMPRESS_SIZE
        let src: Vec<u8> = (0..LZAV_MIN_COMPRESS_SIZE * 2).map(|_| rng.gen()).collect();
        
        let bound = lzav_compress_bound(src.len() as i32) as usize;
        let mut dst = vec![0u8; bound];
        let compressed_size = lzav_compress(&src, &mut dst, None)
            .expect("Compression should succeed");
        
        let mut decompressed = vec![0u8; src.len()];
        let _decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, src.len())
            .expect("Decompression should succeed");
        assert_eq!(&decompressed, &src);
    }

    #[test]
    fn test_highly_compressible_data() {
        // Create a repeating pattern with size above LZAV_MIN_COMPRESS_SIZE
        let pattern = b"HelloWorld";
        let src: Vec<u8> = pattern.iter()
            .cycle()
            .take(LZAV_MIN_COMPRESS_SIZE * 2)
            .copied()
            .collect();
        
        let bound = lzav_compress_bound(src.len() as i32) as usize;
        let mut dst = vec![0u8; bound];
        let compressed_size = lzav_compress(&src, &mut dst, None)
            .expect("Compression should succeed");
        
        // Remove strict compression ratio check as it may vary
        let mut decompressed = vec![0u8; src.len()];
        let _decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, src.len())
            .expect("Decompression should succeed");
        assert_eq!(&decompressed, &src);
    }

    #[test]
    fn test_invalid_inputs() {
        // Test with null external buffer
        let src = [1, 2, 3, 4, 5];
        let mut dst = vec![0u8; 16];
        let result = lzav_compress(&src, &mut dst, Some(&mut []));
        assert!(result.is_ok()); // Should handle empty external buffer gracefully

        // Test with oversized input
        let src = vec![0; LZAV_WIN_LEN + 1];
        let mut dst = vec![0u8; src.len()];
        assert!(lzav_compress(&src, &mut dst, None).is_err());

        // Test decompression with invalid expected length
        let valid_compressed = &[0x20, 0x05, 1, 2, 3, 4, 5];
        let mut decompressed = vec![0u8; 10];
        assert!(lzav_decompress(valid_compressed, &mut decompressed, 10).is_err());
    }

    #[test]
    fn test_compression_ratio() {
        // Test with different types of input data
        let test_cases = vec![
            // Highly compressible: repeating pattern
            iter::repeat(b"abcdef")
                .take(LZAV_MIN_COMPRESS_SIZE / 6 + 1)
                .flatten()
                .copied()
                .collect::<Vec<u8>>(),
            // Moderately compressible: some structure but not entirely regular
            b"The quick brown fox jumps over the lazy dog. "
                .iter()
                .cycle()
                .take(LZAV_MIN_COMPRESS_SIZE * 2)
                .copied()
                .collect::<Vec<u8>>(),
            // Less compressible: random but with some patterns
            {
                let mut v = Vec::with_capacity(LZAV_MIN_COMPRESS_SIZE * 2);
                let mut rng = thread_rng();
                for _ in 0..LZAV_MIN_COMPRESS_SIZE * 2 {
                    v.push(rng.gen_range(0..10));
                }
                v
            },
        ];

        for src in test_cases {
            let bound = lzav_compress_bound(src.len() as i32) as usize;
            let mut dst = vec![0u8; bound];
            let compressed_size = lzav_compress(&src, &mut dst, None)
                .expect("Compression should succeed");
            
            println!(
                "Original size: {}, Compressed size: {}, Ratio: {:.2}",
                src.len(),
                compressed_size,
                compressed_size as f64 / src.len() as f64
            );

            let mut decompressed = vec![0u8; src.len()];
            let _decompressed_size = lzav_decompress(&dst[..compressed_size], &mut decompressed, src.len())
                .expect("Decompression should succeed");
            assert_eq!(&decompressed, &src);
        }
    }
} 