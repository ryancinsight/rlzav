use crate::constants::*;

#[inline(always)]
pub(crate) fn lzav_match_len(p1: &[u8], p2: &[u8], ml: usize) -> usize {
    let mut pos = 0;

    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::*;
        while pos + 8 <= ml && pos + 8 <= p1.len() && pos + 8 <= p2.len() {
            // Safe SIMD operations using checked slices
            let v1 = unsafe { _mm_loadu_si64(p1[pos..].as_ptr() as *const _) };
            let v2 = unsafe { _mm_loadu_si64(p2[pos..].as_ptr() as *const _) };
            let mask = unsafe { _mm_movemask_epi8(_mm_cmpeq_epi8(v1, v2)) };
            if mask != 0xFFFF {
                return pos + (mask.trailing_zeros() as usize >> 3);
            }
            pos += 8;
        }
    }

    // Fallback/remaining bytes using safe slice operations
    while pos < ml && pos < p1.len() && pos < p2.len() && p1[pos] == p2[pos] {
        pos += 1;
    }
    pos
}

#[inline(always)]
pub(crate) fn lzav_match_len_r(p1: &[u8], p2: &[u8], ml: usize) -> usize {
    if ml == 0 || p1[0] != p2[0] {
        return 0;
    }

    let mut pos = 1;
    while pos < ml && p1[pos] == p2[pos] {
        pos += 1;
    }
    pos
}

#[repr(align(32))]
pub(crate) struct AlignedBuffer {
    data: Vec<u8>,
}

impl AlignedBuffer {
    pub fn new(size: usize) -> Self {
        let mut vec = Vec::with_capacity(size);
        vec.resize(size, 0);
        Self { data: vec }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

// Platform-specific optimizations
#[cfg(target_arch = "x86_64")]
pub(crate) mod arch {
    use std::arch::x86_64::*;

    #[inline(always)]
    pub fn copy_block(dst: &mut [u8], src: &[u8], len: usize) -> Option<()> {
        let len = len.min(dst.len()).min(src.len());
        if len >= 32 {
            let mut offset = 0;
            while offset + 32 <= len {
                // Safe SIMD copy with bounds checking
                unsafe {
                    _mm256_storeu_si256(
                        dst[offset..].as_mut_ptr() as *mut __m256i,
                        _mm256_loadu_si256(src[offset..].as_ptr() as *const __m256i)
                    );
                }
                offset += 32;
            }
            if offset < len {
                dst[offset..len].copy_from_slice(&src[offset..len]);
            }
        } else {
            dst[..len].copy_from_slice(&src[..len]);
        }
        Some(())
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub(crate) mod arch {
    #[inline(always)]
    pub fn copy_block(dst: &mut [u8], src: &[u8], len: usize) -> Option<()> {
        let len = len.min(dst.len()).min(src.len());
        dst[..len].copy_from_slice(&src[..len]);
        Some(())
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
mod avx2 {
    use std::arch::x86_64::*;
    
    #[inline(always)]
    pub unsafe fn match_len_simd(p1: &[u8], p2: &[u8], ml: usize) -> usize {
        let p1s = p1.as_ptr();
        let p1e = p1s.add(ml);
        let mut p1p = p1s;

        while p1p.add(31) < p1e {
            let v1 = _mm256_loadu_si256(p1p as *const __m256i);
            let v2 = _mm256_loadu_si256(p2.as_ptr().add(p1p.offset_from(p1s) as usize) as *const __m256i);
            let vd = _mm256_xor_si256(v1, v2);
            
            if _mm256_testz_si256(vd, vd) == 0 {
                #[cfg(target_feature = "bmi1")]
                return p1p.offset_from(p1s) as usize + (_tzcnt_u32(_mm256_movemask_epi8(vd) as u32) as usize);
                
                #[cfg(not(target_feature = "bmi1"))]
                {
                    let mask = _mm256_movemask_epi8(vd) as u32;
                    return p1p.offset_from(p1s) as usize + mask.trailing_zeros() as usize;
                }
            }
            p1p = p1p.add(32);
        }
        ml
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "bmi1"))]
mod bmi {
    use std::arch::x86_64::*;
    
    #[inline(always)]
    pub unsafe fn count_trailing_zeros(value: u64) -> u32 {
        _tzcnt_u64(value)
    }
}

#[cfg(not(all(target_arch = "x86_64", target_feature = "bmi1")))]
mod bmi {
    #[inline(always)]
    pub unsafe fn count_trailing_zeros(value: u64) -> u32 {
        value.trailing_zeros()
    }
}
