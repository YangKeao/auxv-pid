extern crate auxv;

/// Show the auxv entries for this process
fn main() {
    #[cfg(not(target_os="windows"))]
    unsafe {
        for pair in auxv::iterate_stack_auxv() {
            println!("{}\t{}", pair.key, pair.value);
        }
    }
}
