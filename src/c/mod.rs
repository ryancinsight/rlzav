use std::os::raw::{c_int, c_void};

extern "C" {
    pub fn c_lzav_compress_default(
        src: *const c_void,
        dst: *mut c_void,
        srcl: c_int,
        dstl: c_int,
    ) -> c_int;

    pub fn c_lzav_compress_bound(srcl: c_int) -> c_int;

    pub fn c_lzav_decompress(
        src: *const c_void,
        dst: *mut c_void,
        srcl: c_int,
        dstl: c_int,
    ) -> c_int;

    pub fn c_lzav_decompress_partial(
        src: *const c_void,
        dst: *mut c_void,
        srcl: c_int,
        dstl: c_int,
    ) -> c_int;

    // Error code constants
    pub fn c_get_lzav_e_params() -> c_int;
    pub fn c_get_lzav_e_srcoob() -> c_int;
    pub fn c_get_lzav_e_dstoob() -> c_int;
    pub fn c_get_lzav_e_refoob() -> c_int;
    pub fn c_get_lzav_e_dstlen() -> c_int;
    pub fn c_get_lzav_e_unkfmt() -> c_int;
}

// Safe Rust wrappers
pub fn compress_default(src: &[u8], dst: &mut [u8]) -> i32 {
    unsafe {
        c_lzav_compress_default(
            src.as_ptr() as *const _,
            dst.as_mut_ptr() as *mut _,
            src.len() as i32,
            dst.len() as i32,
        )
    }
}

pub fn compress_bound(srcl: i32) -> i32 {
    unsafe { c_lzav_compress_bound(srcl) }
}

pub fn decompress(src: &[u8], dst: &mut [u8]) -> i32 {
    unsafe {
        c_lzav_decompress(
            src.as_ptr() as *const _,
            dst.as_mut_ptr() as *mut _,
            src.len() as i32,
            dst.len() as i32,
        )
    }
}

pub fn decompress_partial(src: &[u8], dst: &mut [u8]) -> i32 {
    unsafe {
        c_lzav_decompress_partial(
            src.as_ptr() as *const _,
            dst.as_mut_ptr() as *mut _,
            src.len() as i32,
            dst.len() as i32,
        )
    }
}
