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

#ifndef LZAV_INCLUDED
#define LZAV_INCLUDED

#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include <stdbool.h>

// Consolidate version and format definitions
#define LZAV_API_VER 0x106
#define LZAV_VER_STR "4.5"
#define LZAV_FMT_MIN 1
#define LZAV_FMT_CUR 2

// Unified error codes
typedef enum {
    LZAV_E_PARAMS = -1,
    LZAV_E_SRCOOB = -2,
    LZAV_E_DSTOOB = -3,
    LZAV_E_REFOOB = -4,
    LZAV_E_DSTLEN = -5,
    LZAV_E_UNKFMT = -6
} LZAV_ERROR;

// Consolidated compression constants
#define LZAV_WIN_LEN ( 1 << 23 )
#define LZAV_REF_MIN 6
#define LZAV_REF_LEN ( LZAV_REF_MIN + 15 + 255 + 254 )
#define LZAV_LIT_FIN 6
static inline void *lzav_memcpy(void *dest, const void *src, size_t n) {
    unsigned char *d = (unsigned char *)dest;
    const unsigned char *s = (const unsigned char *)src;
    const size_t word_size = sizeof(uintptr_t);
    const uintptr_t word_mask = word_size - 1;

    // Fast path for small copies using bitwise ops
    if (n < (word_size << 1)) {
        while (n--) *d++ = *s++;
        return dest;
    }

    // Align destination using bitwise AND
    size_t head = (~(uintptr_t)d + 1) & word_mask;
    if (head) {
        n -= head;
        while (head--) *d++ = *s++;
    }

    uintptr_t *dw = (uintptr_t *)(void *)d;
    
    if ((uintptr_t)s & word_mask) {
        // Unaligned source - use SWAR
        const unsigned char *sw = s;
        size_t words = n >> (word_size == 8 ? 3 : 2);
        while (words--) {
            uintptr_t word = 0;
            for (size_t i = 0; i < word_size; i++) {
                word |= ((uintptr_t)sw[i] << (i << 3));
            }
            *dw++ = word;
            sw += word_size;
        }
        s = sw;
    } else {
        // Aligned source - use direct copy with unrolling
        const uintptr_t *sw = (const uintptr_t *)(const void *)s;
        size_t words = n >> (word_size == 8 ? 3 : 2);
        while (words >= 4) {
            dw[0] = sw[0];
            dw[1] = sw[1];
            dw[2] = sw[2];
            dw[3] = sw[3];
            dw += 4;
            sw += 4;
            words -= 4;
        }
        while (words--) *dw++ = *sw++;
        s = (const unsigned char *)sw;
    }

    d = (unsigned char *)dw;
    n &= word_mask;
    while (n--) *d++ = *s++;

    return dest;
}

static inline void *lzav_memset(void *dest, uint8_t value, size_t n) {
    unsigned char *d = (unsigned char *)dest;
    const size_t word_size = sizeof(uintptr_t);
    const uintptr_t word_mask = word_size - 1;

    if (n < (word_size << 1)) {
        while (n--) *d++ = value;
        return dest;
    }

    // Create pattern using shifts instead of multiplication
    uintptr_t pattern = value;
    pattern |= pattern << 8;
    pattern |= pattern << 16;
    if (word_size == 8) {
        pattern |= pattern << 32;
    }

    // Align destination using bitwise AND
    size_t head = (~(uintptr_t)d + 1) & word_mask;
    if (head) {
        if (head > n) head = n; // Prevent overshoot
        n -= head;
        while (head--) *d++ = value;
    }

    uintptr_t *dw = (uintptr_t *)(void *)d;
    size_t words = n >> (word_size == 8 ? 3 : 2);
    
    while (words >= 8) {
        dw[0] = pattern;
        dw[1] = pattern;
        dw[2] = pattern;
        dw[3] = pattern;
        dw[4] = pattern;
        dw[5] = pattern; 
        dw[6] = pattern;
        dw[7] = pattern;
        dw += 8;
        words -= 8;
    }

    while (words--) *dw++ = pattern;

    d = (unsigned char *)dw;
    n &= word_mask;
    while (n--) *d++ = value;

    return dest;
}

static inline void *lzav_memmove(void *dest, const void *src, size_t n) {
    unsigned char *d = (unsigned char *)dest;
    const unsigned char *s = (const unsigned char *)src;
    
    if ((uintptr_t)d - (uintptr_t)s >= n) {
        return lzav_memcpy(dest, src, n);
    }
    
    const size_t word_size = sizeof(uintptr_t);
    const uintptr_t word_mask = word_size - 1;

    d += n;
    s += n;

    if (n < (word_size << 1)) {
        while (n--) *--d = *--s;
        return dest;
    }

    // Align destination using bitwise AND
    size_t tail = (uintptr_t)d & word_mask;
    if (tail) {
        n -= tail;
        while (tail--) *--d = *--s;
    }

    uintptr_t *dw = (uintptr_t *)(void *)d;
    if ((uintptr_t)s & word_mask) {
        // Unaligned source - use SWAR
        size_t words = n >> (word_size == 8 ? 3 : 2);
        while (words--) {
            s -= word_size;
            --dw;
            uintptr_t word = 0;
            for (size_t i = 0; i < word_size; i++) {
                word |= ((uintptr_t)s[i] << (i << 3));
            }
            *dw = word;
        }
    } else {
        // Aligned source - use direct copy
        const uintptr_t *sw = (const uintptr_t *)(const void *)s;
        size_t words = n >> (word_size == 8 ? 3 : 2);
        while (words--) {
            --sw;
            --dw;
            *dw = *sw;
        }
        s = (const unsigned char *)sw;
    }

    d = (unsigned char *)dw;
    n &= word_mask;
    while (n--) *--d = *--s;

    return dest;
}

// Optimized memory operations
#define LZAV_MEMCPY_SMALL(d, s, n) \
    do { \
        if ((n) > 0) lzav_memcpy(d, s, n); \
    } while (0)

static inline void* lzav_assume_aligned(void* ptr, size_t alignment) {
    // Verify alignment is power of 2 using bitwise check
    if ((alignment & (alignment - 1)) != 0) return ptr;
    
    // Check if pointer is aligned using bitwise AND
    if ((uintptr_t)ptr & (alignment - 1)) return ptr;
    
    // Return aligned pointer
    return ptr;
}


// Unified block handling macros
#define LZAV_BLOCK_HEADER(type, length) ((type) << 4 | (length))
#define LZAV_GET_BLOCK_TYPE(header) (((header) >> 4) & 3)
#define LZAV_GET_BLOCK_LENGTH(header) ((header) & 15)

// Add small data optimization constants
#define LZAV_TINY_MAX 32    // Maximum size for tiny data optimization
#define LZAV_SMALL_MAX 256  // Maximum size for small data optimization
#define LZAV_MIN_HTABLE 512 // Minimum hash table size

// Add memory pool for small allocations
#define LZAV_POOL_SIZE 64
#define LZAV_POOL_BLOCK 2048

// Add branch prediction hints and cache optimizations
#define LZAV_CACHE_LINE 64
#define LZAV_PREFETCH_DIST 512

/**
 * @def LZAV_LITTLE_ENDIAN
 * @brief Endianness definition macro, can be used as a logical constant.
 */

#if defined( __LITTLE_ENDIAN__ ) || defined( __LITTLE_ENDIAN ) || \
	defined( _LITTLE_ENDIAN ) || defined( _WIN32 ) || defined( i386 ) || \
	defined( __i386 ) || defined( __i386__ ) || defined( __x86_64__ ) || \
	( defined( __BYTE_ORDER__ ) && __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__ )

	#define LZAV_LITTLE_ENDIAN 1

#elif defined( __BIG_ENDIAN__ ) || defined( __BIG_ENDIAN ) || \
	defined( _BIG_ENDIAN ) || defined( __SYSC_ZARCH__ ) || \
	defined( __zarch__ ) || defined( __s390x__ ) || defined( __sparc ) || \
	defined( __sparc__ ) || \
	( defined( __BYTE_ORDER__ ) && __BYTE_ORDER__ == __ORDER_BIG_ENDIAN__ )

	#define LZAV_LITTLE_ENDIAN 0

#else // defined( __BIG_ENDIAN__ )

	#warning LZAV: cannot determine endianness, assuming little-endian.

	#define LZAV_LITTLE_ENDIAN 1

#endif // defined( __BIG_ENDIAN__ )

/**
 * @def LZAV_ARCH64
 * @brief Macro that denotes availability of 64-bit instructions.
 */

#if defined( _WIN64 ) || defined( __x86_64__ ) || defined( __ia64__ ) || \
    defined( __aarch64__ ) || defined( __arm64 ) || defined( __PPC64__ ) || \
    defined( __powerpc64__ ) || defined( __LP64__ ) || defined( _LP64 )

    #define LZAV_ARCH64

#endif // 64-bit availability check

/**
 * @def LZAV_GCC_BUILTINS
 * @brief Macro that denotes availability of GCC-style built-in functions.
 */

#if defined( __GNUC__ ) || defined( __clang__ ) || \
	defined( __IBMC__ ) || defined( __IBMCPP__ ) || defined( __COMPCERT__ )

	#define LZAV_GCC_BUILTINS

#endif // GCC built-ins check

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

#if LZAV_LITTLE_ENDIAN

	#define LZAV_IEC16( x )
	#define LZAV_IEC32( x )
	#define LZAV_IEC64( x )

#else // LZAV_LITTLE_ENDIAN

	#if defined( LZAV_GCC_BUILTINS )

		#define LZAV_IEC16( x ) x = __builtin_bswap16( x )
		#define LZAV_IEC32( x ) x = __builtin_bswap32( x )

	#elif defined( _MSC_VER )

		#define LZAV_IEC16( x ) x = _byteswap_ushort( x )
		#define LZAV_IEC32( x ) x = _byteswap_ulong( x )

	#else // defined( _MSC_VER )

		#define LZAV_IEC16( x ) x = (uint16_t) ( x >> 8 | x << 8 )
		#define LZAV_IEC32( x ) x = (uint32_t) ( \
			x >> 24 | \
			( x & 0x00FF0000 ) >> 8 | \
			( x & 0x0000FF00 ) << 8 | \
			x << 24 )

	#endif // defined( _MSC_VER )

	#define LZAV_IEC64( x ) { \
		const uint64_t sw = x >> 32 | ( x & 0xFFFFFFFF ) << 32; \
		const uint64_t sw2 = ( sw & 0xFFFF0000FFFF0000 ) >> 16 | \
			( sw & 0x0000FFFF0000FFFF ) << 16; \
		x = ( sw2 & 0xFF00FF00FF00FF00 ) >> 8 | \
			( sw2 & 0x00FF00FF00FF00FF ) << 8; }

#endif // LZAV_LITTLE_ENDIAN

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

#if defined( LZAV_GCC_BUILTINS ) && \
	!( defined( __aarch64__ ) && defined( __APPLE__ ))

	#define LZAV_LIKELY( x ) __builtin_expect( x, 1 )
	#define LZAV_UNLIKELY( x ) __builtin_expect( x, 0 )

#else // Likelihood macros

	#define LZAV_LIKELY( x ) ( x )
	#define LZAV_UNLIKELY( x ) ( x )

#endif // Likelihood macros

#if defined( _MSC_VER ) && !defined( LZAV_GCC_BUILTINS )
	#include <intrin.h> // For _BitScanForwardX and _byteswap_X.
#endif // defined( _MSC_VER ) && !defined( LZAV_GCC_BUILTINS )

// Optimize hash table size for better cache utilization
#define LZAV_HASH_L1_BITS 12  // 4KB hash table fits well in L1 cache
#define LZAV_HASH_L2_BITS 15  // 32KB for larger inputs
#define LZAV_HASH_L3_BITS 17  // 128KB for maximum compression

// Power of 2 sizing ensures fast modulo via bitwise AND
#define LZAV_HASH_L1_SIZE (1U << LZAV_HASH_L1_BITS)  // 4KB
#define LZAV_HASH_L2_SIZE (1U << LZAV_HASH_L2_BITS)  // 32KB  
#define LZAV_HASH_L3_SIZE (1U << LZAV_HASH_L3_BITS)  // 128KB

// Masks for fast modulo operations
#define LZAV_HASH_L1_MASK (LZAV_HASH_L1_SIZE - 1)
#define LZAV_HASH_L2_MASK (LZAV_HASH_L2_SIZE - 1)
#define LZAV_HASH_L3_MASK (LZAV_HASH_L3_SIZE - 1)

// Faster modulus and bitwise operations
#define FAST_MOD(x, y) ((x) & ((y) - 1))
#define FAST_DIV(x, y) ((x) >> __builtin_ctz(y))
static inline size_t fast_div_pow2(size_t x, size_t y) {
    // Verify y is power of 2
    if (y & (y - 1)) return x / y;
    
    // Count trailing zeros using SWAR
    size_t shift = 0;
    size_t mask = y;
    
    if (sizeof(size_t) == 8) {
        if (!(mask & 0xFFFFFFFF)) { shift += 32; mask >>= 32; }
        if (!(mask & 0xFFFF)) { shift += 16; mask >>= 16; }
        if (!(mask & 0xFF)) { shift += 8; mask >>= 8; }
        if (!(mask & 0xF)) { shift += 4; mask >>= 4; }
        if (!(mask & 0x3)) { shift += 2; mask >>= 2; }
        if (!(mask & 0x1)) { shift += 1; }
    } else {
        if (!(mask & 0xFFFF)) { shift += 16; mask >>= 16; }
        if (!(mask & 0xFF)) { shift += 8; mask >>= 8; }
        if (!(mask & 0xF)) { shift += 4; mask >>= 4; }
        if (!(mask & 0x3)) { shift += 2; mask >>= 2; }
        if (!(mask & 0x1)) { shift += 1; }
    }
    
    return x >> shift;
}

static inline size_t lzav_get_hash_bits(const size_t input_size) {
    const size_t word_size = sizeof(size_t) * 8;
    const size_t word_mask = word_size - 1;
    size_t x = input_size;
    size_t leading_zeros = 0;
    
    // Count leading zeros using SWAR technique
    x |= (x >> 1);
    x |= (x >> 2);
    x |= (x >> 4);
    x |= (x >> 8);
    x |= (x >> 16);
    if (word_size == 64) {
        x |= (x >> 32);
    }
    
    leading_zeros = word_mask - 
        ((((x & ~(x >> 1)) != 0) << 0) |
         (((x & ~(x >> 2)) != 0) << 1) |
         (((x & ~(x >> 4)) != 0) << 2) |
         (((x & ~(x >> 8)) != 0) << 3) |
         (((x & ~(x >> 16)) != 0) << 4) |
         ((word_size == 64) ? (((x & ~(x >> 32)) != 0) << 5) : 0));

    // Calculate base bits using shifts instead of multiplication/division
    const size_t shift_13 = leading_zeros - 13;
    const size_t base_shift = (shift_13 & ~(shift_13 >> (word_size - 1)));
    const size_t base_bits = LZAV_HASH_L1_BITS | 
        ((base_shift + (base_shift >> 1)) & 
         ((leading_zeros > 13) ? -1ULL : 0));

    // Clamp using bitwise operations
    const size_t over = (base_bits > LZAV_HASH_L3_BITS) ? -1ULL : 0;
    const size_t under = (base_bits < LZAV_HASH_L1_BITS) ? -1ULL : 0;
    
    return (base_bits & ~over & ~under) | 
           (LZAV_HASH_L3_BITS & over) |
           (LZAV_HASH_L1_BITS & under);
}


// Optimized hash table size selection
static inline uint32_t lzav_get_hash_mask(const size_t input_size) {
    // Use bit shifts to determine optimal size
    if (input_size <= (16U << 10))        // <= 16KB
        return LZAV_HASH_L1_MASK;
    if (input_size <= (128U << 10))       // <= 128KB
        return LZAV_HASH_L2_MASK;
    return LZAV_HASH_L3_MASK;
}

// Fast hash function using the selected mask
static inline uint32_t lzav_hash(const uint32_t x, const uint32_t mask) {
    uint32_t x_prime = x * 0x1E35A7BDU;
    x_prime ^= x_prime >> LZAV_HASH_L1_BITS;
    x_prime *= 0x1E35A7BDU;
    return x_prime & mask;
}

// Add memory pool for small allocations
typedef struct {
    uint8_t* blocks[LZAV_POOL_SIZE];
    size_t used[LZAV_POOL_SIZE];
    size_t count;
} lzav_pool_t;

static inline void* lzav_pool_alloc(lzav_pool_t* pool, size_t size) {
    // Round up size to alignment boundary
    size = (size + 7) & ~7;
    
    // Try to allocate from existing blocks
    for (size_t i = 0; i < pool->count; ++i) {
        size_t remaining = LZAV_POOL_BLOCK - pool->used[i];
        if (remaining >= size) {
            void* ptr = pool->blocks[i] + pool->used[i];
            pool->used[i] += size;
            return ptr;
        }
    }
    
    // Need new block
    if (pool->count < LZAV_POOL_SIZE) {
        uint8_t* block = malloc(LZAV_POOL_BLOCK);
        if (!block) return NULL;
        
        pool->blocks[pool->count] = block;
        pool->used[pool->count] = size;
        ++pool->count;
        return block;
    }
    
    // Pool exhausted, fallback to direct malloc
    return malloc(size);
}

// Add SIMD hints for modern compilers
#if defined(__GNUC__) || defined(__clang__)
    #define LZAV_LIKELY(x) __builtin_expect(!!(x), 1)
    #define LZAV_UNLIKELY(x) __builtin_expect(!!(x), 0)
    #define LZAV_PREFETCH(x) __builtin_prefetch(x)
#else
    #define LZAV_LIKELY(x) (x)
    #define LZAV_UNLIKELY(x) (x)
    #define LZAV_PREFETCH(x)
#endif

static inline size_t lzav_match_len_opt(const uint8_t p1[], const uint8_t p2[], 
                                      size_t ml) {
    // Early exit for small lengths
    if (LZAV_UNLIKELY(ml < 8)) {
        size_t len = 0;
        while (len < ml && p1[len] == p2[len]) {
            ++len;
        }
        return len;
    }

    // Assume alignment of p1, p2 to 16 bytes for faster SIMD loads
    const uint8_t* p1_aligned = lzav_assume_aligned(p1, 16);
    const uint8_t* p2_aligned = lzav_assume_aligned(p2, 16);

    // Prefetch distant cache lines (unchanged)
    LZAV_PREFETCH(p1_aligned + LZAV_PREFETCH_DIST);
    LZAV_PREFETCH(p2_aligned + LZAV_PREFETCH_DIST);

    const uint8_t* const p1s = p1;
    const uint8_t* const p1e = p1 + ml;

    // Align to 8-byte boundary for better memory access
    size_t misalign = (uintptr_t)p1 & 7;
    if (misalign != 0) {
        misalign = 8 - misalign;
        while (misalign-- && p1 < p1e && *p1 == *p2) {
            ++p1;
            ++p2;
        }
        if (p1 < p1e && *p1 != *p2) {
            return p1 - p1s;
        }
    }

    #if defined(__AVX512F__)
    // Process 64 bytes at a time with AVX-512
    while (LZAV_LIKELY(p1 + 63 < p1e)) {
        __m512i v1 = _mm512_loadu_si512((__m512i*)p1);
        __m512i v2 = _mm512_loadu_si512((__m512i*)p2);
        uint64_t mask = _mm512_cmpeq_epi8_mask(v1, v2);
        if (mask != ~0ULL) {
            return p1 - p1s + __builtin_ctzll(~mask);
        }
        p1 += 64;
        p2 += 64;
    }
    #elif defined(__AVX2__)
    // Process 32 bytes at a time with AVX2
    while (LZAV_LIKELY(p1 + 31 < p1e)) {
        __m256i v1 = _mm256_loadu_si256((__m256i*)p1);
        __m256i v2 = _mm256_loadu_si256((__m256i*)p2);
        uint32_t mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(v1, v2));
        if (mask != ~0U) {
            return p1 - p1s + __builtin_ctz(~mask);
        }
        p1 += 32;
        p2 += 32;
    }
    #elif defined(__SSE2__)
    // Process 16 bytes at a time with SSE2
    while (LZAV_LIKELY(p1 + 15 < p1e)) {
        __m128i v1 = _mm_loadu_si128((__m128i*)p1);
        __m128i v2 = _mm_loadu_si128((__m128i*)p2);
        uint16_t mask = _mm_movemask_epi8(_mm_cmpeq_epi8(v1, v2));
        if (mask != 0xFFFF) {
            return p1 - p1s + __builtin_ctz(~mask);
        }
        p1 += 16;
        p2 += 16;
    }
    #endif

    // Process remaining bytes with 64-bit operations
    while (LZAV_LIKELY(p1 + 7 < p1e)) {
        uint64_t v1, v2;
        lzav_memcpy(&v1, p1, 8);
        lzav_memcpy(&v2, p2, 8);
        if (v1 != v2) {
            #if defined(__GNUC__) || defined(__clang__)
                return p1 - p1s + (__builtin_ctzll(v1 ^ v2) >> 3);
            #else
                uint64_t diff = v1 ^ v2;
                unsigned int shift = 0;
                while ((diff & 0xFF) == 0) {
                    diff >>= 8;
                    shift += 8;
                }
                return p1 - p1s + (shift >> 3);
            #endif
        }
        p1 += 8;
        p2 += 8;
    }

    // Handle remaining bytes (less than 8)
    while (p1 < p1e && *p1 == *p2) {
        ++p1;
        ++p2;
    }

    return p1 - p1s;
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

/* Keep C-compatible version only */
static inline uint32_t lzav_read32_c(const uint8_t* p) {
    uint32_t val;
    // This encapsulates lzav_memcpy but can be switched to pass-by-const-reference in C++
    lzav_memcpy(&val, p, sizeof(val));
    return val;
}

static inline size_t lzav_match_len( const uint8_t p1[], const uint8_t p2[],
	const size_t ml )
{
	const uint8_t* const p1s = p1;
	const uint8_t* const p1e = p1 + ml;

#if defined( LZAV_ARCH64 )

	while( LZAV_LIKELY( p1 + 7 < p1e ))
	{
		uint64_t v1, v2, vd;
		lzav_memcpy( &v1, p1, 8 );
		lzav_memcpy( &v2, p2, 8 );
		vd = v1 ^ v2;

		if( vd != 0 )
		{
		#if defined( LZAV_GCC_BUILTINS )

			#if LZAV_LITTLE_ENDIAN
				return p1 - p1s + ( __builtin_ctzll( vd ) >> 3 );
			#else // LZAV_LITTLE_ENDIAN
				return p1 - p1s + ( __builtin_clzll( vd ) >> 3 );
			#endif // LZAV_LITTLE_ENDIAN

		#else // defined( LZAV_GCC_BUILTINS )

			#if defined( _MSC_VER )
				uint32_t i;
				_BitScanForward64( &i, (unsigned __int64) vd );
				return p1 - p1s + ( i >> 3 );
			#else // defined( _MSC_VER )
				LZAV_IEC64( vd );
				const uint64_t m = 0x0101010101010101;
				return p1 - p1s +
					(((( vd ^ ( vd - 1 )) & ( m - 1 )) * m ) >> 56 );
			#endif // defined( _MSC_VER )

		#endif // defined( LZAV_GCC_BUILTINS )
		}

		p1 += 8;
		p2 += 8;
	}

	// At most 7 bytes left.

	if( LZAV_LIKELY( p1 + 3 < p1e ))
	{

#else // defined( LZAV_ARCH64 )

	while( LZAV_LIKELY( p1 + 3 < p1e ))
	{

#endif // defined( LZAV_ARCH64 )

		uint32_t v1, v2, vd;
		v1 = lzav_read32_c(p1);
		v2 = lzav_read32_c(p2);
		vd = v1 ^ v2;

		if( vd != 0 )
		{
		#if defined( LZAV_GCC_BUILTINS )

			#if LZAV_LITTLE_ENDIAN
				return p1 - p1s + ( __builtin_ctz( vd ) >> 3 );
			#else // LZAV_LITTLE_ENDIAN
				return p1 - p1s + ( __builtin_clz( vd ) >> 3 );
			#endif // LZAV_LITTLE_ENDIAN

		#else // defined( LZAV_GCC_BUILTINS )

			#if defined( _MSC_VER )
				uint32_t i;
				_BitScanForward( &i, (unsigned long) vd );
				return p1 - p1s + ( i >> 3 );
			#else // defined( _MSC_VER )
				LZAV_IEC32( vd );
				const uint32_t m = 0x01010101;
				return p1 - p1s +
					(((( vd ^ ( vd - 1 )) & ( m - 1 )) * m ) >> 24 );
			#endif // defined( _MSC_VER )

		#endif // defined( LZAV_GCC_BUILTINS )
		}

		p1 += 4;
		p2 += 4;
	}

	// At most 3 bytes left.

	if( p1 < p1e )
	{
		if( *p1 != p2[ 0 ])
		{
			return p1 - p1s;
		}

		if( ++p1 < p1e )
		{
			if( *p1 != p2[ 1 ])
			{
				return p1 - p1s;
			}

			if( ++p1 < p1e )
			{
				if( *p1 != p2[ 2 ])
				{
					return p1 - p1s;
				}
			}
		}
	}

	return ml;
}

/**
 * @brief Data match length finding function, reverse direction.
 *
 * @param p1 Origin pointer to buffer 1.
 * @param p2 Origin pointer to buffer 2.
 * @param ml Maximal number of bytes to back-match.
 * @return The number of matching prior bytes, not including origin position.
 */

static inline size_t lzav_match_len_r( const uint8_t p1[], const uint8_t p2[],
	const size_t ml )
{
	if( LZAV_UNLIKELY( ml == 0 ))
	{
		return 0;
	}

	if( p1[ -1 ] != p2[ -1 ])
	{
		return 0;
	}

	if( LZAV_UNLIKELY( ml != 1 ))
	{
		const uint8_t* const p1s = p1;
		const uint8_t* p1e = p1 - ml + 1;
		--p1;
		--p2;

		while( LZAV_UNLIKELY( p1 > p1e ))
		{
			uint16_t v1, v2;
			lzav_memcpy( &v1, p1 - 2, 2 );
			lzav_memcpy( &v2, p2 - 2, 2 );

			const uint32_t vd = v1 ^ v2;

			if( vd != 0 )
			{
			#if LZAV_LITTLE_ENDIAN
				return p1s - p1 + (( vd & 0xFF00 ) == 0 );
			#else // LZAV_LITTLE_ENDIAN
				return p1s - p1 + (( vd & 0x00FF ) == 0 );
			#endif // LZAV_LITTLE_ENDIAN
			}

			p1 -= 2;
			p2 -= 2;
		}

		--p1e;

		if( p1 > p1e && p1[ -1 ] != p2[ -1 ])
		{
			return p1s - p1;
		}
	}

	return ml;
}

// Add an optimized reverse match length function using SIMD
static inline size_t lzav_match_len_opt_r(const uint8_t p1[], const uint8_t p2[], 
                                          const size_t ml) {
    // Early exit for small lengths
    if (LZAV_UNLIKELY(ml < 8)) {
        size_t len = 0;
        while (len < ml && p1[-len -1] == p2[-len -1]) {
            ++len;
        }
        return len;
    }

    // Assume alignment of p1, p2 to 16 bytes for faster SIMD loads
    const uint8_t* p1_aligned = lzav_assume_aligned(p1 - ml, 16);
    const uint8_t* p2_aligned = lzav_assume_aligned(p2 - ml, 16);

    // Prefetch distant cache lines
    LZAV_PREFETCH(p1_aligned - LZAV_PREFETCH_DIST);
    LZAV_PREFETCH(p2_aligned - LZAV_PREFETCH_DIST);

    const uint8_t* const p1s = p1;
    const uint8_t* const p1e = p1 - ml;

    // Align to 8-byte boundary for better memory access
    size_t misalign = ((uintptr_t)(p1 - 1)) & 7;
    if (misalign != 0) {
        misalign = 8 - misalign;
        while (misalign-- && p1 > p1e && p1[-1] == p2[-1]) {
            --p1;
            --p2;
        }
        if (p1 > p1e && p1[-1] != p2[-1]) {
            return p1s - p1;
        }
    }

    #if defined(__AVX512F__)
    // Process 64 bytes at a time with AVX-512
    while (LZAV_LIKELY(p1 - 63 >= p1e)) {
        __m512i v1 = _mm512_loadu_si512((__m512i*)(p1 - 64));
        __m512i v2 = _mm512_loadu_si512((__m512i*)(p2 - 64));
        uint64_t mask = _mm512_cmpeq_epi8_mask(v1, v2);
        if (mask != ~0ULL) {
            // Find the first differing byte
            return p1s - (p1 - (__builtin_ctzll(~mask)) + 64);
        }
        p1 -= 64;
        p2 -= 64;
    }
    #elif defined(__AVX2__)
    // Process 32 bytes at a time with AVX2
    while (LZAV_LIKELY(p1 - 31 >= p1e)) {
        __m256i v1 = _mm256_loadu_si256((__m256i*)(p1 - 32));
        __m256i v2 = _mm256_loadu_si256((__m256i*)(p2 - 32));
        uint32_t mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(v1, v2));
        if (mask != ~0U) {
            return p1s - (p1 - (__builtin_ctz(~mask)) + 32);
        }
        p1 -= 32;
        p2 -= 32;
    }
    #elif defined(__SSE2__)
    // Process 16 bytes at a time with SSE2
    while (LZAV_LIKELY(p1 - 15 >= p1e)) {
        __m128i v1 = _mm_loadu_si128((__m128i*)(p1 - 16));
        __m128i v2 = _mm_loadu_si128((__m128i*)(p2 - 16));
        uint16_t mask = _mm_movemask_epi8(_mm_cmpeq_epi8(v1, v2));
        if (mask != 0xFFFF) {
            // Find the first differing byte
            return p1s - (p1 - (__builtin_ctz(~mask)) + 16);
        }
        p1 -= 16;
        p2 -= 16;
    }
    #endif

    // Process remaining bytes with 64-bit operations
    while (LZAV_LIKELY(p1 - 7 >= p1e)) {
        uint64_t v1, v2;
        lzav_memcpy(&v1, p1 - 8, 8);
        lzav_memcpy(&v2, p2 - 8, 8);
        if (v1 != v2) {
            #if defined(__GNUC__) || defined(__clang__)
                return p1s - (p1 - (__builtin_ctzll(v1 ^ v2) >> 3) + 8);
            #else
                uint64_t diff = v1 ^ v2;
                unsigned int shift = 0;
                while ((diff & 0xFF) == 0) {
                    diff >>= 8;
                    shift += 8;
                }
                return p1s - (p1 - (shift >> 3) + 8);
            #endif
        }
        p1 -= 8;
        p2 -= 8;
    }

    // Handle remaining bytes (less than 8)
    while (p1 > p1e && p1[-1] == p2[-1]) {
        --p1;
        --p2;
    }

    return p1s - p1;
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

static inline uint8_t* lzav_write_blk_2( uint8_t* op, size_t lc, size_t rc,
	size_t d, const uint8_t* ipa, uint8_t** const cbpp, int* const cshp,
	const size_t mref )
{
	// Perform offset carry to a previous block (`csh` may be zero).

	const int csh = *cshp;
	rc = rc + 1 - mref;
	**cbpp |= (uint8_t) (( d << 8 ) >> csh );
	d >>= csh;

	if( LZAV_UNLIKELY( lc != 0 ))
	{
		// Write a literal block.

		size_t cv; // Offset carry value in literal block.
		cv = ( d & 3 ) << 6;
		d >>= 2;

		if( LZAV_LIKELY( lc < 9 ))
		{
			*op = (uint8_t) ( cv | lc );
			++op;

			lzav_memcpy( op, ipa, 8 );
			op += lc;
		}
		else
		if( LZAV_LIKELY( lc < 16 ))
		{
			*op = (uint8_t) ( cv | lc );
			++op;

			lzav_memcpy( op, ipa, 16 );
			op += lc;
		}
		else
		if( LZAV_LIKELY( lc < 16 + 128 ))
		{
		#if LZAV_LITTLE_ENDIAN
			uint16_t ov = (uint16_t) (( lc - 16 ) << 8 | cv );
		#else // LZAV_LITTLE_ENDIAN
			uint16_t ov = (uint16_t) ( cv << 8 | ( lc - 16 ));
		#endif // LZAV_LITTLE_ENDIAN

			lzav_memcpy( op, &ov, 2 );
			op += 2;

			lzav_memcpy( op, ipa, 16 );
			lzav_memcpy( op + 16, ipa + 16, 16 );

			if( lc < 33 )
			{
				op += lc;
			}
			else
			{
				ipa += 32;
				op += 32;
				lc -= 32;

				do
				{
					*op = *ipa;
					++ipa;
					++op;
				} while( --lc != 0 );
			}
		}
		else
		{
			*op = (uint8_t) cv;
			++op;

			size_t lcw = lc - 16;

			while( lcw > 127 )
			{
				*op = (uint8_t) ( 0x80 | lcw );
				lcw >>= 7;
				++op;
			}

			*op = (uint8_t) lcw;
			++op;

			lzav_memcpy( op, ipa, lc );
			op += lc;
		}
	}

	// Write a reference block.

	static const int ocsh[ 4 ] = { 0, 0, 0, 3 };
	const size_t bt = 1 + ( d > ( 1 << 10 ) - 1 ) + ( d > ( 1 << 18 ) - 1 );

	if( LZAV_LIKELY( rc < 16 ))
	{
		uint32_t ov = (uint32_t) ( d << 6 | bt << 4 | rc );
		LZAV_IEC32( ov );
		lzav_memcpy( op, &ov, 4 );

		op += bt;
		*cshp = ocsh[ bt ];
		*cbpp = op;

		return op + 1;
	}

	uint32_t ov = (uint32_t) ( d << 6 | bt << 4 );
	LZAV_IEC32( ov );
	lzav_memcpy( op, &ov, 4 );

	op += bt;
	*cshp = ocsh[ bt ];
	*cbpp = op;

	if( LZAV_LIKELY( rc < 16 + 255 ))
	{
		op[ 1 ] = (uint8_t) ( rc - 16 );
		return op + 2;
	}

	op[ 1 ] = (uint8_t) 255;
	op[ 2 ] = (uint8_t) ( rc - 16 - 255 );
	return op + 3;
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

static inline uint8_t* lzav_write_fin_2( uint8_t* op, size_t lc,
	const uint8_t* ipa )
{
	if( lc < 16 )
	{
		*op = (uint8_t) lc;
		++op;
	}
	else
	{
		*op = 0;
		++op;

		size_t lcw = lc - 16;

		while( lcw > 127 )
		{
			*op = (uint8_t) ( 0x80 | lcw );
			lcw >>= 7;
			++op;
		}

		*op = (uint8_t) lcw;
		++op;

	}

	lzav_memcpy( op, ipa, lc );
	return op + lc;
}

/**
 * @brief Function returns buffer size required for LZAV compression.
 *
 * @param srcl The length of the source data to be compressed.
 * @return The required allocation size for destination compression buffer.
 * Always a positive value.
 */

static inline int lzav_compress_bound(const int srcl)
{
    if (srcl <= 0) {
        return 16;
    }

    const int k = 16 + 127 + 1;
    const int l2 = srcl / (k + 6);

    return (srcl - l2 * 6 + k - 1) / k * 2 - l2 + srcl + 16;
}

/**
 * @brief Function returns buffer size required for the higher-ratio LZAV
 * compression.
 *
 * @param srcl The length of the source data to be compressed.
 * @return The required allocation size for destination compression buffer.
 * Always a positive value.
 */

static inline int lzav_compress_bound_hi(const int srcl)
{
    if (srcl <= 0) {
        return 16;
    }

    const int l2 = srcl / (16 + 5);

    return (srcl - l2 * 5 + 15) / 16 * 2 - l2 + srcl + 16;
}

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
static inline int lzav_tiny_compress(const void* const src, void* const dst,
    const int srcl, const int dstl) {
    if (srcl <= 0 || srcl > LZAV_TINY_MAX || dstl < srcl + 2) 
        return 0;
        
    uint8_t* op = (uint8_t*)dst;
    *op++ = LZAV_FMT_CUR << 4 | LZAV_REF_MIN; // Format byte
    *op++ = (uint8_t)srcl;                     // Length byte
    LZAV_MEMCPY_SMALL(op, src, srcl);          // Direct copy
    
    return srcl + 2;
}

// Optimize existing compress function
static inline int lzav_compress( const void* const src, void* const dst,
	const int srcl, const int dstl, void* const ext_buf, const int ext_bufl )
{
    // Fast path for tiny data
    if (srcl <= LZAV_TINY_MAX) {
        return lzav_tiny_compress(src, dst, srcl, dstl);
    }

	if(( srcl <= 0 ) | ( src == 0 ) | ( dst == 0 ) | ( src == dst ) |
		( dstl < lzav_compress_bound( srcl )))
	{
		return 0;
	}


    uint8_t* op = (uint8_t*)dst;
    *op = (uint8_t)(LZAV_FMT_CUR << 4 | LZAV_REF_MIN);  // prefix byte
    ++op;

    // Handle extremely small data
    if (srcl < 16) {
        *op++ = (uint8_t)srcl;
        LZAV_MEMCPY_SMALL(op, src, srcl);
        if (srcl > LZAV_LIT_FIN - 1) {
            return 2 + srcl;
        }
        lzav_memset(op + srcl, 0, LZAV_LIT_FIN - srcl);
        return 2 + LZAV_LIT_FIN;
    }

	uint32_t stack_buf[ 4096 ]; // On-stack hash-table.
	void* alloc_buf = 0; // Hash-table allocated on heap.
	uint8_t* ht = (uint8_t*) stack_buf; // The actual hash-table pointer.

	size_t htsize; // Hash-table's size in bytes (power-of-2).
    // Optimize hash table size for small data
    if (srcl <= LZAV_SMALL_MAX) {
        htsize = LZAV_MIN_HTABLE;
    } else {
        htsize = (1 << 7) * sizeof(uint32_t) * 4;
        while (htsize != (1 << 20) && (htsize >> 2) < (size_t)srcl) {
            htsize <<= 1;
        }
    }

	if( ext_buf == 0 )
	{
		while( htsize != ( 1 << 20 ) && ( htsize >> 2 ) < (size_t) srcl )
		{
			htsize <<= 1;
		}

		if( htsize > sizeof( stack_buf ))
		{
			alloc_buf = malloc( htsize );

			if( alloc_buf == 0 )
			{
				return 0;
			}

			ht = (uint8_t*) alloc_buf;
		}
	}
	else
	{
		size_t htsizem;

		if( ext_bufl > (int) sizeof( stack_buf ))
		{
			htsizem = (size_t) ext_bufl;
			ht = (uint8_t*) ext_buf;
		}
		else
		{
			htsizem = sizeof( stack_buf );
		}

		while(( htsize >> 2 ) < (size_t) srcl )
		{
			const size_t htsize2 = htsize << 1;

			if( htsize2 > htsizem )
			{
				break;
			}

			htsize = htsize2;
		}
	}

	const uint32_t hmask = (uint32_t) (( htsize - 1 ) ^ 15 ); // Hash mask.
	const uint8_t* ip = (const uint8_t*) src; // Source data pointer.
	const uint8_t* const ipe = ip + srcl - LZAV_LIT_FIN; // End pointer.
	const uint8_t* const ipet = ipe - 9; // Hashing threshold, avoids I/O OOB.
	const uint8_t* ipa = ip; // Literals anchor pointer.

	uint8_t* cbp = op; // Pointer to the latest offset carry block header.
	int csh = 0; // Offset carry shift.

	intptr_t mavg = 100 << 21; // Running average of hash match rate (*2^15).
		// Two-factor average: success (0-64) by average reference length.
	uint32_t rndb = 0; // PRNG bit derived from the non-matching offset.

	ip += 16; // Skip source bytes, to avoid OOB in back-match.

	// Initialize the hash-table. Each hash-table item consists of 2 tuples
	// (4 initial match bytes; 32-bit source data offset). Set source data
	// offset to avoid OOB in back-match.

	uint32_t initv[ 2 ] = { 0, 16 };

	if( LZAV_LIKELY( ip < ipet ))
	{
		lzav_memcpy( initv, ip, 4 );
	}

	uint32_t* ht32 = (uint32_t*) ht;
	uint32_t* const ht32e = (uint32_t*) ( ht + htsize );

	while( ht32 != ht32e )
	{
		ht32[ 0 ] = initv[ 0 ];
		ht32[ 1 ] = initv[ 1 ];
		ht32 += 2;
	}

	while( LZAV_LIKELY( ip < ipet ))
	{
		// Hash source data (endianness is unimportant for compression
		// efficiency). Hash is based on the "komihash" math construct, see
		// https://github.com/avaneev/komihash for details.

		uint32_t iw1;
		uint16_t iw2, ww2;
		lzav_memcpy( &iw1, ip, 4 );
		const uint32_t Seed1 = 0x243F6A88 ^ iw1;
		lzav_memcpy( &iw2, ip + 4, 2 );
		const uint64_t hm = (uint64_t) Seed1 * (uint32_t) ( 0x85A308D3 ^ iw2 );
		const uint32_t hval = (uint32_t) hm ^ (uint32_t) ( hm >> 32 );

		// Hash-table access.

		uint32_t* const hp = (uint32_t*) ( ht + ( hval & hmask ));
		const uint32_t ipo = (uint32_t) ( ip - (const uint8_t*) src );
		const uint32_t hw1 = hp[ 0 ]; // Tuple 1's match word.
		const uint8_t* wp; // At window pointer.
		size_t d, ml, rc, lc;

		// Find source data in hash-table tuples.

		if( LZAV_LIKELY( iw1 != hw1 ))
		{
			if( LZAV_LIKELY( iw1 != hp[ 2 ]))
			{
				goto _no_match;
			}

			wp = (const uint8_t*) src + hp[ 3 ];
			lzav_memcpy( &ww2, wp + 4, 2 );

			if( LZAV_UNLIKELY( iw2 != ww2 ))
			{
				goto _no_match;
			}
		}
		else
		{
			wp = (const uint8_t*) src + hp[ 1 ];
			lzav_memcpy( &ww2, wp + 4, 2 );

			if( LZAV_UNLIKELY( iw2 != ww2 ))
			{
				if( LZAV_LIKELY( iw1 != hp[ 2 ]))
				{
					goto _no_match;
				}

				wp = (const uint8_t*) src + hp[ 3 ];
				lzav_memcpy( &ww2, wp + 4, 2 );

				if( LZAV_UNLIKELY( iw2 != ww2 ))
				{
					goto _no_match;
				}
			}
		}

		d = ip - wp; // Reference offset (distance).

		if( LZAV_UNLIKELY(( d < 8 ) | ( d > LZAV_WIN_LEN - 1 )))
		{
			// Small offsets may be inefficient.

			goto _d_oob;
		}

		// Source data and hash-table entry match.

		// Disallow reference copy overlap by using `d` as max match length.

		ml = ( d > LZAV_REF_LEN ? LZAV_REF_LEN : d );

		if( LZAV_UNLIKELY( ip + ml > ipe ))
		{
			// Make sure `LZAV_LIT_FIN` literals remain on finish.

			ml = ipe - ip;
		}

		if( LZAV_LIKELY( d > 273 ))
		{
			// Update a matching entry which is not inside max reference
			// length's range. Otherwise, source data consisting of same-byte
			// runs won't compress well.

			if( LZAV_LIKELY( iw1 == hw1 )) // Replace tuple, or insert.
			{
				hp[ 1 ] = ipo;
			}
			else
			{
				hp[ 2 ] = hw1;
				hp[ 3 ] = hp[ 1 ];
				hp[ 0 ] = iw1;
				hp[ 1 ] = ipo;
			}
		}

		rc = LZAV_REF_MIN + lzav_match_len( ip + LZAV_REF_MIN,
			wp + LZAV_REF_MIN, ml - LZAV_REF_MIN );

		lc = ip - ipa;

		if( LZAV_UNLIKELY( lc != 0 ))
		{
			// Try to consume literals by finding a match at a back-position.

			ml -= rc;
			size_t bmc = ( lc > 16 ? 16 : lc );

			if( LZAV_LIKELY( ml > bmc ))
			{
				ml = bmc;
			}

			bmc = lzav_match_len_r( ip, wp, ml );

			if( LZAV_UNLIKELY( bmc != 0 ))
			{
				rc += bmc;
				ip -= bmc;
				lc -= bmc;
			}
		}

		op = lzav_write_blk_2( op, lc, rc, d, ipa, &cbp, &csh, LZAV_REF_MIN );
		ip += rc;
		ipa = ip;
		mavg += ( (intptr_t) ( rc << 21 ) - mavg ) >> 10;
		continue;

	_d_oob:
		++ip;

		if( LZAV_LIKELY( d < LZAV_WIN_LEN ))
		{
			continue;
		}

		hp[ 1 + ( iw1 != hw1 ) * 2 ] = ipo;
		continue;

	_no_match:
		hp[ 2 ] = iw1;
		hp[ 3 ] = ipo;

		mavg -= mavg >> 11;

		if( mavg < ( 200 << 14 ) && ip != ipa ) // Speed-up threshold.
		{
			// Compression speed-up technique that keeps the number of hash
			// evaluations around 45% of compressed data length. In some cases
			// reduces the number of blocks by several percent.

			ip += 1 + rndb; // Use PRNG bit to dither match positions.
			rndb = ipo & 1; // Delay to decorrelate from current match.

			if( LZAV_UNLIKELY( mavg < ( 130 << 14 )))
			{
				++ip;

				if( LZAV_UNLIKELY( mavg < ( 100 << 14 )))
				{
					ip += 100 - ( mavg >> 14 ); // Gradually faster.
				}
			}
		}

		++ip;
	}

	if( alloc_buf != 0 )
	{
		free( alloc_buf );
	}

	return (int) ( lzav_write_fin_2( op, ipe - ipa + LZAV_LIT_FIN, ipa ) -
		(uint8_t*) dst );
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

static inline int lzav_compress_default( const void* const src,
	void* const dst, const int srcl, const int dstl )
{
	return lzav_compress( src, dst, srcl, dstl, 0, 0 );
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

static inline int lzav_compress_hi( const void* const src, void* const dst,
	const int srcl, const int dstl )
{
	if(( srcl <= 0 ) | ( src == 0 ) | ( dst == 0 ) | ( src == dst ) |
		( dstl < lzav_compress_bound_hi( srcl )))
	{
		return 0;
	}

	const size_t mref = 5; // Minimal reference length.
	const size_t mlen = LZAV_REF_LEN - LZAV_REF_MIN + mref;

	uint8_t* op = (uint8_t*) dst; // Destination (compressed data) pointer.
	*op = (uint8_t) ( LZAV_FMT_CUR << 4 | mref ); // Write prefix byte.
	++op;

	if( srcl < 16 )
	{
		// Handle a very short source data.

		*op = (uint8_t) srcl;
		++op;

		lzav_memcpy( op, src, srcl );

		if( srcl > LZAV_LIT_FIN - 1 )
		{
			return 2 + srcl;
		}

		lzav_memset( op + srcl, 0, LZAV_LIT_FIN - srcl );
		return 2 + LZAV_LIT_FIN;
	}

	size_t htsize; // Hash-table's size in bytes (power-of-2).
	htsize = ( 1 << 7 ) * sizeof( uint32_t ) * 2 * 8;

	while( htsize != ( 1 << 23 ) && ( htsize >> 2 ) < (size_t) srcl )
	{
		htsize <<= 1;
	}

	uint8_t* ht = (uint8_t*) malloc( htsize ); // The hash-table pointer.

	if( ht == 0 )
	{
		return 0;
	}

	const uint32_t hmask = (uint32_t) (( htsize - 1 ) ^ 63 ); // Hash mask.
	const uint8_t* ip = (const uint8_t*) src; // Source data pointer.
	const uint8_t* const ipe = ip + srcl - LZAV_LIT_FIN; // End pointer.
	const uint8_t* const ipet = ipe - 9; // Hashing threshold, avoids I/O OOB.
	const uint8_t* ipa = ip; // Literals anchor pointer.

	uint8_t* cbp = op; // Pointer to the latest offset carry block header.
	int csh = 0; // Offset carry shift.

	// Initialize the hash-table. Each hash-table item consists of 8 tuples
	// (4 initial match bytes; 32-bit source data offset). The last value of
	// the last tuple is used as head tuple offset (an even value).

	uint32_t initv[ 2 ] = { 0, 0 };
	lzav_memcpy( initv, ip, 4 );

	uint32_t* ht32 = (uint32_t*) ht;
	uint32_t* const ht32e = (uint32_t*) ( ht + htsize );

	while( ht32 != ht32e )
	{
		ht32[ 0 ] = initv[ 0 ];
		ht32[ 1 ] = initv[ 1 ];
		ht32 += 2;
	}

	size_t prc = 0; // Length of a previously found match.
	size_t pd = 0; // Distance of a previously found match.
	const uint8_t* pip = ip; // Source pointer of a previously found match.

	while( LZAV_LIKELY( ip < ipet ))
	{
		// Hash source data (endianness is unimportant for compression
		// efficiency). Hash is based on the "komihash" math construct, see
		// https://github.com/avaneev/komihash for details.

		uint32_t iw1;
		lzav_memcpy( &iw1, ip, 4 );
		const uint64_t hm = (uint64_t) ( 0x243F6A88 ^ iw1 ) *
			(uint32_t) ( 0x85A308D3 ^ ip[ 4 ]);

		const uint32_t hval = (uint32_t) hm ^ (uint32_t) ( hm >> 32 );

		// Hash-table access.

		uint32_t* const hp = (uint32_t*) ( ht + ( hval & hmask ));
		const uint32_t ipo = (uint32_t) ( ip - (const uint8_t*) src );
		size_t ti0 = hp[ 15 ]; // Head tuple offset.

		// Find source data in hash-table tuples, in up to 7 previous
		// positions.

		const uint8_t* wp = ip; // Best found window pointer.
		size_t rc = 0; // Best found match length, 0 - not found.
		size_t d; // Reference offset (distance).
		size_t ti = ti0;
		int i1;

		if( LZAV_LIKELY( ip + mlen < ipe ))
		{
			// Optimized match-finding.

			for( i1 = 0; i1 < 7; ++i1 )
			{
				const uint32_t ww1 = hp[ ti ];
				const uint8_t* const wp0 = (const uint8_t*) src + hp[ ti + 1 ];
				d = ip - wp0;
				ti = ( ti == 12 ? 0 : ti + 2 );

				if( iw1 == ww1 )
				{
					const size_t rc0 = 4 + lzav_match_len( ip + 4, wp0 + 4,
						( d > mlen ? mlen : d ) - 4 );

					if( rc0 > rc + ( d > ( 1 << 18 )))
					{
						wp = wp0;
						rc = rc0;
					}
				}
			}
		}
		else
		{
			for( i1 = 0; i1 < 7; ++i1 )
			{
				const uint32_t ww1 = hp[ ti ];
				const uint8_t* const wp0 = (const uint8_t*) src + hp[ ti + 1 ];
				d = ip - wp0;
				ti = ( ti == 12 ? 0 : ti + 2 );

				if( iw1 == ww1 )
				{
					// Disallow reference copy overlap by using `d` as max
					// match length.

					size_t ml = ( d > mlen ? mlen : d );

					if( LZAV_UNLIKELY( ip + ml > ipe ))
					{
						// Make sure `LZAV_LIT_FIN` literals remain on finish.

						ml = ipe - ip;
					}

					const size_t rc0 = 4 + lzav_match_len( ip + 4, wp0 + 4,
						ml - 4 );

					if( rc0 > rc + ( d > ( 1 << 18 )))
					{
						wp = wp0;
						rc = rc0;
					}
				}
			}
		}

		if(( rc == 0 ) | ( d > 273 ))
		{
			// Update a matching entry which is not inside max reference
			// length's range. Otherwise, source data consisting of same-byte
			// runs won't compress well.

			ti0 = ( ti0 == 0 ? 12 : ti0 - 2 );
			hp[ ti0 ] = iw1;
			hp[ ti0 + 1 ] = ipo;
			hp[ 15 ] = (uint32_t) ti0;
		}

		if(( rc < mref + ( d > ( 1 << 18 ))) | ( d < 8 ) |
			( d > LZAV_WIN_LEN - 1 ))
		{
			++ip;
			continue;
		}

		// Source data and hash-table entry match of suitable length.

		const uint8_t* const ip0 = ip;
		size_t lc = ip - ipa;

		if( LZAV_UNLIKELY( lc != 0 ))
		{
			// Try to consume literals by finding a match at back-position.

			size_t ml = ( d > mlen ? mlen : d );

			if( LZAV_UNLIKELY( ip + ml > ipe ))
			{
				ml = ipe - ip;
			}

			ml -= rc;
			const size_t wpo = wp - (const uint8_t*) src;

			if( LZAV_LIKELY( ml > lc ))
			{
				ml = lc;
			}

			if( LZAV_UNLIKELY( ml > wpo ))
			{
				ml = wpo;
			}

			const size_t bmc = lzav_match_len_r( ip, wp, ml );

			if( LZAV_UNLIKELY( bmc != 0 ))
			{
				rc += bmc;
				ip -= bmc;
				lc -= bmc;
			}
		}

		if( prc == 0 )
		{
			// Save match for a later comparison.

			prc = rc;
			pd = d;
			pip = ip;
			ip = ip0 + 1;
			continue;
		}

		// Block size overhead estimation, and comparison with a previously
		// found match.

		const int lb = ( lc != 0 );
		const int sh0 = 10 + ( csh != 0 ) * 3;
		const int sh = sh0 + lb * 2;
		const size_t ov = lc + lb + ( lc > 15 ) + 2 +
			( d >= ( (size_t) 1 << sh )) +
			( d >= ( (size_t) 1 << ( sh + 8 )));

		const size_t plc = pip - ipa;
		const int plb = ( plc != 0 );
		const int psh = sh0 + plb * 2;
		const size_t pov = plc + plb + ( plc > 15 ) + 2 +
			( pd >= ( (size_t) 1 << psh )) +
			( pd >= ( (size_t) 1 << ( psh + 8 )));

		if( LZAV_LIKELY( prc * ov > rc * pov ))
		{
			if( LZAV_UNLIKELY( pip + prc <= ip ))
			{
				// A winning previous match does not overlap a current match.

				op = lzav_write_blk_2( op, plc, prc, pd, ipa, &cbp, &csh,
					mref );

				ipa = pip + prc;
				prc = rc;
				pd = d;
				pip = ip;
				++ip;
				continue;
			}

			rc = prc;
			d = pd;
			ip = pip;
			lc = plc;
		}

		op = lzav_write_blk_2( op, lc, rc, d, ipa, &cbp, &csh, mref );
		ip += rc;
		ipa = ip;
		prc = 0;
	}

	if( prc != 0 )
	{
		op = lzav_write_blk_2( op, pip - ipa, prc, pd, ipa, &cbp, &csh,
			mref );

		ipa = pip + prc;
	}

	free( ht );

	return (int) ( lzav_write_fin_2( op, ipe - ipa + LZAV_LIT_FIN, ipa ) -
		(uint8_t*) dst );
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

static inline int32_t lzav_decompress_2( const void* const src, void* const dst,
	const size_t srcl, const size_t dstl, size_t* const pwl )
{
	const uint8_t* ip = (const uint8_t*) src; // Compressed data pointer.
	const uint8_t* const ipe = ip + srcl; // Compressed data boundary pointer.
	const uint8_t* const ipet = ipe - 6; // Block header read threshold.
	uint8_t* op = (uint8_t*) dst; // Destination (decompressed data) pointer.
	uint8_t* const ope = op + dstl; // Destination boundary pointer.
	uint8_t* const opet = ope - 63; // Threshold for fast copy to destination.
	*pwl = dstl;
	const size_t mref1 = ( *ip & 15 ) - 1; // Minimal reference length - 1.
	size_t bh = 0; // Current block header, updated in each branch.
	size_t cv = 0; // Reference offset carry value.
	int32_t csh = 0; // Reference offset carry shift.

	#define LZAV_LOAD16( a ) \
		uint16_t bv; \
		lzav_memcpy( &bv, a, 2 ); \
		LZAV_IEC16( bv );

	#define LZAV_LOAD32( a ) \
		uint32_t bv; \
		lzav_memcpy( &bv, a, 4 ); \
		LZAV_IEC32( bv );

	#define LZAV_MEMMOVE( d, s, c ) \
		{ uint8_t tmp[ c ]; lzav_memcpy( tmp, s, c ); lzav_memcpy( d, tmp, c ); }

	#define LZAV_SET_IPD_CV( x, v, sh ) \
		const size_t d = ( x ) << csh | cv; \
		ipd = op - d; \
		if( LZAV_UNLIKELY( (uint8_t*) dst + d > op )) \
			goto _err_refoob; \
		csh = ( sh ); \
		cv = ( v );

	#define LZAV_SET_IPD( x ) \
		LZAV_SET_IPD_CV( x, 0, 0 )

	++ip; // Advance beyond prefix byte.

	if( LZAV_UNLIKELY( ip >= ipet ))
	{
		goto _err_srcoob;
	}

	bh = *ip;

	while( LZAV_LIKELY( ip < ipet ))
	{
		const uint8_t* ipd; // Source data pointer.
		size_t cc; // Byte copy count.
		size_t bt; // Block type.

		if( LZAV_UNLIKELY(( bh & 0x30 ) == 0 )) // Block type 0.
		{
			size_t ncv = bh >> 6;
			++ip;
			cc = bh & 15;

			if( LZAV_LIKELY( cc != 0 )) // True, if no additional length byte.
			{
				ipd = ip;
				ncv <<= csh;
				ip += cc;

				if( LZAV_LIKELY(( op < opet ) & ( ipd < ipe - 15 - 7 )))
				{
					cv |= ncv;
					csh += 2;
					bh = *ip;
					lzav_memcpy( op, ipd, 16 );
					op += cc;
					goto _refblk; // Reference block follows, if not EOS.
				}
			}
			else
			{
				size_t lcw = *ip;
				ncv <<= csh;
				++ip;
				cc = lcw & 0x7F;
				int sh = 7;

				while(( lcw & 0x80 ) != 0 )
				{
					lcw = *ip;
					++ip;
					cc |= ( lcw & 0x7F ) << sh;

					if( sh == 28 ) // No more than 4 additional bytes.
					{
						break;
					}

					sh += 7;
				}

				cc += 16;
				ipd = ip;
				ip += cc;

				if( LZAV_LIKELY(( op < opet ) & ( ipd < ipe - 63 - 16 )))
				{
					lzav_memcpy( op, ipd, 16 );
					lzav_memcpy( op + 16, ipd + 16, 16 );
					lzav_memcpy( op + 32, ipd + 32, 16 );
					lzav_memcpy( op + 48, ipd + 48, 16 );

					if( LZAV_LIKELY( cc < 65 ))
					{
						cv |= ncv;
						csh += 2;
						bh = *ip;
						op += cc;
						goto _refblk; // Reference block follows, if not EOS.
					}

					ipd += 64;
					op += 64;
					cc -= 64;
				}
			}

			cv |= ncv;
			csh += 2;

			if( LZAV_LIKELY( ip < ipe ))
			{
				bh = *ip;
			}
			else
			if( LZAV_UNLIKELY( ip != ipe ))
			{
				goto _err_srcoob_lit;
			}

			if( LZAV_UNLIKELY( op + cc > ope ))
			{
				goto _err_dstoob_lit;
			}

			// This and other alike copy-blocks are transformed into fast SIMD
			// instructions, by a modern compiler. Direct use of `lzav_memcpy` is
			// slower due to shortness of data remaining to copy, on average.

			while( cc != 0 )
			{
				*op = *ipd;
				++ipd;
				++op;
				--cc;
			}

			continue;

		_err_srcoob_lit:
			cc = ipe - ipd;

			if( op + cc < ope )
			{
				lzav_memcpy( op, ipd, cc );
				*pwl = (int32_t) ( op + cc - (uint8_t*) dst );
			}
			else
			{
				lzav_memcpy( op, ipd, ope - op );
			}

			return LZAV_E_SRCOOB;

		_err_dstoob_lit:
			lzav_memcpy( op, ipd, ope - op );
			return LZAV_E_DSTOOB;
		}

	_refblk:
		bt = ( bh >> 4 ) & 3;
		++ip;
		const int bt8 = (int) ( bt << 3 );

		LZAV_LOAD32( ip );
		const uint32_t om = (uint32_t) (( 1 << bt8 ) - 1 );
		ip += bt;
		const size_t o = bv & om;
		bv >>= bt8;

		static const int ocsh[ 4 ] = { 0, 0, 0, 3 };
		const int wcsh = ocsh[ bt ];

		LZAV_SET_IPD_CV( bh >> 6 | ( o & 0x1FFFFF ) << 2, o >> 21, wcsh );

		cc = bh & 15;

		if( LZAV_LIKELY( cc != 0 )) // True, if no additional length byte.
		{
			bh = bv & 0xFF;
			cc += mref1;

			if( LZAV_LIKELY( op < opet ))
			{
				LZAV_MEMMOVE( op, ipd, 16 );
				LZAV_MEMMOVE( op + 16, ipd + 16, 4 );

				op += cc;
				continue;
			}
		}
		else
		{
			bh = bv & 0xFF;

			if( LZAV_UNLIKELY( bh == 255 ))
			{
				cc = 16 + mref1 + 255 + ip[ 1 ];
				bh = ip[ 2 ];
				ip += 2;
			}
			else
			{
				cc = 16 + mref1 + bh;
				++ip;
				bh = *ip;
			}

			if( LZAV_LIKELY( op < opet ))
			{
				LZAV_MEMMOVE( op, ipd, 16 );
				LZAV_MEMMOVE( op + 16, ipd + 16, 16 );
				LZAV_MEMMOVE( op + 32, ipd + 32, 16 );
				LZAV_MEMMOVE( op + 48, ipd + 48, 16 );

				if( LZAV_LIKELY( cc < 65 ))
				{
					op += cc;
					continue;
				}

				ipd += 64;
				op += 64;
				cc -= 64;
			}
		}

		if( LZAV_UNLIKELY( op + cc > ope ))
		{
			goto _err_dstoob_ref;
		}

		while( cc != 0 )
		{
			*op = *ipd;
			++ipd;
			++op;
			--cc;
		}

		continue;

	_err_dstoob_ref:
		lzav_memmove( op, ipd, ope - op );
		return LZAV_E_DSTOOB;
	}

	if( LZAV_UNLIKELY( op != ope ))
	{
		goto _err_dstlen;
	}

	return (int32_t) ( op - (uint8_t*) dst );

_err_srcoob:
	*pwl = (int32_t) ( op - (uint8_t*) dst );
	return LZAV_E_SRCOOB;

_err_refoob:
	*pwl = (int32_t) ( op - (uint8_t*) dst );
	return LZAV_E_REFOOB;

_err_dstlen:
	*pwl = (int32_t) ( op - (uint8_t*) dst );
	return LZAV_E_DSTLEN;
}

#if LZAV_FMT_MIN < 2

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

static inline int32_t lzav_decompress_1( const void* const src, void* const dst,
	const size_t srcl, const size_t dstl )
{
	const uint8_t* ip = (const uint8_t*) src; // Compressed data pointer.
	const uint8_t* const ipe = ip + srcl; // Compressed data boundary pointer.
	const uint8_t* const ipet = ipe - 5; // Block header read threshold.
	uint8_t* op = (uint8_t*) dst; // Destination (decompressed data) pointer.
	uint8_t* const ope = op + dstl; // Destination boundary pointer.
	uint8_t* const opet = ope - 63; // Threshold for fast copy to destination.
	const size_t mref1 = ( *ip & 15 ) - 1; // Minimal reference length - 1.
	size_t bh = 0; // Current block header, updated in each branch.
	size_t cv = 0; // Reference offset carry value.
	int32_t csh = 0; // Reference offset carry shift.

	++ip; // Advance beyond prefix byte.

	if( LZAV_UNLIKELY( ip >= ipet ))
	{
		goto _err_srcoob;
	}

	bh = *ip;

	while( LZAV_LIKELY( ip < ipet ))
	{
		const uint8_t* ipd; // Source data pointer.
		size_t cc; // Byte copy count.

		if( LZAV_UNLIKELY(( bh & 0x30 ) == 0 )) // Block type 0.
		{
			cv = bh >> 6;
			csh = 2;
			++ip;
			cc = bh & 15;

			if( LZAV_LIKELY( cc != 0 )) // True, if no additional length byte.
			{
				ipd = ip;
				ip += cc;

				if( LZAV_LIKELY(( op < opet ) & ( ipd < ipe - 15 - 6 )))
				{
					bh = *ip;
					lzav_memcpy( op, ipd, 16 );
					op += cc;
					goto _refblk; // Reference block follows, if not EOS.
				}
			}
			else
			{
				LZAV_LOAD16( ip );

				const int l2 = bv & 0xFF;
				cc = 16;
				++ip;
				const int lb = ( l2 == 255 );
				cc += l2 + (( bv >> 8 ) & ( 0x100 - lb ));
				ip += lb;

				ipd = ip;
				ip += cc;

				if( LZAV_LIKELY(( op < opet ) & ( ipd < ipe - 63 - 1 )))
				{
					lzav_memcpy( op, ipd, 16 );
					lzav_memcpy( op + 16, ipd + 16, 16 );
					lzav_memcpy( op + 32, ipd + 32, 16 );
					lzav_memcpy( op + 48, ipd + 48, 16 );

					if( LZAV_LIKELY( cc < 65 ))
					{
						bh = *ip;
						op += cc;
						continue;
					}

					ipd += 64;
					op += 64;
					cc -= 64;
				}
			}

			if( LZAV_LIKELY( ip < ipe ))
			{
				bh = *ip;
			}
			else
			if( LZAV_UNLIKELY( ip != ipe ))
			{
				goto _err_srcoob;
			}

			if( LZAV_UNLIKELY( op + cc > ope ))
			{
				goto _err_dstoob;
			}

			// This and other alike copy-blocks are transformed into fast SIMD
			// instructions, by a modern compiler. Direct use of `lzav_memcpy` is
			// slower due to shortness of data remaining to copy, on average.

			while( cc != 0 )
			{
				*op = *ipd;
				++ipd;
				++op;
				--cc;
			}

			continue;
		}

	_refblk:
		cc = bh & 15;

		if( LZAV_UNLIKELY(( bh & 32 ) == 0 )) // True, if block type 1.
		{
			LZAV_SET_IPD( bh >> 6 | (size_t) ip[ 1 ] << 2 );
			ip += 2;
			bh = *ip;
		}
		else // Block type 2 or 3.
		{
			if( LZAV_LIKELY(( bh & 16 ) == 0 )) // True, if block type 2.
			{
				LZAV_LOAD16( ip + 1 );
				LZAV_SET_IPD( bh >> 6 | (size_t) bv << 2 );
				ip += 3;
				bh = *ip;
			}
			else // Block type 3.
			{
				LZAV_LOAD32( ip + 1 );
				LZAV_SET_IPD_CV( bv & 0xFFFFFF, bh >> 6, 2 );
				ip += 4;
				bh = bv >> 24;
			}
		}

		if( LZAV_LIKELY( cc != 0 )) // True, if no additional length byte.
		{
			cc += mref1;

			if( LZAV_LIKELY( op < opet ))
			{
				LZAV_MEMMOVE( op, ipd, 16 );
				LZAV_MEMMOVE( op + 16, ipd + 16, 4 );

				op += cc;
				continue;
			}
		}
		else
		{
			cc = 16 + mref1 + bh;
			++ip;
			bh = *ip;

			if( LZAV_LIKELY( op < opet ))
			{
				LZAV_MEMMOVE( op, ipd, 16 );
				LZAV_MEMMOVE( op + 16, ipd + 16, 16 );
				LZAV_MEMMOVE( op + 32, ipd + 32, 16 );
				LZAV_MEMMOVE( op + 48, ipd + 48, 16 );

				if( LZAV_LIKELY( cc < 65 ))
				{
					op += cc;
					continue;
				}

				ipd += 64;
				op += 64;
				cc -= 64;
			}
		}

		if( LZAV_UNLIKELY( op + cc > ope ))
		{
			goto _err_dstoob;
		}

		while( cc != 0 )
		{
			*op = *ipd;
			++ipd;
			++op;
			--cc;
		}
	}

	if( LZAV_UNLIKELY( op != ope ))
	{
		goto _err_dstlen;
	}

	return (int32_t) ( op - (uint8_t*) dst );

_err_srcoob:
	return LZAV_E_SRCOOB;

_err_dstoob:
	return LZAV_E_DSTOOB;

_err_refoob:
	return LZAV_E_REFOOB;

_err_dstlen:
	return LZAV_E_DSTLEN;
}

#endif // LZAV_FMT_MIN < 2

#undef LZAV_LOAD16
#undef LZAV_LOAD32
#undef LZAV_MEMMOVE
#undef LZAV_SET_IPD_CV
#undef LZAV_SET_IPD

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

static inline int32_t lzav_decompress_partial( const void* const src,
	void* const dst, size_t srcl, size_t dstl )
{
	if( src == 0 || dst == 0 || src == dst )
	{
		return 0;
	}

	const int fmt = *(const uint8_t*) src >> 4;
	size_t dl = 0;

	if( fmt == 2 )
	{
		lzav_decompress_2( src, dst, srcl, dstl, &dl );
	}

	return dl;
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
static inline int32_t lzav_decompress(const void* const src, void* const dst,
    size_t srcl, size_t dstl)
{
    if (src == 0 || dst == 0 || src == dst)
    {
        return LZAV_E_PARAMS;
    }

    const int fmt = *(const uint8_t*)src >> 4;

    // Fast path for tiny data
    const uint8_t* ip = (const uint8_t*)src;
    if (srcl <= LZAV_TINY_MAX + 2 && ip[0] >> 4 == LZAV_FMT_CUR) {
        const int len = ip[1];
        if (len <= LZAV_TINY_MAX && len <= dstl) {
            lzav_memcpy(dst, ip + 2, len);
            return len;
        }
    }

    if (fmt == 2) {
        size_t tmp;
        return lzav_decompress_2(src, dst, srcl, dstl, &tmp);
    }

#if LZAV_FMT_MIN < 2
    if (fmt == 1) {
        return lzav_decompress_1(src, dst, srcl, dstl);
    }
#endif // LZAV_FMT_MIN < 2

    return LZAV_E_UNKFMT;
}

static inline void lzav_pool_free(lzav_pool_t* pool) {
    for (size_t i = 0; i < pool->count; ++i) {
        free(pool->blocks[i]);
    }
    pool->count = 0;
}

static inline uint32_t lzav_bitwise_rotate_left(uint32_t val, int shift) {
	// Rotate left using shifts and bitwise OR
	return (val << shift) | (val >> (32 - shift));
}

static inline uint32_t lzav_bitwise_xor(uint32_t a, uint32_t b) {
	// Simple XOR operation
	return a ^ b;
}

#endif // LZAV_INCLUDED
