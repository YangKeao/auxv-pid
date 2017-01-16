extern crate auxv;

fn main() {
    match auxv::search_procfs_auxv(&[auxv::AT_HWCAP]) {
        Ok(map) => {
            match map.get(&auxv::AT_HWCAP) {
                Some(v) => println!("Got HWCAP 0x{:016X}", v),
                None => println!("No HWCAP")
            }
        }
        Err(e) => println!("Could not search procfs auxv {:?}", e)
    }
}
