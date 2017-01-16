extern crate auxv;

/// Show the auxv entries for this process
fn main() {
    unsafe {
        for pair in auxv::iterate_auxv() {
            println!("{}\t{}", pair.key, pair.value);
        }
    }
}
