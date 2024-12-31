#![allow(non_snake_case)]

// Version constants
pub const LZAV_API_VER: u32 = 0x106;
pub const LZAV_VER_STR: &str = "4.5";

// Error codes
pub const LZAV_E_PARAMS: i32 = -1; 
pub const LZAV_E_SRCOOB: i32 = -2;
pub const LZAV_E_DSTOOB: i32 = -3;
pub const LZAV_E_REFOOB: i32 = -4;
pub const LZAV_E_DSTLEN: i32 = -5;
pub const LZAV_E_UNKFMT: i32 = -6;

// Algorithm constants
pub const LZAV_WIN_LEN: usize = 1 << 23;
pub const LZAV_REF_MIN: usize = 6;
pub const LZAV_REF_LEN: usize = LZAV_REF_MIN + 15 + 255 + 254;
pub const LZAV_LIT_FIN: usize = 6;
pub const LZAV_FMT_CUR: u8 = 2;
pub const LZAV_FMT_MIN: u8 = 1;

// Platform-specific optimizations
#[cfg(target_endian = "little")]
pub const LZAV_LITTLE_ENDIAN: bool = true;

#[cfg(target_endian = "big")]
pub const LZAV_LITTLE_ENDIAN: bool = false;

#[cfg(target_pointer_width = "64")]
pub const LZAV_ARCH64: bool = true;

#[cfg(target_pointer_width = "32")]
pub const LZAV_ARCH64: bool = false;

// Additional constants for Rust implementation
pub const LZAV_MIN_COMPRESS_SIZE: usize = 16;

// Endianness correction functions
#[cfg(target_endian = "big")]
#[inline(always)]
pub fn swap16(x: u16) -> u16 { x.swap_bytes() }

#[cfg(target_endian = "little")]
#[inline(always)]
pub fn swap16(x: u16) -> u16 { x }

#[cfg(target_endian = "big")]
#[inline(always)]
pub fn swap32(x: u32) -> u32 { x.swap_bytes() }

#[cfg(target_endian = "little")]
#[inline(always)]
pub fn swap32(x: u32) -> u32 { x }

#[cfg(target_endian = "big")]
#[inline(always)]
pub fn swap64(x: u64) -> u64 { x.swap_bytes() }

#[cfg(target_endian = "little")]
#[inline(always)]
pub fn swap64(x: u64) -> u64 { x }
