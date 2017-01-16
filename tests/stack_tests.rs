extern crate auxv;
extern crate libc;

use auxv::{AuxvPair, AT_HWCAP};
use auxv::stack::*;
use auxv::procfs::iterate_procfs_auxv;

#[test]
#[cfg(target_os="linux")]
fn auxv_via_stack_equals_auxv_via_procfs() {
    let procfs: Vec<AuxvPair> = iterate_procfs_auxv().unwrap()
        .map(|r| r.unwrap())
        .collect();
    unsafe {
        let stack: Vec<AuxvPair> = iterate_stack_auxv()
            .collect();
        assert_eq!(procfs, stack);
    }
}

#[test]
#[cfg(any(target_os="linux", target_os="freebsd"))]
fn test_iterate_stack_finds_hwcap() {
    unsafe {
        let iter = iterate_stack_auxv();

        assert_eq!(1, iter.filter(|p| p.key == AT_HWCAP).count());
    }
}

#[test]
#[cfg(target_os="linux")]
fn test_stack_auxv_uid_matches_libc_uid() {
    // AT_UID not populated on FreeBSD, so this is linux only
    unsafe {
        // AT_UID = 11
        let auxv_uid = iterate_stack_auxv().filter(|p| p.key == 11)
            .map(|p| p.value)
            .next()
            .unwrap();

        let libc_uid = libc::getuid();
        assert_eq!(libc_uid as u64, auxv_uid as u64);
    }
}

