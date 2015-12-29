use std::env;
use std::fs::File;
use std::io::{Write, BufWriter};
use std::path::Path;

fn main() {
    //TODO: Buildscript for config stuff (...)
    println!("cargo:rustc-link-lib=dylib=php7_debug"); //TODO: Determine name
    println!("cargo:rustc-link-search={}", "..\\..\\php7\\x64\\Debug"); //TODO

    let path = env::var_os("OUT_DIR").unwrap();
    let path: &Path = path.as_ref();
    let path = path.join("test_helper.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    write!(file, "pub static PHP_PATH: &'static str = \"{}\";", "../../php7/x64/Debug/php.exe").unwrap(); //TODO
}
