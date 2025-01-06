use std::collections::HashMap;
use std::alloc::{Layout, alloc_zeroed, dealloc};

// Ensure cache line alignment
const CACHE_LINE: usize = 64;
const WINDOW_SIZE: usize = 8 * 1024 * 1024;
const MIN_MATCH_LENGTH: usize = 4;
const MAX_MATCH_LENGTH: usize = 258;
const HASH_BITS: u32 = 16;

// Add optimized hash table constants
const HASH_L1_BITS: u32 = 12;  // 4KB hash table fits in L1 cache
const HASH_L2_BITS: u32 = 15;  // 32KB for larger inputs
const HASH_L3_BITS: u32 = 17;  // 128KB for maximum compression

#[derive(Debug, Clone)]  // Add Clone to fix move issues
pub struct CompressedData {
    pub metadata: FileMetadata,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]  // Add Copy to allow dereferencing
pub struct FileMetadata {
    pub original_size: u32,  // Changed from usize to u32 to reduce size
    pub checksum: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(align(8))]  // Proper alignment for SIMD
struct Swar {
    data: u64,
}

impl Swar {
    #[inline(always)]
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut data = 0u64;
        // Unrolled loop for better performance
        if bytes.len() >= 8 {
            data = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        } else {
            for (i, &byte) in bytes.iter().take(8).enumerate() {
                data |= (byte as u64) << (i * 8);
            }
        }
        Self { data }
    }

    #[inline(always)]
    fn find_match_length(&self, other: &Self) -> usize {
        let xor = self.data ^ other.data;
        if xor == 0 {
            return 8;
        }
        
        // Find first differing byte using trailing zeros of byte-wise comparison
        let byte_diff = xor.trailing_zeros() / 8;
        byte_diff as usize
    }

    #[inline(always)]
    fn to_bytes(&self) -> [u8; 8] {
        self.data.to_le_bytes()
    }
}

#[repr(align(64))]  // Cache line alignment without packing
struct HashTable {
    buckets: *mut Vec<usize>,
    size: u32,  // Changed from usize to u32 to reduce size
}

impl HashTable {
    fn new(size: usize) -> Self {
        unsafe {
            let layout = Layout::from_size_align(
                size * std::mem::size_of::<Vec<usize>>(),
                CACHE_LINE
            ).unwrap();
            let ptr = alloc_zeroed(layout) as *mut Vec<usize>;
            // Initialize each bucket
            for i in 0..size {
                ptr.add(i).write(Vec::new());
            }
            Self { buckets: ptr, size: size as u32 }
        }
    }

    #[inline(always)]
    unsafe fn get_bucket(&self, hash: u32) -> &mut Vec<usize> {
        &mut *self.buckets.add(hash as usize % self.size as usize)
    }
}

impl Drop for HashTable {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align(
                self.size as usize * std::mem::size_of::<Vec<usize>>(),
                CACHE_LINE
            ).unwrap();
            dealloc(self.buckets as *mut u8, layout);
        }
    }
}

pub struct SWARCompressor {
    hash_table: HashMap<u32, Vec<usize>>,
}

impl SWARCompressor {
    pub fn new() -> Self {
        Self {
            hash_table: HashMap::with_capacity(1 << HASH_BITS),
        }
    }

    #[inline(always)]
    fn get_hash_bits(&self, input_size: usize) -> u32 {
        // Calculate optimal hash table size based on input size
        if (input_size <= (16 << 10)) {      // <= 16KB
            HASH_L1_BITS
        } else if (input_size <= (128 << 10)) { // <= 128KB
            HASH_L2_BITS
        } else {
            HASH_L3_BITS
        }
    }

    #[inline(always)]
    fn hash(&self, data: &[u8], pos: usize) -> u32 {
        if pos + 4 > data.len() {
            return 0;
        }
        
        // Improved hashing using komihash-style mixing
        let mut h = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        let seed1 = 0x243F6A88 ^ h;
        let mut seed2 = 0x85A308D3;
        
        if pos + 6 <= data.len() {
            seed2 ^= u16::from_le_bytes(data[pos + 4..pos + 6].try_into().unwrap()) as u32;
        }
        
        let hm = (seed1 as u64).wrapping_mul(seed2 as u64);
        let hval = (hm as u32) ^ ((hm >> 32) as u32);
        
        hval & ((1 << self.get_hash_bits(data.len())) - 1)
    }

    #[inline(always)]
    fn compare_bytes_swar(&self, a: &[u8], b: &[u8], len: usize) -> bool {
        let chunks = len / 8;
        let remainder = len % 8;

        for i in 0..chunks {
            let a_swar = Swar::from_bytes(&a[i * 8..]);
            let b_swar = Swar::from_bytes(&b[i * 8..]);
            if a_swar.data != b_swar.data {
                return false;
            }
        }

        if remainder > 0 {
            let start = chunks * 8;
            let a_remainder = &a[start..start + remainder];
            let b_remainder = &b[start..start + remainder];
            a_remainder == b_remainder
        } else {
            true
        }
    }

    #[inline(always)]
    fn find_match(&self, data: &[u8], pos: usize, hash: u32) -> Option<(u32, u16)> {
        let positions = self.hash_table.get(&hash)?;
        
        // Prefetch next hash bucket
        #[cfg(target_arch = "x86_64")]
        if pos + 4 <= data.len() {
            let next_hash = self.hash(data, pos + 1);
            if let Some(next_positions) = self.hash_table.get(&next_hash) {
                unsafe {
                    use std::arch::x86_64::_mm_prefetch;
                    _mm_prefetch::<3>(next_positions.as_ptr() as *const i8);
                }
            }
        }

        let mut best_len = MIN_MATCH_LENGTH - 1;
        let mut best_dist = 0;

        if pos + 8 <= data.len() {
            let current_swar = Swar::from_bytes(&data[pos..]);

            for &prev_pos in positions.iter().rev() {
                let distance = pos - prev_pos;
                if distance >= WINDOW_SIZE {
                    break;
                }

                let prev_swar = Swar::from_bytes(&data[prev_pos..]);
                let match_len = current_swar.find_match_length(&prev_swar);

                if match_len > best_len {
                    best_len = match_len;
                    best_dist = distance as u32;

                    if match_len == 8 {
                        let mut total_len = 8;
                        let mut curr_pos = pos + 8;
                        let mut prev_pos = prev_pos + 8;

                        // Use SWAR for bulk comparison
                        while curr_pos + 8 <= data.len() && 
                              total_len < MAX_MATCH_LENGTH && 
                              self.compare_bytes_swar(
                                  &data[prev_pos..prev_pos + 8],
                                  &data[curr_pos..curr_pos + 8],
                                  8
                              ) {
                            total_len += 8;
                            curr_pos += 8;
                            prev_pos += 8;
                        }

                        // Handle remaining bytes
                        while curr_pos < data.len() && 
                              total_len < MAX_MATCH_LENGTH && 
                              data[prev_pos] == data[curr_pos] {
                            total_len += 1;
                            curr_pos += 1;
                            prev_pos += 1;
                        }

                        return Some((best_dist, total_len as u16));
                    }
                }
            }
        }

        if best_len >= MIN_MATCH_LENGTH {
            Some((best_dist, best_len as u16))
        } else {
            None
        }
    }

    #[inline(always)]
    fn compare_bytes_simd(&self, a: &[u8], b: &[u8], len: usize) -> bool {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::*;
            let chunks = len / 16;
            let remainder = len % 16;

            for i in 0..chunks {
                let a_vec = _mm_loadu_si128(a[i * 16..].as_ptr() as *const __m128i);
                let b_vec = _mm_loadu_si128(b[i * 16..].as_ptr() as *const __m128i);
                if _mm_movemask_epi8(_mm_cmpeq_epi8(a_vec, b_vec)) != 0xFFFF {
                    return false;
                }
            }

            if remainder > 0 {
                let start = chunks * 16;
                a[start..start + remainder] == b[start..start + remainder]
            } else {
                true
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        self.compare_bytes_swar(a, b, len)
    }

    pub fn compress(&mut self, data: &[u8]) -> CompressedData {
        let mut compressed = Vec::with_capacity(data.len());
        let mut pos = 0;
        let mut literals = Vec::new();
        let mut mavg: i64 = 100 << 21; // Running average of match rate
        
        // Setup optimized hash table size
        self.hash_table = HashMap::with_capacity(
            1 << self.get_hash_bits(data.len())
        );

        while pos < data.len() {
            // Early exit for small remaining data
            if pos + MIN_MATCH_LENGTH > data.len() {
                literals.push(data[pos]);
                pos += 1;
                continue;
            }

            let hash = self.hash(data, pos);
            if let Some((distance, length)) = self.find_match(data, pos, hash) {
                // Update match rate average
                mavg += ((length as i64) << 21) - (mavg >> 10);

                // Handle literals if any
                if !literals.is_empty() {
                    self.write_literals(&mut compressed, &literals);
                    literals.clear();
                }

                // Write match reference
                self.write_match(&mut compressed, distance, length);

                // Update hash table entries
                for i in 0..length as usize {
                    if pos + i + 4 <= data.len() {
                        let h = self.hash(data, pos + i);
                        self.hash_table.entry(h)
                            .or_insert_with(Vec::new)
                            .push(pos + i);
                    }
                }
                pos += length as usize;
            } else {
                // Compression speed-up for poor match rates
                if mavg < (200 << 14) {
                    let skip = if mavg < (130 << 14) {
                        if mavg < (100 << 14) {
                            // Very poor matching, skip aggressively
                            3
                        } else {
                            2
                        }
                    } else {
                        1
                    };
                    
                    // Add skipped bytes to literals
                    literals.extend_from_slice(&data[pos..pos + skip.min(data.len() - pos)]);
                    pos += skip;
                } else {
                    literals.push(data[pos]);
                    self.hash_table.entry(hash)
                        .or_insert_with(Vec::new)
                        .push(pos);
                    pos += 1;
                }
            }
        }

        // Handle remaining literals
        if !literals.is_empty() {
            self.write_literals(&mut compressed, &literals);
        }

        CompressedData {
            metadata: FileMetadata {
                original_size: data.len() as u32,
                checksum: self.calculate_checksum(data),
            },
            data: compressed,
        }
    }

    #[inline(always)]
    fn write_literals(&self, compressed: &mut Vec<u8>, literals: &[u8]) {
        compressed.push(0);
        compressed.extend_from_slice(&(literals.len() as u16).to_le_bytes());
        compressed.extend_from_slice(literals);
    }

    #[inline(always)]
    fn write_match(&self, compressed: &mut Vec<u8>, distance: u32, length: u16) {
        compressed.push(1);
        compressed.extend_from_slice(&distance.to_le_bytes());
        compressed.extend_from_slice(&length.to_le_bytes());
    }

    pub fn decompress(&self, compressed: &CompressedData) -> Vec<u8> {
        let mut result = Vec::with_capacity(compressed.metadata.original_size as usize);
        let mut pos = 0;
        let data = &compressed.data; // Local copy to avoid packed field access

        while pos < data.len() {
            match data[pos] {
                0 => {
                    let len = u16::from_le_bytes(
                        data[pos + 1..pos + 3].try_into().unwrap()
                    ) as usize;
                    result.extend_from_slice(&data[pos + 3..pos + 3 + len]);
                    pos += 3 + len;
                }
                1 => {
                    let distance = u32::from_le_bytes(
                        data[pos + 1..pos + 5].try_into().unwrap()
                    ) as usize;
                    let length = u16::from_le_bytes(
                        data[pos + 5..pos + 7].try_into().unwrap()
                    ) as usize;
                    
                    let start = result.len() - distance;
                    // Handle backreferences byte by byte to ensure correct order
                    for i in 0..length {
                        result.push(result[start + i]);
                    }
                    pos += 7;
                }
                _ => panic!("Invalid token type"),
            }
        }

        assert_eq!(result.len() as u32, compressed.metadata.original_size);
        
        // Calculate checksum on the fully decompressed data
        let decompressed_checksum = self.calculate_checksum(&result);
        assert_eq!(decompressed_checksum, compressed.metadata.checksum);
        
        result
    }

    pub fn decompress_size(&self, data: &[u8]) -> usize {
        let mut pos = 0;
        let mut total_size = 0;
        
        while pos < data.len() {
            match data[pos] {
                0 => {
                    if pos + 3 > data.len() { break; }
                    let len = u16::from_le_bytes(data[pos + 1..pos + 3].try_into().unwrap()) as usize;
                    total_size += len;
                    pos += 3 + len;
                }
                1 => {
                    if pos + 7 > data.len() { break; }
                    let length = u16::from_le_bytes(data[pos + 5..pos + 7].try_into().unwrap()) as usize;
                    total_size += length;
                    pos += 7;
                }
                _ => break,
            }
        }
        total_size
    }

    pub fn calculate_initial_checksum(&self, data: &[u8]) -> u32 {
        let mut result = Vec::new();
        let mut pos = 0;

        // Decompress data to calculate checksum
        while pos < data.len() {
            match data[pos] {
                0 => {
                    if pos + 3 > data.len() { break; }
                    let len = u16::from_le_bytes(data[pos + 1..pos + 3].try_into().unwrap()) as usize;
                    if pos + 3 + len > data.len() { break; }
                    result.extend_from_slice(&data[pos + 3..pos + 3 + len]);
                    pos += 3 + len;
                }
                1 => {
                    if pos + 7 > data.len() { break; }
                    let distance = u32::from_le_bytes(data[pos + 1..pos + 5].try_into().unwrap()) as usize;
                    let length = u16::from_le_bytes(data[pos + 5..pos + 7].try_into().unwrap()) as usize;
                    
                    let start = result.len() - distance;
                    for i in 0..length {
                        result.push(result[start + i]);
                    }
                    pos += 7;
                }
                _ => break,
            }
        }

        self.calculate_checksum(&result)
    }

    #[inline(always)]
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        let mut checksum = 0u32;
        for &byte in data {
            checksum = checksum.wrapping_add(byte as u32);
            checksum = checksum.rotate_left(1);
        }
        checksum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let data = b"";
        let mut compressor = SWARCompressor::new();
        let compressed = compressor.compress(data);
        let decompressed = compressor.decompress(&compressed);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_decompression() {
        let data = b"This is a test string with some repetition. \
                    This is a test string with some repetition. \
                    This is a test string with some repetition. \
                    This is a test string with some repetition.";
        
        let mut compressor = SWARCompressor::new();
        let compressed = compressor.compress(data);
        let compressed_len = compressed.data.len(); // Store length locally
        
        println!("Original size: {}, Compressed size: {}", data.len(), compressed_len);
        assert!(compressed_len < data.len());
        
        let decompressed = compressor.decompress(&compressed);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_large_repetitive_input() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let mut compressor = SWARCompressor::new();
        let compressed = compressor.compress(&data);
        
        println!("Original size: {}, Compressed size: {}", data.len(), compressed.data.len());
        assert!(compressed.data.len() < data.len());
        
        let decompressed = compressor.decompress(&compressed);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_highly_repetitive() {
        let mut data = Vec::with_capacity(1000);
        for _ in 0..100 {
            data.extend_from_slice(b"HelloWorld");
        }
        
        let mut compressor = SWARCompressor::new();
        let compressed = compressor.compress(&data);
        
        println!("Highly repetitive - Original size: {}, Compressed size: {}", 
                data.len(), compressed.data.len());
        assert!(compressed.data.len() < data.len());
        
        let decompressed = compressor.decompress(&compressed);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_swar_operations() {
        let a = Swar::from_bytes(b"AAAAAAAA");
        let b = Swar::from_bytes(b"AAAAAAAB");
        
        assert_eq!(a.find_match_length(&b), 7);
        
        let c = Swar::from_bytes(b"AAAAAAAA");
        assert_eq!(a.find_match_length(&c), 8);
    }
}
