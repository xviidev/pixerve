fn main() {
    println!("cargo:rerun-if-env-changed=VIPS_LIB_DIR");
    if let Ok(dir) = std::env::var("VIPS_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=dylib=libglib-2.0");
        println!("cargo:rustc-link-lib=dylib=libgobject-2.0");
    }
}
