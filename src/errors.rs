// Error codes shared between C and Rust implementations
pub const LZAV_E_PARAMS: i32 = -1;
pub const LZAV_E_SRCOOB: i32 = -2;
pub const LZAV_E_DSTOOB: i32 = -3;
pub const LZAV_E_REFOOB: i32 = -4;
pub const LZAV_E_DSTLEN: i32 = -5;
pub const LZAV_E_UNKFMT: i32 = -6;

// Ensure no macro_rules! redefinitions exist here

#[cfg(feature = "c-backend")]
use std::sync::OnceLock;

#[cfg(feature = "c-backend")]
pub static LZAV_ERR_CODES: [OnceLock<i32>; 6] = [
    OnceLock::new(), // PARAMS
    OnceLock::new(), // SRCOOB 
    OnceLock::new(), // DSTOOB
    OnceLock::new(), // REFOOB
    OnceLock::new(), // DSTLEN
    OnceLock::new()  // UNKFMT
];
