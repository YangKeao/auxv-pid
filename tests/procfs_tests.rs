#[cfg(target_os = "linux")]
extern crate auxv;
#[cfg(target_os = "linux")]
extern crate libc;

#[test]
#[cfg(target_os = "linux")]
fn search_procfs_finds_hwcap() {
    let map = auxv::procfs::search_procfs_auxv(&[auxv::AT_HWCAP]).unwrap();
    // there should be SOMETHING in the value
    assert!(*map.get(&auxv::AT_HWCAP).unwrap() > 0);
}

#[test]
#[cfg(target_os = "linux")]
fn search_procfs_finds_uid_matches_libc() {
    let map = auxv::procfs::search_procfs_auxv(&[11]).unwrap();
    // AT_UID
    let uid = map.get(&11).unwrap();

    let libc_uid = unsafe { libc::getuid() };
    assert_eq!(libc_uid as u64, *uid as u64);
}

#[test]
#[cfg(target_os="linux")]
fn iterate_procfs_finds_hwcap() {
    let iter = auxv::procfs::iterate_procfs_auxv().unwrap();

    assert_eq!(1, iter.map(|r| r.unwrap())
        .filter(|p| p.key == auxv::AT_HWCAP)
        .count());
}
