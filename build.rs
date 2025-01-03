fn main() {
    // Only compile C code if c-backend feature is enabled
    #[cfg(feature = "c-backend")]
    {
        println!("cargo:rerun-if-changed=src/c/wrapper.c");
        println!("cargo:rerun-if-changed=src/c/lzav.h");
        cc::Build::new()
            .file("src/c/wrapper.c")
            .include("src/c")
            .compile("lzav");
    }

    // When using rust-backend, no C compilation needed
    #[cfg(not(feature = "c-backend"))]
    {
        // Nothing to do for Rust-only build
    }
}
