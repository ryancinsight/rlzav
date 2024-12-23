pub const LZAV_API_VER: u32 = 0x106; // API version, unrelated to code's version.
pub const LZAV_VER_STR: &str = "4.5"; // LZAV source code version string.

pub const LZAV_E_PARAMS: i32 = -1; // Incorrect function parameters.
pub const LZAV_E_SRCOOB: i32 = -2; // Source buffer OOB.
pub const LZAV_E_DSTOOB: i32 = -3; // Destination buffer OOB.
pub const LZAV_E_REFOOB: i32 = -4; // Back-reference OOB.
pub const LZAV_E_DSTLEN: i32 = -5; // Decompressed length mismatch.
pub const LZAV_E_UNKFMT: i32 = -6; // Unknown stream format.

pub const LZAV_WIN_LEN: usize = 1 << 23; // LZ77 window length, in bytes.
pub const LZAV_REF_MIN: usize = 6; // Min reference length, in bytes.
pub const LZAV_REF_LEN: usize = LZAV_REF_MIN + 15 + 255 + 254; // Max ref length.
pub const LZAV_LIT_FIN: usize = 6; // The number of literals required at finish.
pub const LZAV_FMT_CUR: u8 = 2; // Stream format identifier used by the compressor.
pub const LZAV_FMT_MIN: u8 = 1;

// Add endianness detection
#[cfg(target_endian = "little")]
pub const LZAV_LITTLE_ENDIAN: bool = true;

#[cfg(target_endian = "big")]
pub const LZAV_LITTLE_ENDIAN: bool = false;

// Add architecture detection
#[cfg(target_pointer_width = "64")]
pub const LZAV_ARCH64: bool = true;

#[cfg(target_pointer_width = "32")]
pub const LZAV_ARCH64: bool = false;

// Add safety limits
pub const LZAV_MAX_SIZE: usize = 1 << 30; // 1GB max size
pub const LZAV_MAX_ITERATIONS: usize = 1_000_000;
pub const LZAV_MIN_COMPRESS_SIZE: usize = 16;