#[cfg(target_os = "linux")]
extern crate auxv;
#[cfg(target_os = "linux")]
extern crate byteorder;
#[cfg(target_os = "linux")]
extern crate libc;

#[cfg(target_os = "linux")]
use auxv::getauxval::{GetauxvalError, Getauxval, NativeGetauxval};
#[cfg(target_os = "linux")]
use auxv::AT_HWCAP;

#[test]
#[cfg(target_os = "linux")]
fn test_getauxv_hwcap_linux_finds_hwcap() {
    let native_getauxval = NativeGetauxval {};
    let result = native_getauxval.getauxval(AT_HWCAP);
    // there should be SOMETHING in the value
    assert!(result.unwrap() > 0);
}

#[test]
#[cfg(target_os = "linux")]
fn test_getauxv_hwcap_linux_doesnt_find_bogus_key() {
    let native_getauxval = NativeGetauxval {};

    // AT_NULL aka 0 is effectively the EOF for auxv, so it's never a valid key
    assert_eq!(GetauxvalError::NotFound, native_getauxval.getauxval(0).unwrap_err());
}
