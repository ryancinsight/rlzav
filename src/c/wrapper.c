#include "lzav.h"

// Export compression functions
int c_lzav_compress_default(const void* src, void* dst, int srcl, int dstl) {
    return lzav_compress_default(src, dst, srcl, dstl);
}

int c_lzav_compress_bound(int srcl) {
    return lzav_compress_bound(srcl);
}

int c_lzav_decompress(const void* src, void* dst, int srcl, int dstl) {
    return lzav_decompress(src, dst, srcl, dstl);
}

int c_lzav_decompress_partial(const void* src, void* dst, int srcl, int dstl) {
    return lzav_decompress_partial(src, dst, srcl, dstl);
}

// Constants getters
int c_get_lzav_e_params() { return LZAV_E_PARAMS; }
int c_get_lzav_e_srcoob() { return LZAV_E_SRCOOB; }
int c_get_lzav_e_dstoob() { return LZAV_E_DSTOOB; }
int c_get_lzav_e_refoob() { return LZAV_E_REFOOB; }
int c_get_lzav_e_dstlen() { return LZAV_E_DSTLEN; }
int c_get_lzav_e_unkfmt() { return LZAV_E_UNKFMT; }
