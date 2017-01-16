extern crate auxv;

/// Demonstrate that adding to the environment breaks stack crawling for auxv
fn main() {
    #[cfg(not(target_os = "windows"))]
    unsafe {
        println!("Auxv before modifying environ:");
        for pair in auxv::stack::iterate_stack_auxv() {
            println!("{}\t{}", pair.key, pair.value);
        };

        println!("Setting new env var, which sets environ to new array");
        std::env::set_var("QWERTY12345", "ASDF");

        println!("Auxv after");
        for pair in auxv::stack::iterate_stack_auxv() {
            println!("{}\t{}", pair.key, pair.value);
        };
    };
}
