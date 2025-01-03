/**
 * @file lzav.h
 *
 * @version 4.5
 *
 * @brief The inclusion file for the "LZAV" in-memory data compression and
 * decompression algorithms.
 *
 * Description is available at https://github.com/avaneev/lzav
 *
 * E-mail: aleksey.vaneev@gmail.com or info@voxengo.com
 *
 * LICENSE:
 *
 * Copyright (c) 2023-2024 Aleksey Vaneev
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 */
// Consolidate version and format definitions
// Unified error codes
pub enum LZAV_ERROR {
    LZAV_E_PARAMS = -1,
    LZAV_E_SRCOOB = -2,
    LZAV_E_DSTOOB = -3,
    LZAV_E_REFOOB = -4,
    LZAV_E_DSTLEN = -5,
    LZAV_E_UNKFMT = -6,
}
// Consolidated compression constants
// Optimized memory operations
/*
 * macro to let the compiler assume
 * the given pointer is aligned. This macro will fall back to the pointer
 * unchanged if the compiler doesn't support __builtin_assume_aligned.
 */
// Unified block handling macros
// Add small data optimization constants
// Maximum size for tiny data optimization
// Maximum size for small data optimization
// Minimum hash table size
// Add memory pool for small allocations
// Add branch prediction hints and cache optimizations
/**
 * @def LZAV_LITTLE_ENDIAN
 * @brief Endianness definition macro, can be used as a logical constant.
 */
// defined( __BIG_ENDIAN__ )
// defined( __BIG_ENDIAN__ )
/**
 * @def LZAV_ARCH64
 * @brief Macro that denotes availability of 64-bit instructions.
 */
// 64-bit availability check
/**
 * @def LZAV_GCC_BUILTINS
 * @brief Macro that denotes availability of GCC-style built-in functions.
 */
// GCC built-ins check
/**
 * @def LZAV_IEC16( x )
 * @brief In-place endianness-correction macro, for singular 16-bit variables.
 * @param x Value to correct in-place.
 */
/**
 * @def LZAV_IEC32( x )
 * @brief In-place endianness-correction macro, for singular 32-bit variables.
 * @param x Value to correct in-place.
 */
/**
 * @def LZAV_IEC64( x )
 * @brief In-place endianness-correction macro, for singular 64-bit variables.
 * @param x Value to correct in-place.
 */
// LZAV_LITTLE_ENDIAN
// defined( _MSC_VER )
// defined( _MSC_VER )
// LZAV_LITTLE_ENDIAN
/**
 * @def LZAV_LIKELY( x )
 * @brief Likelihood macro that is used for manually-guided
 * micro-optimization.
 * @param x Expression that is likely to be evaluated to 1.
 */
/**
 * @def LZAV_UNLIKELY( x )
 * @brief Unlikelihood macro that is used for manually-guided
 * micro-optimization.
 * @param x Expression that is unlikely to be evaluated to 1.
 */
// Likelihood macros
// Likelihood macros
// For _BitScanForwardX and _byteswap_X.
// defined( _MSC_VER ) && !defined( LZAV_GCC_BUILTINS )
// Optimize hash table size for better cache utilization
pub static LZAV_HASH_L1_BITS: c_uint =
     // 4KB hash table fits well in L1 cache
    // 32KB for larger inputs
    // 128KB for maximum compression
    // Power of 2 sizing ensures fast modulo via bitwise AND
    // 4KB
    // 32KB
    // 128KB
    // Masks for fast modulo operations
    // Faster modulus and bitwise operations
    // Use leading zero count to determine optimal bits
    // This replaces branches with direct bit manipulation
    // Map size ranges to bit values using modulo math
    // 12 bits (4KB) for small, 15 bits (32KB) for medium, 17 bits (128KB) for large
    12;
pub static LZAV_HASH_L3_BITS: c_long =
     // Clamp to valid range using min/max
    17;
unsafe fn lzav_get_hash_bits(mut input_size: usize) -> usize {
    let leading_zeros =
        std::mem::size_of::<usize>() * 8 - 1 -
            __builtin_clzll(input_size as u64);
    let base_bits =
        LZAV_HASH_L1_BITS +
            if leading_zeros > 13 {
                ((leading_zeros - 13) * 3) >> 1
            } else { 0 };
    return if base_bits > LZAV_HASH_L3_BITS {
               LZAV_HASH_L3_BITS
           } else if base_bits < LZAV_HASH_L1_BITS {
               LZAV_HASH_L1_BITS
           } else { base_bits };
}
pub static LZAV_HASH_L1_MASK: c_uint =

    // Optimized hash table size selection
    // Use bit shifts to determine optimal size
    // <= 16KB
    (1 << 12) - 1;
pub static LZAV_HASH_L2_MASK: c_uint =
     // <= 128KB
    (1 << 15) - 1;
pub static LZAV_HASH_L3_MASK: c_uint = (1 << 17) - 1;
unsafe fn lzav_get_hash_mask(mut input_size: usize) -> u32 {
    if input_size <= (16 << 10) { return LZAV_HASH_L1_MASK; }
    if input_size <= (128 << 10) { return LZAV_HASH_L2_MASK; }
    return LZAV_HASH_L3_MASK;
}
// Fast hash function using the selected mask
unsafe fn lzav_hash(mut x: u32, mut mask: u32) -> u32 {
    x *= 506832829; // Multiplicative hash
    x ^=
        x >> LZAV_HASH_L1_BITS; // Second multiplication improves distribution
    x *= 506832829; // Fast modulo via bitwise AND
    return x & mask;
}
// Add memory pool for small allocations
pub struct lzav_pool_t {
    pub blocks: [*mut u8; 64],
    pub used: [usize; 64],
    pub count: usize,
}
pub static LZAV_POOL_BLOCK: c_long =
     // Round up size to alignment boundary
    // Try to allocate from existing blocks
    2048;
pub static LZAV_POOL_SIZE: c_long =
     // Need new block
    64;
unsafe fn lzav_pool_alloc(mut pool: &mut lzav_pool_t, mut size: usize)
 -> *mut c_void {
    size = (size + 7) & !7; // Pool exhausted, fallback to direct malloc
    for mut i in 0..pool.count {
        let mut remaining = LZAV_POOL_BLOCK - pool.used[i];
        if remaining >= size {
            let mut ptr = (pool.blocks[i]).offset(pool.used[i]);
            pool.used[i] += size;
            return ptr;
        };
    }
    if pool.count < LZAV_POOL_SIZE {
        let mut block = malloc(LZAV_POOL_BLOCK) as *mut u8;
        if block.is_null() { return ptr::null_mut(); }
        pool.blocks[pool.count] = block;
        pool.used[pool.count] = size;
        pool.count += 1;
        return block as *mut _;
    }
    return malloc(size);
}
pub static LZAV_PREFETCH_DIST: c_int =

    // Add SIMD hints for modern compilers
    // Early exit for small lengths
    // Assume alignment of p1, p2 to 16 bytes for faster SIMD loads
    // Prefetch distant cache lines (unchanged)
    512;
unsafe fn lzav_match_len_opt(mut p1: *const u8, mut p2: *const u8, ml: usize)
 -> usize {
    if __builtin_expect((ml < 8), 0) != 0 {
        let mut len = 0; // Align to 8-byte boundary for better memory access
        while len < ml && p1[len] == p2[len] {
            len += 1; // Process 64 bytes at a time with AVX-512
        } // Process 32 bytes at a time with AVX2
        return len; // Process 16 bytes at a time with SSE2
    } // Process remaining bytes with 64-bit operations
    let mut p1_aligned =
        __builtin_assume_aligned((p1) as *const _, 16) as
            *const u8; // Handle remaining bytes (less than 8)
    let mut p2_aligned =
        __builtin_assume_aligned((p2) as *const _, 16) as *const u8;
    __builtin_prefetch(p1_aligned.offset(LZAV_PREFETCH_DIST));
    __builtin_prefetch(p2_aligned.offset(LZAV_PREFETCH_DIST));
    let p1s = p1;
    let p1e = p1.offset(ml);
    let mut misalign = (p1 as usize) & 7;
    if misalign != 0 {
        misalign = 8 - misalign;
        while { let mut _t = misalign; misalign -= 1; _t } != 0 && p1 < p1e &&
                  *p1 == *p2 {
            p1 = p1.offset(1);
            p2 = p2.offset(1);
        }
        if p1 < p1e && *p1 != *p2 { return p1.offset(-p1s); };
    }
    while __builtin_expect((p1.offset(7) < p1e), 1) != 0 {
        let mut v1: u64;
        let mut v2: u64;
        memcpy(&mut v1, p1 as *const _, 8);
        memcpy(&mut v2, p2 as *const _, 8);
        if v1 != v2 {
            return p1.offset(-p1s) + (__builtin_ctzll(v1 ^ v2) >> 3);
        }
        p1 = p1.offset(8);
        p2 = p2.offset(8);
    }
    while p1 < p1e && *p1 == *p2 { p1 = p1.offset(1); p2 = p2.offset(1); }
    return p1.offset(-p1s);
}
/**
 * @brief Data match length finding function.
 *
 * Function finds the number of continuously-matching leading bytes between
 * two buffers. This function is well-optimized for a wide variety of
 * compilers and platforms.
 *
 * @param p1 Pointer to buffer 1.
 * @param p2 Pointer to buffer 2.
 * @param ml Maximal number of bytes to match.
 * @return The number of matching leading bytes.
 */
unsafe fn lzav_match_len(mut p1: *const u8, mut p2: *const u8, ml: usize)
 -> usize {
    let p1s = p1; // LZAV_LITTLE_ENDIAN
    let p1e = p1.offset(ml); // LZAV_LITTLE_ENDIAN
    while __builtin_expect((p1.offset(7) < p1e), 1) != 0 {
        let mut v1: u64; // defined( LZAV_GCC_BUILTINS )
        let mut v2: u64; // defined( _MSC_VER )
        let mut vd: u64; // defined( _MSC_VER )
        memcpy(&mut v1, p1 as *const _, 8); // defined( LZAV_GCC_BUILTINS )
        memcpy(&mut v2, p2 as *const _, 8);
        vd = v1 ^ v2;
        if vd != 0 { return (p1.offset(-p1s) + (__builtin_ctzll(vd) >> 3)); }
        p1 = p1.offset(8);
        p2 = p2.offset(8);
    }
    // At most 7 bytes left.
    if __builtin_expect((p1.offset(3) < p1e), 1) != 0
       { // defined( LZAV_ARCH64 )
        // defined( LZAV_ARCH64 )
        let mut v1: u32; // LZAV_LITTLE_ENDIAN
        let mut v2: u32; // LZAV_LITTLE_ENDIAN
        memcpy(&mut v1, p1 as *const _, 4); // defined( LZAV_GCC_BUILTINS )
        memcpy(&mut v2, p2 as *const _, 4); // defined( _MSC_VER )
        let mut vd = v1 ^ v2; // defined( _MSC_VER )
        if vd != 0 {
            return (p1.offset(-p1s) +
                        (__builtin_ctz(vd) >>
                             3)); // defined( LZAV_GCC_BUILTINS )
        }
        p1 = p1.offset(4);
        p2 = p2.offset(4);
    }
    // At most 3 bytes left.
    if p1 < p1e {
        if *p1 != p2[0] { return p1.offset(-p1s); }
        if { p1 = p1.offset(1); p1 } < p1e {
            if *p1 != p2[1] { return p1.offset(-p1s); }
            if { p1 = p1.offset(1); p1 } < p1e {
                if *p1 != p2[2] { return p1.offset(-p1s); };
            };
        };
    }
    return (ml);
}
/**
 * @brief Data match length finding function, reverse direction.
 *
 * @param p1 Origin pointer to buffer 1.
 * @param p2 Origin pointer to buffer 2.
 * @param ml Maximal number of bytes to back-match.
 * @return The number of matching prior bytes, not including origin position.
 */
unsafe fn lzav_match_len_r(mut p1: *const u8, mut p2: *const u8, ml: usize)
 -> usize {
    if __builtin_expect((ml == 0), 0) != 0 {
        return 0; // LZAV_LITTLE_ENDIAN
    } // LZAV_LITTLE_ENDIAN
    if p1[-1] != p2[-1] { return 0; }
    if __builtin_expect((ml != 1), 0) != 0 {
        let p1s = p1;
        let mut p1e = p1.offset(-ml).offset(1);
        p1 = p1.offset(-1);
        p2 = p2.offset(-1);
        while __builtin_expect((p1 > p1e), 0) != 0 {
            let mut v1: u16;
            let mut v2: u16;
            memcpy(&mut v1, p1.offset(-2), 2);
            memcpy(&mut v2, p2.offset(-2), 2);
            let vd = v1 ^ v2;
            if vd != 0 { return (p1s.offset(-p1) + ((vd & 65280) == 0)); }
            p1 = p1.offset(-2);
            p2 = p2.offset(-2);
        }
        p1e = p1e.offset(-1);
        if p1 > p1e && p1[-1] != p2[-1] { return p1s.offset(-p1); };
    }
    return (ml);
}
// Add an optimized reverse match length function using SIMD
unsafe fn lzav_match_len_opt_r(mut p1: *const u8, mut p2: *const u8,
                               ml: usize)
 -> usize { // Early exit for small lengths
    if __builtin_expect((ml < 8), 0) != 0 {
        let mut len =
            0; // Assume alignment of p1, p2 to 16 bytes for faster SIMD loads
        while len < ml && p1[-len - 1] == p2[-len - 1] {
            len += 1; // Prefetch distant cache lines
        } // Align to 8-byte boundary for better memory access
        return len; // Process 64 bytes at a time with AVX-512
    } // Find the first differing byte
    let mut p1_aligned =
        __builtin_assume_aligned(p1.offset(-ml), 16) as
            *const u8; // Process 32 bytes at a time with AVX2
    let mut p2_aligned =
        __builtin_assume_aligned(p2.offset(-ml), 16) as
            *const u8; // Process 16 bytes at a time with SSE2
    __builtin_prefetch(p1_aligned.offset(-LZAV_PREFETCH_DIST)); // Find the first differing byte
    __builtin_prefetch(p2_aligned.offset(-LZAV_PREFETCH_DIST)); // Process remaining bytes with 64-bit operations
    let p1s = p1; // Handle remaining bytes (less than 8)
    let p1e = p1.offset(-ml);
    let mut misalign = (p1.offset(-1) as usize) & 7;
    if misalign != 0 {
        misalign = 8 - misalign;
        while { let mut _t = misalign; misalign -= 1; _t } != 0 && p1 > p1e &&
                  p1[-1] == p2[-1] {
            p1 = p1.offset(-1);
            p2 = p2.offset(-1);
        }
        if p1 > p1e && p1[-1] != p2[-1] { return p1s.offset(-p1); };
    }
    while __builtin_expect((p1.offset(-7) >= p1e), 1) != 0 {
        let mut v1: u64;
        let mut v2: u64;
        memcpy(&mut v1, p1.offset(-8), 8);
        memcpy(&mut v2, p2.offset(-8), 8);
        if v1 != v2 {
            return p1s.offset(-p1.offset(-(__builtin_ctzll(v1 ^ v2) >>
                                               3)).offset(8));
        }
        p1 = p1.offset(-8);
        p2 = p2.offset(-8);
    }
    while p1 > p1e && p1[-1] == p2[-1] {
        p1 = p1.offset(-1);
        p2 = p2.offset(-1);
    }
    return p1s.offset(-p1);
}
/**
 * @brief Internal LZAV block header writing function (stream format 2).
 *
 * Internal function writes a block to the output buffer. This function can be
 * used in custom compression algorithms.
 *
 * Stream format 2.
 *
 * "Raw" compressed stream consists of any quantity of unnumerated "blocks".
 * A block starts with a header byte, followed by several optional bytes.
 * Bits 4-5 of the header specify block's type.
 *
 * CC00LLLL: literal block (1-6 bytes). `LLLL` is literal length.
 * OO01RRRR: 10-bit offset block (2-4 bytes). `RRRR` is reference length.
 * OO10RRRR: 18-bit offset block (3-5 bytes).
 * OO11RRRR: 23-bit offset block (4-6 bytes).
 *
 * If `LLLL` or `RRRR` equals 0, a value of 16 is assumed, and an additional
 * length byte follows. If in a literal block this additional byte's highest
 * bit is 1, one more length byte follows that defines higher bits of length
 * (up to 4 bytes). In a reference block, additional 1-2 length bytes follow
 * the offset bytes. `CC` is a reference offset carry value (additional 2
 * lowest bits of offset of the next reference block). Block type 3 includes 3
 * carry bits (highest bits of 4th byte).
 *
 * The overall compressed data is prefixed with a byte whose lower 4 bits
 * contain minimal reference length (mref), and the highest 4 bits contain
 * stream format identifier. Compressed data always finishes with
 * `LZAV_LIT_FIN` literals. The lzav_write_fin_2() function should be used to
 * finalize compression.
 *
 * Except the last block, a literal block is always followed by a reference
 * block.
 *
 * @param op Output buffer pointer.
 * @param lc Literal length, in bytes.
 * @param rc Reference length, in bytes, not lesser than mref.
 * @param d Reference offset, in bytes. Should be lesser than `LZAV_WIN_LEN`,
 * and not lesser than `rc` since fast copy on decompression cannot provide
 * consistency of copying of data that is not in the output yet.
 * @param ipa Literals anchor pointer.
 * @param cbpp Pointer to the pointer to the latest offset carry block header.
 * Cannot be 0, but the contained pointer can be 0 (initial value).
 * @param cshp Pointer to offset carry shift.
 * @param mref Minimal reference length, in bytes, used by the compression
 * algorithm.
 * @return Incremented output buffer pointer.
 */
unsafe fn lzav_write_blk_2(mut op: *mut u8, mut lc: usize, mut rc: usize,
                           mut d: usize, mut ipa: *const u8,
                           cbpp: &mut *mut u8, cshp: &mut c_int, mref: usize)
 -> *mut u8 {
    // Perform offset carry to a previous block (`csh` may be zero).
    let csh = *cshp;
    rc = rc + 1 - mref;
    **cbpp |= ((d << 8) >> csh) as u8;
    d >>= csh;
    if __builtin_expect((lc != 0), 0) != 0 {
        // Write a literal block.
        // Offset carry value in literal block.
        let mut cv = (d & 3) << 6; // LZAV_LITTLE_ENDIAN
        d >>= 2; // LZAV_LITTLE_ENDIAN
        if __builtin_expect((lc < 9), 1) != 0 {
            *op = (cv | lc) as u8;
            op = op.offset(1);
            memcpy(op as *mut _, ipa as *const _, 8);
            op = op.offset(lc);
        } else if __builtin_expect((lc < 16), 1) != 0 {
            *op = (cv | lc) as u8;
            op = op.offset(1);
            memcpy(op as *mut _, ipa as *const _, 16);
            op = op.offset(lc);
        } else if __builtin_expect((lc < 16 + 128), 1) != 0 {
            let mut ov = ((lc - 16) << 8 | cv) as u16;
            memcpy(op as *mut _, &mut ov, 2);
            op = op.offset(2);
            memcpy(op as *mut _, ipa as *const _, 16);
            memcpy(op.offset(16), ipa.offset(16), 16);
            if lc < 33 {
                op = op.offset(lc);
            } else {
                ipa = ipa.offset(32);
                op = op.offset(32);
                lc -= 32;
                loop  {
                    *op = *ipa;
                    ipa = ipa.offset(1);
                    op = op.offset(1);
                    if { lc -= 1; lc } == 0 { break  };
                };
            };
        } else {
            *op = cv as u8;
            op = op.offset(1);
            let mut lcw = lc - 16;
            while lcw > 127 {
                *op = (128 | lcw) as u8;
                lcw >>= 7;
                op = op.offset(1);
            }
            *op = lcw as u8;
            op = op.offset(1);
            memcpy(op as *mut _, ipa as *const _, lc);
            op = op.offset(lc);
        };
    }
    // Write a reference block.
    let mut ocsh = [0, 0, 0, 3];
    let bt = 1 + (d > (1 << 10) - 1) + (d > (1 << 18) - 1);
    if __builtin_expect((rc < 16), 1) != 0 {
        let mut ov = (d << 6 | bt << 4 | rc) as u32;
        memcpy(op as *mut _, &mut ov, 4);
        op = op.offset(bt);
        *cshp = ocsh[bt];
        *cbpp = op;
        return op.offset(1);
    }
    let mut ov = (d << 6 | bt << 4) as u32;
    memcpy(op as *mut _, &mut ov, 4);
    op = op.offset(bt);
    *cshp = ocsh[bt];
    *cbpp = op;
    if __builtin_expect((rc < 16 + 255), 1) != 0 {
        op[1] = (rc - 16) as u8;
        return op.offset(2);
    }
    op[1] = 255;
    op[2] = (rc - 16 - 255) as u8;
    return op.offset(3);
}
/**
 * @brief Internal LZAV finishing function (stream format 2).
 *
 * Internal function writes finishing literal block(s) to the output buffer.
 * This function can be used in custom compression algorithms.
 *
 * Stream format 2.
 *
 * @param op Output buffer pointer.
 * @param lc Literal length, in bytes. Not less than `LZAV_LIT_FIN`.
 * @param ipa Literals anchor pointer.
 * @return Incremented output buffer pointer.
 */
unsafe fn lzav_write_fin_2(mut op: *mut u8, mut lc: usize, mut ipa: *const u8)
 -> *mut u8 {
    if lc < 16 {
        *op = lc as u8;
        op = op.offset(1);
    } else {
        *op = 0;
        op = op.offset(1);
        let mut lcw = lc - 16;
        while lcw > 127 {
            *op = (128 | lcw) as u8;
            lcw >>= 7;
            op = op.offset(1);
        }
        *op = lcw as u8;
        op = op.offset(1);
    }
    memcpy(op as *mut _, ipa as *const _, lc);
    return op.offset(lc);
}
/**
 * @brief Function returns buffer size required for LZAV compression.
 *
 * @param srcl The length of the source data to be compressed.
 * @return The required allocation size for destination compression buffer.
 * Always a positive value.
 */
unsafe fn lzav_compress_bound(srcl: c_int) -> c_int {
    if srcl <= 0 { return 16; }
    let k = 16 + 127 + 1;
    let l2 = srcl / (k + 6);
    return ((srcl - l2 * 6 + k - 1) / k * 2 - l2 + srcl + 16);
}
/**
 * @brief Function returns buffer size required for the higher-ratio LZAV
 * compression.
 *
 * @param srcl The length of the source data to be compressed.
 * @return The required allocation size for destination compression buffer.
 * Always a positive value.
 */
unsafe fn lzav_compress_bound_hi(srcl: c_int) -> c_int {
    if srcl <= 0 { return 16; }
    let l2 = srcl / (16 + 5);
    return ((srcl - l2 * 5 + 15) / 16 * 2 - l2 + srcl + 16);
}
pub static LZAV_TINY_MAX: c_int =

    /**
     * @brief LZAV compression function, with external buffer option.
     *
     * Function performs in-memory data compression using the LZAV compression
     * algorithm and stream format. The function produces a "raw" compressed data,
     * without a header containing data length nor identifier nor checksum.
     *
     * Note that compression algorithm and its output on the same source data may
     * differ between LZAV versions, and may differ between big- and little-endian
     * systems. However, the decompression of a compressed data produced by any
     * prior compressor version will remain possible.
     *
     * @param[in] src Source (uncompressed) data pointer, can be 0 if `srcl`
     * equals 0. Address alignment is unimportant.
     * @param[out] dst Destination (compressed data) buffer pointer. The allocated
     * size should be at least lzav_compress_bound() bytes large. Address
     * alignment is unimportant. Should be different to `src`.
     * @param srcl Source data length, in bytes, can be 0: in this case the
     * compressed length is assumed to be 0 as well.
     * @param dstl Destination buffer's capacity, in bytes.
     * @param ext_buf External buffer to use for hash-table, set to 0 for the
     * function to manage memory itself (via standard `malloc`). Supplying a
     * pre-allocated buffer is useful if compression is performed during
     * application's operation often: this reduces memory allocation overhead and
     * fragmentation. Note that the access to the supplied buffer is not
     * implicitly thread-safe. Buffer's address must be aligned to 32 bits.
     * @param ext_bufl The capacity of the `ext_buf`, in bytes, should be a
     * power-of-2 value. Set to 0 if `ext_buf` is 0. The capacity should not be
     * lesser than 4 x `srcl`, and for default compression ratio should not be
     * greater than 1 MiB. Same `ext_bufl` value can be used for any smaller
     * source data. Using smaller `ext_bufl` values reduces the compression ratio
     * and, at the same time, increases compression speed. This aspect can be
     * utilized on memory-constrained and low-performance processors.
     * @return The length of compressed data, in bytes. Returns 0 if `srcl` is
     * lesser or equal to 0, or if `dstl` is too small, or if buffer pointers are
     * invalid, or if not enough memory.
     */
    // Add optimized small data functions
    32;
pub static LZAV_FMT_CUR: c_int = 2;
pub static LZAV_REF_MIN: c_int = 6;
unsafe fn lzav_tiny_compress(src: *const c_void, dst: *mut c_void,
                             srcl: c_int, dstl: c_int) -> c_int {
    if srcl <= 0 || srcl > LZAV_TINY_MAX || dstl < srcl + 2 {
        return 0; // Format byte
    } // Length byte
    let mut op = dst as *mut u8; // Direct copy
    *{ let mut _t = op; op = op.offset(1); _t } =
        LZAV_FMT_CUR << 4 | LZAV_REF_MIN;
    *{ let mut _t = op; op = op.offset(1); _t } = srcl as u8;
    loop  {
        if (srcl) > 0 { memcpy(op as *mut _, src, srcl); }
        if 0 == 0 { break  };
    }
    return srcl + 2;
}
pub static LZAV_LIT_FIN: c_int =

    // Optimize existing compress function
    // Fast path for tiny data
    // prefix byte
    // Handle extremely small data
    6;
pub static LZAV_SMALL_MAX: c_int =
     // On-stack hash-table.
    // Hash-table allocated on heap.
    // The actual hash-table pointer.
    // Hash-table's size in bytes (power-of-2).
    // Optimize hash table size for small data
    256;
pub static LZAV_MIN_HTABLE: usize = 512;
pub static LZAV_WIN_LEN: c_int =
     // Hash mask.
    // Source data pointer.
    // End pointer.
    // Hashing threshold, avoids I/O OOB.
    // Literals anchor pointer.
    // Pointer to the latest offset carry block header.
    // Offset carry shift.
    // Running average of hash match rate (*2^15).
    // Two-factor average: success (0-64) by average reference length.
    // PRNG bit derived from the non-matching offset.
    // Skip source bytes, to avoid OOB in back-match.
    // Initialize the hash-table. Each hash-table item consists of 2 tuples
    // (4 initial match bytes; 32-bit source data offset). Set source data
    // offset to avoid OOB in back-match.
    // Hash source data (endianness is unimportant for compression
    // efficiency). Hash is based on the "komihash" math construct, see
    // https://github.com/avaneev/komihash for details.
    // Hash-table access.
    // Tuple 1's match word.
    // At window pointer.
    // Find source data in hash-table tuples.
    // Reference offset (distance).
    1 << 23;
pub static LZAV_REF_LEN: c_long =

    // Small offsets may be inefficient.
    // Source data and hash-table entry match.
    // Disallow reference copy overlap by using `d` as max match length.
    (6 + 15 + 255 + 254);
unsafe fn lzav_compress(src: *const c_void, dst: &mut c_void, srcl: c_int,
                        dstl: c_int, ext_buf: *mut c_void, ext_bufl: c_int)
 -> c_int {
    if srcl <= LZAV_TINY_MAX {
        return lzav_tiny_compress(src, dst, srcl, dstl);
    }
    if (srcl <= 0) | (src == ptr::null_mut()) | (dst == ptr::null_mut()) |
           (src == (dst as *const c_void)) |
           (dstl < lzav_compress_bound(srcl)) != 0 {
        return 0;
    }
    let mut op = dst as *mut u8;
    *op = (LZAV_FMT_CUR << 4 | LZAV_REF_MIN) as u8;
    op = op.offset(1);
    if srcl < 16 {
        *{ let mut _t = op; op = op.offset(1); _t } = srcl as u8;
        loop  {
            if (srcl) > 0 { memcpy(op as *mut _, src, srcl); }
            if 0 == 0 { break  };
        }
        if srcl > LZAV_LIT_FIN - 1 { return 2 + srcl; }
        memset(op.offset(srcl), 0, LZAV_LIT_FIN - srcl);
        return 2 + LZAV_LIT_FIN;
    }
    let mut stack_buf: [u32; 4096];
    let mut alloc_buf = ptr::null_mut();
    let mut ht = stack_buf as *mut u8;
    let mut htsize: usize;
    if srcl <= LZAV_SMALL_MAX {
        htsize = LZAV_MIN_HTABLE;
    } else {
        htsize = (1 << 7) * std::mem::size_of::<u32>() * 4;
        while htsize != (1 << 20) && (htsize >> 2) < (srcl as usize) {
            htsize <<= 1;
        };
    }
    if ext_buf == ptr::null_mut() {
        while htsize != (1 << 20) && (htsize >> 2) < (srcl as usize) {
            htsize <<= 1;
        }
        if htsize > std::mem::size_of::<[u32; 4096]>() {
            alloc_buf = malloc(htsize);
            if alloc_buf == ptr::null_mut() { return 0; }
            ht = alloc_buf as *mut u8;
        };
    } else {
        let mut htsizem: usize;
        if ext_bufl > (std::mem::size_of::<[u32; 4096]>() as c_int) {
            htsizem = ext_bufl as usize;
            ht = ext_buf as *mut u8;
        } else { htsizem = std::mem::size_of::<[u32; 4096]>(); }
        while (htsize >> 2) < (srcl as usize) {
            let htsize2 = htsize << 1;
            if htsize2 > htsizem { break ; }
            htsize = htsize2;
        };
    }
    let hmask = ((htsize - 1) ^ 15) as u32;
    let mut ip = src as *const u8;
    let ipe = ip.offset(srcl).offset(-LZAV_LIT_FIN);
    let ipet = ipe.offset(-9);
    let mut ipa = ip;
    let mut cbp = op;
    let mut csh = 0;
    let mut mavg = 100 << 21;
    let mut rndb = 0;
    ip = ip.offset(16);
    let mut initv = [0, 16];
    if __builtin_expect((ip < ipet), 1) != 0 {
        memcpy(initv as *mut _, ip as *const _, 4);
    }
    let mut ht32 = ht as *mut u32;
    let ht32e = ht.offset(htsize) as *mut u32;
    while ht32 != ht32e {
        ht32[0] = initv[0];
        ht32[1] = initv[1];
        ht32 = ht32.offset(2);
    }
    while __builtin_expect((ip < ipet), 1) != 0 {
        let mut iw1: u32;
        let mut iw2: u16;
        let mut ww2: u16;
        memcpy(&mut iw1, ip as *const _, 4);
        let Seed1 = 608135816 ^ iw1;
        memcpy(&mut iw2, ip.offset(4), 2);
        let hm = (Seed1 as u64) * ((2242054355 ^ iw2) as u32);
        let hval = (hm as u32) ^ ((hm >> 32) as u32);
        let hp = ht.offset((hval & hmask)) as *mut u32;
        let ipo = ip.offset(-(src as *const u8)) as u32;
        let hw1 = hp[0];
        let mut wp: *const u8;
        let mut d: usize;
        let mut ml: usize;
        let mut rc: usize;
        let mut lc: usize;
        if __builtin_expect((iw1 != hw1), 1) != 0 {
            if __builtin_expect((iw1 != hp[2]), 1) != 0 { break _no_match ; }
            wp = (src as *const u8).offset(hp[3]);
            memcpy(&mut ww2, wp.offset(4), 2);
            if __builtin_expect((iw2 != ww2), 0) != 0 { break _no_match ; };
        } else {
            wp = (src as *const u8).offset(hp[1]);
            memcpy(&mut ww2, wp.offset(4), 2);
            if __builtin_expect((iw2 != ww2), 0) != 0 {
                if __builtin_expect((iw1 != hp[2]), 1) != 0 {
                    break _no_match ;
                }
                wp = (src as *const u8).offset(hp[3]);
                memcpy(&mut ww2, wp.offset(4), 2);
                if __builtin_expect((iw2 != ww2), 0) != 0 {
                    break _no_match ;
                };
            };
        }
        d = ip.offset(-wp);
        if __builtin_expect(((d < 8) | (d > LZAV_WIN_LEN - 1)) != 0, 0) != 0 {
            break _d_oob ;
        }
        ml = if d > LZAV_REF_LEN { LZAV_REF_LEN } else { d };
        if __builtin_expect((ip.offset(ml) > ipe), 0) != 0 {
            // Make sure `LZAV_LIT_FIN` literals remain on finish.
            ml = ipe.offset(-ip);
        }
        if __builtin_expect((d > 273), 1) != 0 {
            // Update a matching entry which is not inside max reference
            // length's range. Otherwise, source data consisting of same-byte
            // runs won't compress well.
            if __builtin_expect((iw1 == hw1), 1) != 0
                // Replace tuple, or insert.
               {
                hp[1] = ipo;
            } else { hp[2] = hw1; hp[3] = hp[1]; hp[0] = iw1; hp[1] = ipo; };
        }
        rc =
            LZAV_REF_MIN +
                lzav_match_len(ip.offset(LZAV_REF_MIN),
                               wp.offset(LZAV_REF_MIN), ml - LZAV_REF_MIN);
        lc = ip.offset(-ipa);
        if __builtin_expect((lc != 0), 0) != 0 {
            // Try to consume literals by finding a match at a back-position.
            ml -= rc; // Speed-up threshold.
            let mut bmc = if lc > 16 { 16 } else { lc };
            if __builtin_expect((ml > bmc), 1) != 0 { ml = bmc; }
            bmc = lzav_match_len_r(ip, wp, ml);
            if __builtin_expect((bmc != 0), 0) != 0 {
                rc += bmc;
                ip = ip.offset(-bmc);
                lc -= bmc;
            };
        }
        op =
            lzav_write_blk_2(op, lc, rc, d, ipa, &mut cbp, &mut csh,
                             LZAV_REF_MIN);
        ip = ip.offset(rc);
        ipa = ip;
        mavg += (((rc << 21) as isize) - mavg) >> 10;
        continue ;
        _d_oob: loop  { ip = ip.offset(1); break  }
        if __builtin_expect((d < LZAV_WIN_LEN), 1) != 0 { continue ; }
        hp[1 + (iw1 != hw1) * 2] = ipo;
        continue ;
        _no_match: loop  { hp[2] = iw1; break  }
        hp[3] = ipo;
        mavg -= mavg >> 11;
        if mavg < (200 << 14) && ip != ipa {
            // Compression speed-up technique that keeps the number of hash
            // evaluations around 45% of compressed data length. In some cases
            // reduces the number of blocks by several percent.
            ip =
                ip.offset(1 +
                              rndb); // Use PRNG bit to dither match positions.
            rndb = ipo & 1; // Delay to decorrelate from current match.
            if __builtin_expect((mavg < (130 << 14)), 0) != 0 {
                ip = ip.offset(1); // Gradually faster.
                if __builtin_expect((mavg < (100 << 14)), 0) != 0 {
                    ip = ip.offset(100 - (mavg >> 14));
                };
            };
        }
        ip = ip.offset(1);
    }
    if alloc_buf != ptr::null_mut() { free(alloc_buf); }
    return (lzav_write_fin_2(op, ipe.offset(-ipa) + LZAV_LIT_FIN,
                             ipa).offset(-(dst as *mut u8)) as c_int);
}
/**
 * @brief Default LZAV compression function.
 *
 * Function performs in-memory data compression using the LZAV compression
 * algorithm, with the default settings.
 *
 * See the lzav_compress() function for a more detailed description.
 *
 * @param[in] src Source (uncompressed) data pointer.
 * @param[out] dst Destination (compressed data) buffer pointer. The allocated
 * size should be at least lzav_compress_bound() bytes large.
 * @param srcl Source data length, in bytes.
 * @param dstl Destination buffer's capacity, in bytes.
 * @return The length of compressed data, in bytes. Returns 0 if `srcl` is
 * lesser or equal to 0, or if `dstl` is too small, or if not enough memory.
 */
unsafe fn lzav_compress_default(src: *const c_void, dst: *mut c_void,
                                srcl: c_int, dstl: c_int) -> c_int {
    return lzav_compress(src, dst, srcl, dstl, ptr::null_mut(), 0);
}
/**
 * @brief Higher-ratio LZAV compression function (much slower).
 *
 * Function performs in-memory data compression using the higher-ratio LZAV
 * compression algorithm.
 *
 * @param[in] src Source (uncompressed) data pointer.
 * @param[out] dst Destination (compressed data) buffer pointer. The allocated
 * size should be at least lzav_compress_bound_hi() bytes large.
 * @param srcl Source data length, in bytes.
 * @param dstl Destination buffer's capacity, in bytes.
 * @return The length of compressed data, in bytes. Returns 0 if `srcl` is
 * lesser or equal to 0, or if `dstl` is too small, or if buffer pointers are
 * invalid, or if not enough memory.
 */
unsafe fn lzav_compress_hi(src: *const c_void, dst: &mut c_void, srcl: c_int,
                           dstl: c_int) -> c_int {
    if (srcl <= 0) | (src == ptr::null_mut()) | (dst == ptr::null_mut()) |
           (src == (dst as *const c_void)) |
           (dstl < lzav_compress_bound_hi(srcl)) != 0 {
        return 0; // Minimal reference length.
    } // Destination (compressed data) pointer.
    let mref = 5; // Write prefix byte.
    let mlen = LZAV_REF_LEN - LZAV_REF_MIN + mref;
    let mut op = dst as *mut u8;
    *op = (LZAV_FMT_CUR << 4 | mref) as u8;
    op = op.offset(1);
    if srcl < 16 {
        // Handle a very short source data.
        *op = srcl as u8; // Hash-table's size in bytes (power-of-2).
        op = op.offset(1); // The hash-table pointer.
        memcpy(op as *mut _, src, srcl); // Hash mask.
        if srcl > LZAV_LIT_FIN - 1 {
            return (2 + srcl); // Source data pointer.
        } // End pointer.
        memset(op.offset(srcl), 0,
               LZAV_LIT_FIN - srcl); // Hashing threshold, avoids I/O OOB.
        return (2 + LZAV_LIT_FIN); // Literals anchor pointer.
    } // Pointer to the latest offset carry block header.
    let mut htsize =
        (1 << 7) * std::mem::size_of::<u32>() * 2 * 8; // Offset carry shift.
    while htsize != (1 << 23) && (htsize >> 2) < (srcl as usize) {
        htsize <<= 1;
    }
    let mut ht = malloc(htsize) as *mut u8;
    if ht == ptr::null_mut() { return 0; }
    let hmask = ((htsize - 1) ^ 63) as u32;
    let mut ip = src as *const u8;
    let ipe = ip.offset(srcl).offset(-LZAV_LIT_FIN);
    let ipet = ipe.offset(-9);
    let mut ipa = ip;
    let mut cbp = op;
    let mut csh = 0;
    // Initialize the hash-table. Each hash-table item consists of 8 tuples
    // (4 initial match bytes; 32-bit source data offset). The last value of
    // the last tuple is used as head tuple offset (an even value).
    let mut initv = [0, 0]; // Length of a previously found match.
    memcpy(initv as *mut _, ip as *const _,
           4); // Distance of a previously found match.
    let mut ht32 =
        ht as *mut u32; // Source pointer of a previously found match.
    let ht32e = ht.offset(htsize) as *mut u32;
    while ht32 != ht32e {
        ht32[0] = initv[0];
        ht32[1] = initv[1];
        ht32 = ht32.offset(2);
    }
    let mut prc = 0;
    let mut pd = 0;
    let mut pip = ip;
    while __builtin_expect((ip < ipet), 1) != 0 {
        // Hash source data (endianness is unimportant for compression
        // efficiency). Hash is based on the "komihash" math construct, see
        // https://github.com/avaneev/komihash for details.
        let mut iw1: u32;
        memcpy(&mut iw1, ip as *const _, 4);
        let hm = ((608135816 ^ iw1) as u64) * ((2242054355 ^ ip[4]) as u32);
        let hval = (hm as u32) ^ ((hm >> 32) as u32);
        // Hash-table access.
        let hp = ht.offset((hval & hmask)) as *mut u32; // Head tuple offset.
        let ipo = ip.offset(-(src as *const u8)) as u32;
        let mut ti0 = hp[15];
        // Find source data in hash-table tuples, in up to 7 previous
        // positions.
        let mut wp = ip; // Best found window pointer.
        let mut rc = 0; // Best found match length, 0 - not found.
        let mut d: usize; // Reference offset (distance).
        let mut ti = ti0;
        let mut i;
        if __builtin_expect((ip.offset(mlen) < ipe), 1) != 0 {
            // Optimized match-finding.
            i = 0; // Disallow reference copy overlap by using `d` as max
            while i < 7 {
                let ww1 = hp[ti]; // match length.
                let wp0 =
                    (src as
                         *const u8).offset(hp[ti +
                                                  1]); // Make sure `LZAV_LIT_FIN` literals remain on finish.
                d = ip.offset(-wp0);
                ti = if ti == 12 { 0 } else { ti + 2 };
                if iw1 == ww1 {
                    let rc0 =
                        4 +
                            lzav_match_len(ip.offset(4), wp0.offset(4),
                                           if d > mlen { mlen } else { d } -
                                               4);
                    if rc0 > rc + (d > (1 << 18)) { wp = wp0; rc = rc0; };
                };
                i += 1
            };
        } else {
            i = 0;
            while i < 7 {
                let ww1 = hp[ti];
                let wp0 = (src as *const u8).offset(hp[ti + 1]);
                d = ip.offset(-wp0);
                ti = if ti == 12 { 0 } else { ti + 2 };
                if iw1 == ww1 {
                    let mut ml = if d > mlen { mlen } else { d };
                    if __builtin_expect((ip.offset(ml) > ipe), 0) != 0 {
                        ml = ipe.offset(-ip);
                    }
                    let rc0 =
                        4 +
                            lzav_match_len(ip.offset(4), wp0.offset(4),
                                           ml - 4);
                    if rc0 > rc + (d > (1 << 18)) { wp = wp0; rc = rc0; };
                };
                i += 1
            };
        }
        if (rc == 0) | (d > 273) != 0 {
            // Update a matching entry which is not inside max reference
            // length's range. Otherwise, source data consisting of same-byte
            // runs won't compress well.
            ti0 = if ti0 == 0 { 12 } else { ti0 - 2 };
            hp[ti0] = iw1;
            hp[ti0 + 1] = ipo;
            hp[15] = ti0 as u32;
        }
        if (rc < mref + (d > (1 << 18))) | (d < 8) | (d > LZAV_WIN_LEN - 1) !=
               0 {
            ip = ip.offset(1);
            continue ;
        }
        // Source data and hash-table entry match of suitable length.
        let ip0 = ip;
        let mut lc = ip.offset(-ipa);
        if __builtin_expect((lc != 0), 0) != 0 {
            // Try to consume literals by finding a match at back-position.
            let mut ml = if d > mlen { mlen } else { d };
            if __builtin_expect((ip.offset(ml) > ipe), 0) != 0 {
                ml = ipe.offset(-ip);
            }
            ml -= rc;
            let wpo = wp.offset(-(src as *const u8));
            if __builtin_expect((ml > lc), 1) != 0 { ml = lc; }
            if __builtin_expect((ml > wpo), 0) != 0 { ml = wpo; }
            let bmc = lzav_match_len_r(ip, wp, ml);
            if __builtin_expect((bmc != 0), 0) != 0 {
                rc += bmc;
                ip = ip.offset(-bmc);
                lc -= bmc;
            };
        }
        if prc == 0 {
            // Save match for a later comparison.
            prc = rc;
            pd = d;
            pip = ip;
            ip = ip0.offset(1);
            continue ;
        }
        // Block size overhead estimation, and comparison with a previously
        // found match.
        let lb =
            (lc !=
                 0); // A winning previous match does not overlap a current match.
        let sh0 = 10 + (csh != 0) * 3;
        let sh = sh0 + lb * 2;
        let ov =
            lc + lb + (lc > 15) + 2 + (d >= (1 << sh)) +
                (d >= (1 << (sh + 8)));
        let plc = pip.offset(-ipa);
        let plb = (plc != 0);
        let psh = sh0 + plb * 2;
        let pov =
            plc + plb + (plc > 15) + 2 + (pd >= (1 << psh)) +
                (pd >= (1 << (psh + 8)));
        if __builtin_expect((prc * ov > rc * pov), 1) != 0 {
            if __builtin_expect((pip.offset(prc) <= ip), 0) != 0 {
                op =
                    lzav_write_blk_2(op, plc, prc, pd, ipa, &mut cbp,
                                     &mut csh, mref);
                ipa = pip.offset(prc);
                prc = rc;
                pd = d;
                pip = ip;
                ip = ip.offset(1);
                continue ;
            }
            rc = prc;
            d = pd;
            ip = pip;
            lc = plc;
        }
        op = lzav_write_blk_2(op, lc, rc, d, ipa, &mut cbp, &mut csh, mref);
        ip = ip.offset(rc);
        ipa = ip;
        prc = 0;
    }
    if prc != 0 {
        op =
            lzav_write_blk_2(op, pip.offset(-ipa), prc, pd, ipa, &mut cbp,
                             &mut csh, mref);
        ipa = pip.offset(prc);
    }
    free(ht as *mut _);
    return (lzav_write_fin_2(op, ipe.offset(-ipa) + LZAV_LIT_FIN,
                             ipa).offset(-(dst as *mut u8)) as c_int);
}
/**
 * @brief Internal LZAV decompression function (stream format 2).
 *
 * Function decompresses "raw" data previously compressed into the LZAV stream
 * format 2.
 *
 * This function should not be called directly since it does not check the
 * format identifier.
 *
 * @param[in] src Source (compressed) data pointer.
 * @param[out] dst Destination (decompressed data) buffer pointer.
 * @param srcl Source data length, in bytes.
 * @param dstl Expected destination data length, in bytes.
 * @param[out] pwl Pointer to variable that receives the number of bytes
 * written to the destination buffer (until error or end of buffer).
 * @return The length of decompressed data, in bytes, or any negative value if
 * some error happened.
 */
unsafe fn lzav_decompress_2(src: *const c_void, dst: *mut c_void, srcl: c_int,
                            dstl: c_int, pwl: &mut c_int) -> c_int {
    let mut ip = src as *const u8; // Compressed data pointer.
    let ipe = ip.offset(srcl); // Compressed data boundary pointer.
    let ipet = ipe.offset(-6); // Block header read threshold.
    let mut op = dst as *mut u8; // Destination (decompressed data) pointer.
    let ope = op.offset(dstl); // Destination boundary pointer.
    let opet = ope.offset(-63); // Threshold for fast copy to destination.
    *pwl = dstl; // Minimal reference length - 1.
    let mref1 =
        (*ip & 15) - 1; // Current block header, updated in each branch.
    let mut bh = 0; // Reference offset carry value.
    let mut cv = 0; // Reference offset carry shift.
    let mut csh = 0; // Advance beyond prefix byte.
    ip = ip.offset(1); // Source data pointer.
    if __builtin_expect((ip >= ipet), 0) != 0 {
        break _err_srcoob ; // Byte copy count.
    } // Block type.
    bh = *ip; // Block type 0.
    while __builtin_expect((ip < ipet), 1) != 0 {
        let mut ipd: *const u8; // True, if no additional length byte.
        let mut cc: usize; // Reference block follows, if not EOS.
        let mut bt: usize; // No more than 4 additional bytes.
        if __builtin_expect(((bh & 48) == 0), 0) != 0 {
            let mut ncv = bh >> 6; // Reference block follows, if not EOS.
            ip = ip.offset(1);
            cc = bh & 15;
            if __builtin_expect((cc != 0), 1) != 0 {
                ipd = ip;
                ncv <<= csh;
                ip = ip.offset(cc);
                if __builtin_expect(((op < opet) &
                                         (ipd < ipe.offset(-15).offset(-7)))
                                        != 0, 1) != 0 {
                    cv |= ncv;
                    csh += 2;
                    bh = *ip;
                    memcpy(op as *mut _, ipd as *const _, 16);
                    op = op.offset(cc);
                    break _refblk ;
                };
            } else {
                let mut lcw = *ip;
                ncv <<= csh;
                ip = ip.offset(1);
                cc = lcw & 127;
                let mut sh = 7;
                while (lcw & 128) != 0 {
                    lcw = *ip;
                    ip = ip.offset(1);
                    cc |= (lcw & 127) << sh;
                    if sh == 28 { break ; }
                    sh += 7;
                }
                cc += 16;
                ipd = ip;
                ip = ip.offset(cc);
                if __builtin_expect(((op < opet) &
                                         (ipd < ipe.offset(-63).offset(-16)))
                                        != 0, 1) != 0 {
                    memcpy(op as *mut _, ipd as *const _, 16);
                    memcpy(op.offset(16), ipd.offset(16), 16);
                    memcpy(op.offset(32), ipd.offset(32), 16);
                    memcpy(op.offset(48), ipd.offset(48), 16);
                    if __builtin_expect((cc < 65), 1) != 0 {
                        cv |= ncv;
                        csh += 2;
                        bh = *ip;
                        op = op.offset(cc);
                        break _refblk ;
                    }
                    ipd = ipd.offset(64);
                    op = op.offset(64);
                    cc -= 64;
                };
            }
            cv |= ncv;
            csh += 2;
            if __builtin_expect((ip < ipe), 1) != 0 {
                bh = *ip;
            } else if __builtin_expect((ip != ipe), 0) != 0 {
                break _err_srcoob_lit ;
            }
            if __builtin_expect((op.offset(cc) > ope), 0) != 0 {
                break _err_dstoob_lit ;
            }
            // This and other alike copy-blocks are transformed into fast SIMD
            // instructions, by a modern compiler. Direct use of `memcpy` is
            // slower due to shortness of data remaining to copy, on average.
            while cc != 0 {
                *op = *ipd; // True, if no additional length byte.
                ipd = ipd.offset(1);
                op = op.offset(1);
                cc -= 1;
            }
            continue ;
            _err_srcoob_lit: loop  { cc = ipe.offset(-ipd); break  }
            if op.offset(cc) < ope {
                memcpy(op as *mut _, ipd as *const _, cc);
                *pwl = op.offset(cc).offset(-(dst as *mut u8)) as c_int;
            } else { memcpy(op as *mut _, ipd as *const _, ope.offset(-op)); }
            return (LZAV_E_SRCOOB);
            _err_dstoob_lit:
                loop  {
                    memcpy(op as *mut _, ipd as *const _, ope.offset(-op));
                    break
                }
            return (LZAV_E_DSTOOB);
        }
        _refblk: loop  { bt = (bh >> 4) & 3; break  }
        ip = ip.offset(1);
        let bt8 = (bt << 3) as c_int;
        let mut bv: u32;
        memcpy(&mut bv, ip as *const _, 4);
        let om = ((1 << bt8) - 1) as u32;
        ip = ip.offset(bt);
        let o = bv & om;
        bv >>= bt8;
        let mut ocsh = [0, 0, 0, 3];
        let wcsh = ocsh[bt];
        let d = (bh >> 6 | (o & 2097151) << 2) << csh | cv;
        ipd = op.offset(-d);
        if __builtin_expect(((dst as *mut u8).offset(d) > op), 0) != 0 {
            break _err_refoob ;
        }
        csh = (wcsh);
        cv = (o >> 21);
        cc = bh & 15;
        if __builtin_expect((cc != 0), 1) != 0 {
            bh = bv & 255;
            cc += mref1;
            if __builtin_expect((op < opet), 1) != 0 {
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd as *const _, 16);
                    memcpy(op as *mut _, tmp as *const _, 16);
                }
                {
                    let mut tmp: [u8; 4];
                    memcpy(tmp as *mut _, ipd.offset(16), 4);
                    memcpy(op.offset(16), tmp as *const _, 4);
                }
                op = op.offset(cc);
                continue ;
            };
        } else {
            bh = bv & 255;
            if __builtin_expect((bh == 255), 0) != 0 {
                cc = 16 + mref1 + 255 + ip[1];
                bh = ip[2];
                ip = ip.offset(2);
            } else { cc = 16 + mref1 + bh; ip = ip.offset(1); bh = *ip; }
            if __builtin_expect((op < opet), 1) != 0 {
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd as *const _, 16);
                    memcpy(op as *mut _, tmp as *const _, 16);
                }
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd.offset(16), 16);
                    memcpy(op.offset(16), tmp as *const _, 16);
                }
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd.offset(32), 16);
                    memcpy(op.offset(32), tmp as *const _, 16);
                }
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd.offset(48), 16);
                    memcpy(op.offset(48), tmp as *const _, 16);
                }
                if __builtin_expect((cc < 65), 1) != 0 {
                    op = op.offset(cc);
                    continue ;
                }
                ipd = ipd.offset(64);
                op = op.offset(64);
                cc -= 64;
            };
        }
        if __builtin_expect((op.offset(cc) > ope), 0) != 0 {
            break _err_dstoob_ref ;
        }
        while cc != 0 {
            *op = *ipd;
            ipd = ipd.offset(1);
            op = op.offset(1);
            cc -= 1;
        }
        continue ;
        _err_dstoob_ref:
            loop  {
                memmove(op as *mut _, ipd as *const _, ope.offset(-op));
                break
            }
        return (LZAV_E_DSTOOB);
    }
    if __builtin_expect((op != ope), 0) != 0 { break _err_dstlen ; }
    return (op.offset(-(dst as *mut u8)) as c_int);
    _err_srcoob:
        loop  { *pwl = op.offset(-(dst as *mut u8)) as c_int; break  }
    return (LZAV_E_SRCOOB);
    _err_refoob:
        loop  { *pwl = op.offset(-(dst as *mut u8)) as c_int; break  }
    return (LZAV_E_REFOOB);
    _err_dstlen:
        loop  { *pwl = op.offset(-(dst as *mut u8)) as c_int; break  }
    return (LZAV_E_DSTLEN);
}
/**
 * @brief Internal LZAV decompression function (stream format 1).
 *
 * Function decompresses "raw" data previously compressed into the LZAV stream
 * format 1.
 *
 * This function should not be called directly since it does not check the
 * format identifier.
 *
 * @param[in] src Source (compressed) data pointer.
 * @param[out] dst Destination (decompressed data) buffer pointer.
 * @param srcl Source data length, in bytes.
 * @param dstl Expected destination data length, in bytes.
 * @return The length of decompressed data, in bytes, or any negative value if
 * some error happened.
 */
unsafe fn lzav_decompress_1(src: *const c_void, dst: *mut c_void, srcl: c_int,
                            dstl: c_int) -> c_int {
    let mut ip = src as *const u8; // Compressed data pointer.
    let ipe = ip.offset(srcl); // Compressed data boundary pointer.
    let ipet = ipe.offset(-5); // Block header read threshold.
    let mut op = dst as *mut u8; // Destination (decompressed data) pointer.
    let ope = op.offset(dstl); // Destination boundary pointer.
    let opet = ope.offset(-63); // Threshold for fast copy to destination.
    let mref1 = (*ip & 15) - 1; // Minimal reference length - 1.
    let mut bh = 0; // Current block header, updated in each branch.
    let mut cv = 0; // Reference offset carry value.
    let mut csh = 0; // Reference offset carry shift.
    ip = ip.offset(1); // Advance beyond prefix byte.
    if __builtin_expect((ip >= ipet), 0) != 0 {
        break _err_srcoob ; // Source data pointer.
    } // Byte copy count.
    bh = *ip; // Block type 0.
    while __builtin_expect((ip < ipet), 1) != 0 {
        let mut ipd: *const u8; // True, if no additional length byte.
        let mut cc: usize; // Reference block follows, if not EOS.
        if __builtin_expect(((bh & 48) == 0), 0) != 0 {
            cv = bh >> 6;
            csh = 2;
            ip = ip.offset(1);
            cc = bh & 15;
            if __builtin_expect((cc != 0), 1) != 0 {
                ipd = ip;
                ip = ip.offset(cc);
                if __builtin_expect(((op < opet) &
                                         (ipd < ipe.offset(-15).offset(-6)))
                                        != 0, 1) != 0 {
                    bh = *ip;
                    memcpy(op as *mut _, ipd as *const _, 16);
                    op = op.offset(cc);
                    break _refblk ;
                };
            } else {
                let mut bv: u16;
                memcpy(&mut bv, ip as *const _, 2);
                let l2 = bv & 255;
                cc = 16;
                ip = ip.offset(1);
                let lb = (l2 == 255);
                cc += l2 + ((bv >> 8) & (256 - lb));
                ip = ip.offset(lb);
                ipd = ip;
                ip = ip.offset(cc);
                if __builtin_expect(((op < opet) &
                                         (ipd < ipe.offset(-63).offset(-1)))
                                        != 0, 1) != 0 {
                    memcpy(op as *mut _, ipd as *const _, 16);
                    memcpy(op.offset(16), ipd.offset(16), 16);
                    memcpy(op.offset(32), ipd.offset(32), 16);
                    memcpy(op.offset(48), ipd.offset(48), 16);
                    if __builtin_expect((cc < 65), 1) != 0 {
                        bh = *ip;
                        op = op.offset(cc);
                        continue ;
                    }
                    ipd = ipd.offset(64);
                    op = op.offset(64);
                    cc -= 64;
                };
            }
            if __builtin_expect((ip < ipe), 1) != 0 {
                bh = *ip;
            } else if __builtin_expect((ip != ipe), 0) != 0 {
                break _err_srcoob ;
            }
            if __builtin_expect((op.offset(cc) > ope), 0) != 0 {
                break _err_dstoob ;
            }
            // This and other alike copy-blocks are transformed into fast SIMD
            // instructions, by a modern compiler. Direct use of `memcpy` is
            // slower due to shortness of data remaining to copy, on average.
            while cc != 0 {
                *op = *ipd; // True, if block type 1.
                ipd = ipd.offset(1); // Block type 2 or 3.
                op = op.offset(1); // True, if block type 2.
                cc -= 1; // Block type 3.
            } // True, if no additional length byte.
            continue ; // LZAV_FMT_MIN < 2
        }
        _refblk: loop  { cc = bh & 15; break  }
        if __builtin_expect(((bh & 32) == 0), 0) != 0 {
            let d = (bh >> 6 | (ip[1] as usize) << 2) << csh | cv;
            ipd = op.offset(-d);
            if __builtin_expect(((dst as *mut u8).offset(d) > op), 0) != 0 {
                break _err_refoob ;
            }
            csh = 0;
            cv = 0;
            ip = ip.offset(2);
            bh = *ip;
        } else {
            if __builtin_expect(((bh & 16) == 0), 1) != 0 {
                let mut bv: u16;
                memcpy(&mut bv, ip.offset(1), 2);
                let d = (bh >> 6 | (bv as usize) << 2) << csh | cv;
                ipd = op.offset(-d);
                if __builtin_expect(((dst as *mut u8).offset(d) > op), 0) != 0
                   {
                    break _err_refoob ;
                }
                csh = 0;
                cv = 0;
                ip = ip.offset(3);
                bh = *ip;
            } else {
                let mut bv: u32;
                memcpy(&mut bv, ip.offset(1), 4);
                let d = (bv & 16777215) << csh | cv;
                ipd = op.offset(-d);
                if __builtin_expect(((dst as *mut u8).offset(d) > op), 0) != 0
                   {
                    break _err_refoob ;
                }
                csh = 2;
                cv = (bh >> 6);
                ip = ip.offset(4);
                bh = bv >> 24;
            };
        }
        if __builtin_expect((cc != 0), 1) != 0 {
            cc += mref1;
            if __builtin_expect((op < opet), 1) != 0 {
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd as *const _, 16);
                    memcpy(op as *mut _, tmp as *const _, 16);
                }
                {
                    let mut tmp: [u8; 4];
                    memcpy(tmp as *mut _, ipd.offset(16), 4);
                    memcpy(op.offset(16), tmp as *const _, 4);
                }
                op = op.offset(cc);
                continue ;
            };
        } else {
            cc = 16 + mref1 + bh;
            ip = ip.offset(1);
            bh = *ip;
            if __builtin_expect((op < opet), 1) != 0 {
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd as *const _, 16);
                    memcpy(op as *mut _, tmp as *const _, 16);
                }
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd.offset(16), 16);
                    memcpy(op.offset(16), tmp as *const _, 16);
                }
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd.offset(32), 16);
                    memcpy(op.offset(32), tmp as *const _, 16);
                }
                {
                    let mut tmp: [u8; 16];
                    memcpy(tmp as *mut _, ipd.offset(48), 16);
                    memcpy(op.offset(48), tmp as *const _, 16);
                }
                if __builtin_expect((cc < 65), 1) != 0 {
                    op = op.offset(cc);
                    continue ;
                }
                ipd = ipd.offset(64);
                op = op.offset(64);
                cc -= 64;
            };
        }
        if __builtin_expect((op.offset(cc) > ope), 0) != 0 {
            break _err_dstoob ;
        }
        while cc != 0 {
            *op = *ipd;
            ipd = ipd.offset(1);
            op = op.offset(1);
            cc -= 1;
        };
    }
    if __builtin_expect((op != ope), 0) != 0 { break _err_dstlen ; }
    return (op.offset(-(dst as *mut u8)) as c_int);
    _err_srcoob: loop  { return (LZAV_E_SRCOOB); break  }
    _err_dstoob: loop  { return (LZAV_E_DSTOOB); break  }
    _err_refoob: loop  { return (LZAV_E_REFOOB); break  }
    _err_dstlen: loop  { return (LZAV_E_DSTLEN); break  };
}
/**
 * @brief LZAV decompression function (partial).
 *
 * Function decompresses "raw" data previously compressed into the LZAV stream
 * format, for partial or recovery decompression. For example, this function
 * can be used to decompress only an initial segment of a larger data block.
 *
 * @param[in] src Source (compressed) data pointer, can be 0 if `srcl` is 0.
 * Address alignment is unimportant.
 * @param[out] dst Destination (decompressed data) buffer pointer. Address
 * alignment is unimportant. Should be different to `src`.
 * @param srcl Source data length, in bytes, can be 0.
 * @param dstl Destination buffer length, in bytes, can be 0.
 * @return The length of decompressed data, in bytes. Always a non-negative
 * value (error codes are not returned).
 */
unsafe fn lzav_decompress_partial(src: &c_void, dst: *mut c_void, srcl: c_int,
                                  dstl: c_int) -> c_int {
    if srcl <= 0 || src == ptr::null_mut() || dst == ptr::null_mut() ||
           src == (dst as *const c_void) || dstl <= 0 {
        return 0;
    }
    let fmt = *(src as *const u8) >> 4;
    let mut dl = 0;
    if fmt == 2 { lzav_decompress_2(src, dst, srcl, dstl, &mut dl); }
    return (dl);
}
/**
 * @brief LZAV decompression function.
 *
 * Function decompresses "raw" data previously compressed into the LZAV stream
 * format.
 *
 * Note that while the function does perform checks to avoid OOB memory
 * accesses, and checks for decompressed data length equality, this is not a
 * strict guarantee of a valid decompression. In cases when the compressed
 * data is stored in a long-term storage without embedded data integrity
 * mechanisms (e.g., a database without RAID 1 guarantee, a binary container
 * without a digital signature nor CRC), then a checksum (hash) of the
 * original uncompressed data should be stored, and then evaluated against
 * that of the decompressed data. Also, a separate checksum (hash) of
 * application-defined header, which contains uncompressed and compressed data
 * lengths, should be checked before decompression. A high-performance
 * "komihash" hash function can be used to obtain a hash value of the data.
 *
 * @param[in] src Source (compressed) data pointer, can be 0 if `srcl` is 0.
 * Address alignment is unimportant.
 * @param[out] dst Destination (decompressed data) buffer pointer. Address
 * alignment is unimportant. Should be different to `src`.
 * @param srcl Source data length, in bytes, can be 0.
 * @param dstl Expected destination data length, in bytes, can be 0. Should
 * not be confused with the actual size of the destination buffer (which may
 * be larger).
 * @return The length of decompressed data, in bytes, or any negative value if
 * some error happened. Always returns a negative value if the resulting
 * decompressed data length differs from `dstl`. This means that error result
 * handling requires just a check for a negative return value (see the
 * `LZAV_E_` macros for possible values).
 */
// Optimize decompress function for small data
unsafe fn lzav_decompress(src: &c_void, dst: *mut c_void, srcl: c_int,
                          dstl: c_int) -> c_int {
    if srcl < 0 {
        return LZAV_E_PARAMS; // Fast path for tiny data
    } // LZAV_FMT_MIN < 2
    if srcl == 0 {
        return if dstl == 0 { 0 } else { LZAV_E_PARAMS }; // LZAV_INCLUDED
    }
    if src == ptr::null_mut() || dst == ptr::null_mut() ||
           src == (dst as *const c_void) || dstl <= 0 {
        return LZAV_E_PARAMS;
    }
    let fmt = *(src as *const u8) >> 4;
    let mut ip = src as *const u8;
    if srcl <= LZAV_TINY_MAX + 2 && ip[0] >> 4 == LZAV_FMT_CUR {
        let len = ip[1];
        if len <= LZAV_TINY_MAX && len <= dstl {
            memcpy(dst, ip.offset(2), len);
            return len;
        };
    }
    if fmt == 2 {
        let mut tmp;
        return lzav_decompress_2(src, dst, srcl, dstl, &mut tmp);
    }
    if fmt == 1 { return lzav_decompress_1(src, dst, srcl, dstl); }
    return LZAV_E_UNKFMT;
}
unsafe fn lzav_pool_free(mut pool: &mut lzav_pool_t) {
    for mut i in 0..pool.count { free(pool.blocks[i] as *mut _); }
    pool.count = 0;
}