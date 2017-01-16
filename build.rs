extern crate gcc;

use std::env;
fn main() {
    if env::var("TARGET").unwrap().contains("linux") {
        gcc::compile_library("libgetauxval-wrapper.a", &["c/getauxval-wrapper.c"]);
    }

}
