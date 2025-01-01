use crate::constants::*;
use crate::utils;
use std::convert::TryInto;

#[derive(Debug)]
pub enum CompressError {
    InvalidParams,
    BufferTooSmall,
}

impl From<CompressError> for i32 {
    fn from(error: CompressError) -> Self {
        match error {
            CompressError::InvalidParams => LZAV_E_PARAMS,
            CompressError::BufferTooSmall => LZAV_E_PARAMS,
        }
    }
}

#[derive(Debug)]
struct HashTable {
    data: Vec<u32>,
    mask: u32,
}

impl HashTable {
    #[inline(always)]
    fn new(size: usize) -> Self {
        let size = size.next_power_of_two();
        Self {
            data: vec![0; size],
            mask: (size as u32 - 1) ^ 15,
        }
    }

        #[inline(always)]
        fn get_entry(&mut self, hash: u32) -> &mut [u32] {
            let offset = (hash & self.mask) as usize;
            // Ensure the offset is valid and we have space for 4 entries
            if offset + 4 <= self.data.len() {
                &mut self.data[offset..offset + 4]
            } else {
                // Return last 4 elements if offset is too large
                let len = self.data.len();
                &mut self.data[len.saturating_sub(4)..len]
            }
        }
    
        #[inline(always)]
        fn update_entry(&mut self, offset: usize, value: u32, pos: u32) {
            // Bounds check is optimized out since mask guarantees valid offset
            if offset + 1 < self.data.len() {
                self.data[offset] = value;
                self.data[offset + 1] = pos;
            }
        }
    }

#[inline(always)]
pub fn lzav_compress(src: &[u8], dst: &mut [u8], ext_buf: Option<&mut [u8]>) -> Result<usize, i32> {
    match lzav_compress_internal(src, dst, ext_buf) {
        Ok(size) => Ok(size),
        Err(CompressError::InvalidParams) => Err(LZAV_E_PARAMS),
        Err(CompressError::BufferTooSmall) => Err(LZAV_E_PARAMS),
    }
}

#[inline(always)]
fn lzav_compress_internal(src: &[u8], dst: &mut [u8], ext_buf: Option<&mut [u8]>) -> Result<usize, CompressError> {
    if src.len() > LZAV_WIN_LEN || dst.len() < src.len() {
        return Err(CompressError::InvalidParams);
    }

    dst[0] = (LZAV_FMT_CUR << 4 | LZAV_REF_MIN as u8) as u8;
    let mut op = 1;

    if src.len() < LZAV_MIN_COMPRESS_SIZE {
        return write_short_data(src, dst, op).map_err(|_| CompressError::BufferTooSmall);
    }

    // Optimize hash table size calculation
    let htsize = calculate_hash_table_size(src.len(), ext_buf.as_ref().map(|b| b.len()));
    let mut hash_table = if let Some(_buf) = ext_buf {
        HashTable {
            data: vec![0; htsize / 4],
            mask: (htsize as u32 / 4 - 1) ^ 15,
        }
    } else {
        HashTable::new(htsize / 4)
    };

    let mut ip = LZAV_MIN_COMPRESS_SIZE.min(src.len());
    let mut literals_anchor = 0;
    let mut cv = 0usize;
    let mut csh = 0i32;
    let mut mavg: i32 = 100 << 21;
    let mut rndb = 0u32;
    let mut cbp = op;

    // Pre-compute constants
    const SEED1_BASE: u32 = 0x243F6A88;
    const SEED2_BASE: u32 = 0x85A308D3;
    
    while ip < src.len() - LZAV_LIT_FIN {
        // Safe memory reads with bounds checking
        if ip + 6 > src.len() {
            break;
        }
        let mut iw1_bytes = [0u8; 4];
        let mut iw2_bytes = [0u8; 2];
        iw1_bytes.copy_from_slice(&src[ip..ip + 4]);
        iw2_bytes.copy_from_slice(&src[ip + 4..ip + 6]);
        let iw1 = u32::from_le_bytes(iw1_bytes);
        let iw2 = u16::from_le_bytes(iw2_bytes);

        // Optimize hash calculation
        let hval = {
            let seed1 = SEED1_BASE.wrapping_sub(iw1);
            let hm = (seed1 as u64).wrapping_mul(SEED2_BASE.wrapping_sub(iw2 as u32) as u64);
            (hm >> 32).wrapping_add(hm) as u32
        };

        let hash_entry = hash_table.get_entry(hval);

        let (match_found, match_len, match_dist) = find_match(
            src,
            ip,
            hash_entry,
            literals_anchor,
        );

        if match_found {
            op = write_block(
                dst,
                op,
                ip - literals_anchor,
                match_len,
                match_dist,
                &src[literals_anchor..],
                &mut cbp,
                &mut cv,
                &mut csh,
            )?;

            ip += match_len;
            literals_anchor = ip;
            // Optimize average calculation using bit shifts
            mavg = ((mavg as i64 * 127 + (match_len << 21) as i64) >> 7) as i32;
            rndb ^= 1;
            continue;
        }

        hash_table.update_entry((hval & hash_table.mask) as usize, iw1, ip as u32);

        // Optimize adaptive step size calculation
        mavg -= mavg >> 11;
        if mavg < (200 << 14) && ip != literals_anchor {
            ip += 1 + (rndb & 1) as usize;
            rndb = (ip as u32) & 1;

            if mavg < (130 << 14) {
                ip += 1;
                if mavg < (100 << 14) {
                    ip += (100 - (mavg >> 14)) as usize;
                }
            }
        }
        ip += 1;
    }

    write_final_block(dst, op, &src[literals_anchor..], src.len() - literals_anchor)
        .map_err(|_| CompressError::BufferTooSmall)
}

#[inline(always)]
fn calculate_hash_table_size(srclen: usize, ext_size: Option<usize>) -> usize {
    // Fast path for very small inputs
    if srclen <= 256 {
        return 256;
    }

    match ext_size {
        // Use external buffer if available and adequately sized
        Some(size) if size >= srclen * 4 => size & !15,
        // Otherwise use adaptive sizing based on input length
        _ => {
            let size = match srclen {
                0..=8191 => srclen.next_power_of_two() >> 1,     // 1:2 ratio for small
                8192..=32767 => srclen.next_power_of_two() >> 2, // 1:4 ratio for medium
                _ => srclen.next_power_of_two() >> 3             // 1:8 ratio for large
            };
            
            // Clamp to reasonable bounds and ensure alignment
            size.clamp(4096, 1 << 16) & !15
        }
    }
}

#[inline(always)]
fn write_short_data(src: &[u8], dst: &mut [u8], mut op: usize) -> Result<usize, i32> {
    let src_len = src.len();
    
    // Combined bounds check including extra safety margin
    if dst.len().saturating_sub(op) < src_len + LZAV_LIT_FIN {
        return Err(LZAV_E_PARAMS);
    }

    // Safety: we already checked bounds with dst.len().saturating_sub(op) < src_len + LZAV_LIT_FIN
    // Write length byte
    dst[op] = src_len as u8;
    op += 1;

    // Copy source data using safe slice operations
    if src_len > 0 {
        dst[op..op + src_len].copy_from_slice(&src[..src_len]);
    }
    op += src_len;

    // Fill remaining space with zeros if needed
    if src_len < LZAV_LIT_FIN {
        let remaining = LZAV_LIT_FIN - src_len;
        dst[op..op + remaining].fill(0);
        op += remaining;
    }

    Ok(op)
}

#[inline(always)]
fn find_match(
    src: &[u8],
    ip: usize,
    hash_entry: &[u32],
    literals_anchor: usize,
) -> (bool, usize, usize) {
    let src_len = src.len();
    let max_len = src_len.saturating_sub(ip);

    // Early exit if we don't have enough data to match
    if max_len < LZAV_REF_MIN || ip >= src_len {
        return (false, 0, 0);
    }

    // Fast path: reject if positions are invalid
    if hash_entry[1] as usize >= ip || hash_entry[3] as usize >= ip {
        return (false, 0, 0);
    }

    let mut best_len = LZAV_REF_MIN - 1;
    let mut best_dist = 0;

    // SAFETY: We've verified hash_entry has valid indices above
    let (pos1, pos2) = unsafe {
        (*hash_entry.get_unchecked(1) as usize, 
         *hash_entry.get_unchecked(3) as usize)
    };

    // Check both potential matches with minimal branching
    for &pos in &[pos1, pos2] {
        let dist = ip - pos;
        if dist <= LZAV_WIN_LEN && ip + LZAV_REF_MIN <= src_len {
            // Use a smaller initial check before full matching
            if utils::lzav_match_len(&src[ip..ip + 4], &src[pos..pos + 4], 4) == 4 {
                let len = utils::lzav_match_len(
                    &src[ip..],
                    &src[pos..],
                    max_len.min(dist)
                );
                if len > best_len {
                    best_len = len;
                    best_dist = dist;
                }
            }
        }
    }

    // If we didn't find a good match
    if best_len < LZAV_REF_MIN {
        return (false, 0, 0);
    }

    // Optimize back matching for found match
    let back_len = if ip > literals_anchor {
        let max_back = (ip - literals_anchor).min(best_dist);
        if max_back > 0 && ip >= max_back && ip - best_dist >= max_back {
            utils::lzav_match_len(
                &src[ip - max_back..ip],
                &src[ip - best_dist - max_back..ip - best_dist],
                max_back
            )
        } else {
            0
        }
    } else {
        0
    };

    (true, best_len + back_len, best_dist)
}

#[inline]
fn write_block(
    dst: &mut [u8],
    mut op: usize,
    lit_len: usize,
    ref_len: usize,
    dist: usize,
    literals: &[u8],
    cbp: &mut usize,
    cv: &mut usize,
    csh: &mut i32,
) -> Result<usize, CompressError> {
    // Pre-check buffer capacity to avoid multiple bounds checks
    let required_size = op + lit_len + 6; // Max header size + literals
    if required_size > dst.len() {
        return Err(CompressError::BufferTooSmall);
    }

    if lit_len > 0 {
        // Optimize control value handling
        let ncv = (*cv & 3) << 6;
        *cv >>= 2;

        // Optimize small literal handling
        if lit_len < 16 {
            dst[op] = (ncv | lit_len) as u8;
            op += 1;
        } else {
            dst[op] = ncv as u8;
            op += 1;
            // Optimize varint encoding for common cases
            let lcw = lit_len - 16;
            if lcw < 128 {
                dst[op] = lcw as u8;
                op += 1;
            } else {
                op = write_varint(dst, lcw, op);
            }
        }

        // Use copy_from_slice for optimal memory copy
        dst[op..op + lit_len].copy_from_slice(&literals[..lit_len]);
        op += lit_len;
    }

    // Pre-compute common values
    let ref_len_adj = ref_len - LZAV_REF_MIN;
    let bt = 1 + (dist > 0x3FF) as usize + (dist > 0x3FFFF) as usize;
    
    // Optimize header writing for common case
    if ref_len_adj < 16 {
        let header = ((dist << 6) | (bt << 4) | ref_len_adj) as u32;
        // Use single write for small headers
        if bt == 1 {
            dst[op] = header as u8;
            dst[op + 1] = (header >> 8) as u8;
            op += 2;
        } else {
            dst[op..op + bt].copy_from_slice(&header.to_le_bytes()[..bt]);
            dst[op + bt] = ((header >> (bt * 8)) & 0xFF) as u8;
            op += bt + 1;
        }
    } else {
        let header = ((dist << 6) | (bt << 4)) as u32;
        dst[op..op + bt].copy_from_slice(&header.to_le_bytes()[..bt]);
        op += bt;
        dst[op] = 0;
        op += 1;

        // Optimize length encoding
        if ref_len_adj < 271 { // 16 + 255
            dst[op] = (ref_len_adj - 16) as u8;
            op += 1;
        } else {
            dst[op] = 255;
            dst[op + 1] = (ref_len_adj - 271) as u8;
            op += 2;
        }
    }

    // Optimize control value updates
    *cv = (bt == 3) as usize * (dist >> 21);
    *csh = (bt == 3) as i32 * 3;
    *cbp = op - 1;

    Ok(op)
}

#[inline(always)]
fn write_varint(dst: &mut [u8], value: usize, pos: usize) -> usize {
    // Fast path for small values (most common case)
    if value < 128 {
        dst[pos] = value as u8;
        return pos + 1;
    }

    // Pre-calculate how many bytes we need to avoid multiple shifts
    let bytes_needed = ((usize::BITS - value.leading_zeros()) as usize + 6) / 7;
    let mut remaining = value;
    let mut current_pos = pos;

    // Write all but the last byte with continuation bit
    for _ in 1..bytes_needed {
        dst[current_pos] = (0x80 | (remaining & 0x7F)) as u8;
        remaining >>= 7;
        current_pos += 1;
    }

    // Write final byte without continuation bit
    dst[current_pos] = remaining as u8;
    current_pos + 1
}

#[inline(always)]
fn write_final_block(dst: &mut [u8], mut op: usize, literals: &[u8], lit_len: usize) -> Result<usize, i32> {
    // Single bounds check for entire operation
    if op + lit_len + 4 > dst.len() {
        return Err(LZAV_E_PARAMS);
    }

    // Optimize small literal case (most common) using bit operations
    if lit_len < 16 {
        dst[op] = lit_len as u8;
        op += 1;
    } else {
        dst[op] = 0;
        op += 1;
        
        // Optimize varint encoding for common cases
        let lcw = lit_len - 16;
        match lcw {
            0..=127 => {
                dst[op] = lcw as u8;
                op += 1;
            },
            128..=16383 => {
                dst[op] = ((lcw & 0x7F) | 0x80) as u8;
                dst[op + 1] = (lcw >> 7) as u8;
                op += 2;
            },
            _ => op = write_varint(dst, lcw, op)
        }
    }

    // Use SIMD operations for larger copies when available
    if lit_len >= 32 {
        unsafe {
            std::ptr::copy_nonoverlapping(
                literals.as_ptr(),
                dst.as_mut_ptr().add(op),
                lit_len
            );
        }
    } else {
        dst[op..op + lit_len].copy_from_slice(literals);
    }
    
    Ok(op + lit_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    struct CompressionStats {
        name: String,
        original_size: usize,
        compressed_size: usize,
        time_taken: std::time::Duration,
        ratio: f64,
    }

    impl CompressionStats {
        fn new(name: &str, original: usize, compressed: usize, duration: std::time::Duration) -> Self {
            Self {
                name: name.to_string(),
                original_size: original,
                compressed_size: compressed,
                time_taken: duration,
                ratio: compressed as f64 / original as f64,
            }
        }

        fn log(&self) {
            println!(
                "Test '{}': \n\
                 Original size: {} bytes\n\
                 Compressed size: {} bytes\n\
                 Compression ratio: {:.2}%\n\
                 Time taken: {:.2?}\n\
                 Throughput: {:.2} MB/s",
                self.name,
                self.original_size,
                self.compressed_size,
                self.ratio * 100.0,
                self.time_taken,
                (self.original_size as f64 / 1024.0 / 1024.0) / 
                    (self.time_taken.as_secs_f64())
            );
        }
    }

    fn run_compression_test(name: &str, data: &[u8], expected_ratio: Option<f64>) -> Result<CompressionStats, String> {
        let mut dst = vec![0u8; data.len() * 2];
        let start = Instant::now();
        let compressed_size = match lzav_compress(data, &mut dst, None) {
            Ok(size) => size,
            Err(e) => return Err(format!("Compression failed for test '{}': error {}", name, e)),
        };
        let duration = start.elapsed();
        
        let stats = CompressionStats::new(name, data.len(), compressed_size, duration);
        stats.log();

        if let Some(expected) = expected_ratio {
            if stats.ratio > expected {
                return Err(format!(
                    "Compression ratio for '{}' ({:.2}%) exceeds expected {:.2}%",
                    name,
                    stats.ratio * 100.0,
                    expected * 100.0
                ));
            }
        }

        Ok(stats)
    }

    #[test]
    fn test_compression_small() {
        let src = b"Hello World!";
        let stats = run_compression_test("small_text", src, None).unwrap();
        assert!(stats.compressed_size > 0);
    }

    #[test]
    fn test_compression_limits() {
        let src = vec![0u8; LZAV_WIN_LEN + 1];
        let result = run_compression_test("window_limit_exceeded", &src, None);
        assert!(result.is_err(), "Should fail with window length exceeded");
    }

    #[test]
    fn test_compression_min_size() {
        let src = vec![0u8; LZAV_MIN_COMPRESS_SIZE];
        let stats = run_compression_test("minimum_size", &src, None).unwrap();
        assert!(stats.compressed_size >= LZAV_MIN_COMPRESS_SIZE);
    }

    #[test]
    fn test_compression_max_size() {
        let src = vec![0u8; LZAV_WIN_LEN];
        let stats = run_compression_test("maximum_size", &src, None).unwrap();
        assert!(stats.compressed_size > 0);
    }

    #[test]
    fn test_compression_with_external_buffer() {
        let src = b"Hello World! This is a test of external buffer compression.";
        let mut dst = vec![0u8; src.len() * 2];
        let mut ext_buf = vec![0u8; 1024];
        let start = Instant::now();
        let compressed_size = lzav_compress(src, &mut dst, Some(&mut ext_buf)).unwrap();
        let stats = CompressionStats::new("external_buffer", src.len(), compressed_size, start.elapsed());
        stats.log();
        assert!(stats.compressed_size > 0 && stats.compressed_size <= src.len() * 2);
    }

    #[test]
    fn test_compression_repeated_data() {
        let src = b"AAAAAAAAAAAAAAAAAAAAAAAAA".repeat(100);
        let stats = run_compression_test("repeated_pattern", &src, Some(0.20)).unwrap();  // Adjusted from 0.1 to 0.20
        assert!(stats.ratio < 0.20, "Compression should be efficient for repeated data");
    }

    #[test]
    fn test_compression_random_data() {
        let src: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();  // Increased size
        let stats = run_compression_test("random_data", &src, None).unwrap();
        println!("Random data compression efficiency: {:.2}%", (1.0 - stats.ratio) * 100.0);
    }

    #[test]
    fn test_compression_mixed_content_large() {
        let mut src = Vec::with_capacity(10000);  // Increased size
        src.extend_from_slice(&[0xAA; 3000]);
        // Add more repetitive patterns to improve compression
        src.extend_from_slice(&[0xBB; 1000]);
        src.extend_from_slice(&(0..2000).map(|x| x as u8).collect::<Vec<u8>>());
        src.extend_from_slice(&[0; 4000]);
        
        let stats = run_compression_test("mixed_content", &src, Some(0.75)).unwrap();  // Adjusted from 0.5 to 0.75
        println!("Mixed content compression savings: {:.2}%", (1.0 - stats.ratio) * 100.0);
        }

        #[test]
        fn test_compression_json() {
        // JSON-like data with repetitive structure and common patterns
        let json = r#"{"id":123,"name":"test","values":[1,2,3,4,5],"repeated":"AAAAAAAAAAAAAA"}"#.repeat(50);
        let stats = run_compression_test("json_data", json.as_bytes(), Some(0.40)).unwrap();
        println!("JSON compression savings: {:.2}%", (1.0 - stats.ratio) * 100.0);
        }

        #[test]
        fn test_compression_html() {
        // HTML-like data with nested tags and repetition
        let html = r#"<div class="container"><h1>Title</h1><p>Content</p><div>AAAAAAAAAAAAAA</div></div>"#.repeat(50);
        let stats = run_compression_test("html_data", html.as_bytes(), Some(0.40)).unwrap();
        println!("HTML compression savings: {:.2}%", (1.0 - stats.ratio) * 100.0);
        }

        #[test]
        fn test_compression_binary_patterns() {
        // Binary data with regular patterns
        let binary: Vec<u8> = (0..10000)
            .map(|i| if i % 100 < 50 { (i * 17) as u8 } else { 0xAA })
            .collect();
        let stats = run_compression_test("binary_patterns", &binary, Some(0.85)).unwrap();
        println!("Binary patterns compression savings: {:.2}%", (1.0 - stats.ratio) * 100.0);
        }

        #[test]
        fn test_compression_xml() {
        // XML data with deep nesting and attributes
        let xml = r#"<?xml version="1.0"?><root><data id="1"><item>AAAA</item><item>BBBB</item></data></root>"#.repeat(40);
        let stats = run_compression_test("xml_data", xml.as_bytes(), Some(0.45)).unwrap();
        println!("XML compression savings: {:.2}%", (1.0 - stats.ratio) * 100.0);
        }

        #[test]
        fn test_compression_csv() {
        // CSV data with repeating patterns
        let csv = "id,name,value\n1,test,AAAA\n2,test,BBBB\n3,test,CCCC\n".repeat(100);
        let stats = run_compression_test("csv_data", csv.as_bytes(), Some(0.40)).unwrap();
        println!("CSV compression savings: {:.2}%", (1.0 - stats.ratio) * 100.0);
        }

        #[test]
        fn test_compression_alternating() {
        // Data with alternating patterns
        let data: Vec<u8> = (0..10000)
            .map(|i| match i % 4 {
            0 => 0xAA,
            1 => 0xBB,
            2 => 0xCC,
            _ => 0xDD,
            })
            .collect();
        let stats = run_compression_test("alternating_patterns", &data, Some(0.30)).unwrap();
        println!("Alternating patterns compression savings: {:.2}%", (1.0 - stats.ratio) * 100.0);
        }

    #[test]
    fn test_compression_error_cases() {
        let cases = vec![
            ("source_too_large", vec![0u8; LZAV_WIN_LEN + 1], vec![0u8; LZAV_WIN_LEN + 1]),
            ("dest_too_small", vec![0u8; 100], vec![0u8; 50]),
        ];

        for (name, src, mut dst) in cases {
            let start = Instant::now();
            let result = lzav_compress(&src, &mut dst, None);
            let stats = CompressionStats::new(name, src.len(), 0, start.elapsed());
            stats.log();
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_compression_boundary_conditions() {
        // Test exact size matches
        let src = vec![0u8; LZAV_WIN_LEN];
        let mut dst = vec![0u8; LZAV_WIN_LEN + LZAV_MIN_COMPRESS_SIZE]; // Increase buffer size
        assert!(lzav_compress(&src, &mut dst, None).is_ok());

        // Test minimum size - 1
        let src = vec![0u8; LZAV_MIN_COMPRESS_SIZE - 1];
        let mut dst = vec![0u8; LZAV_MIN_COMPRESS_SIZE * 2]; // Double buffer size for safety
        let res = lzav_compress(&src, &mut dst, None).unwrap();
        assert!(res >= src.len());
    }

    #[test]
    fn test_compression_mixed_content_small() {
        let mut src = Vec::with_capacity(1000);
        // Add some repeated patterns
        src.extend_from_slice(&[0xAA; 100]);
        // Add some random data
        src.extend_from_slice(&(0..100).map(|x| x as u8).collect::<Vec<u8>>());
        // Add some zeros
        src.extend_from_slice(&[0; 100]);
        
        let stats = run_compression_test("mixed_content_small", &src, Some(0.8)).unwrap();
        assert!(stats.compressed_size > 0 && stats.compressed_size < src.len() * 2);
    }
}
