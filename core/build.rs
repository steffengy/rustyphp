fn main() {
    //TODO: Buildscript for config stuff (...)
    println!("cargo:rustc-link-lib=dylib=php7_debug"); //TODO: Determine name
    println!("cargo:rustc-link-search={}", "..\\..\\php7\\x64\\Debug"); //TODO
}
