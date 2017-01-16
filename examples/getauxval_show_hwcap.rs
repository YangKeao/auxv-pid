extern crate auxv;

use auxv::AT_HWCAP;
use auxv::getauxval::Getauxval;
#[cfg(target_os="linux")]
use auxv::getauxval::NativeGetauxval;
#[cfg(not(target_os="linux"))]
use auxv::getauxval::NotAvailableGetauxval;

fn main() {
   show_auxv(); 
}

#[cfg(target_os="linux")]
fn show_auxv() {
    let getauxval = NativeGetauxval {};
    print_hwcap(getauxval);
}

#[cfg(not(target_os="linux"))]
fn show_auxv() {
    let getauxval = NotAvailableGetauxval {};
    print_hwcap(getauxval);
}

fn print_hwcap<G: Getauxval>(g: G) {
    match g.getauxval(AT_HWCAP) {
        Ok(v) => println!("Got HWCAP 0x{:016X}", v),
        Err(e) => println!("Got an error {:?}", e)
    }
}
