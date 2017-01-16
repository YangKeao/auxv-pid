extern crate auxv;

fn main() {
    #[cfg(not(target_os="windows"))]
    unsafe {
        match auxv::iterate_stack_auxv().filter(|p| p.key == auxv::AT_HWCAP).next() {
            Some(p) => println!("Got HWCAP 0x{:016X}", p.value),
            None => println!("No HWCAP")
        }
    }
}
