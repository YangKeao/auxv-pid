extern crate auxv;

fn main() {
    match auxv::iterate_procfs_auxv() {
        Ok(iter) => {
            for pair_res in iter {
                match pair_res {
                    Ok(pair) => println!("{}\t{}", pair.key, pair.value),
                    Err(e) => println!("Error {:?}", e)
                }
            }
        }
        Err(e) => println!("Could not open procfs auxv {:?}", e)
    }
}
