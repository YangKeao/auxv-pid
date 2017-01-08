extern crate gcc;
extern crate libc;

use std::env;
fn main() {
    if env::var("TARGET").unwrap().contains("linux") {
        gcc::compile_library("libgetauxval-wrapper.a", &["c/getauxval-wrapper.c"]);
    }

    let ulong_width = std::mem::size_of::<libc::c_ulong>();

    println!("cargo:rustc-cfg=autodetect_c_ulong_{}", ulong_width * 8);
}
