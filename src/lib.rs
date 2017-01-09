extern crate byteorder;
extern crate libc;

use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::fs::File;
use std::path::Path;
use std::vec::Vec;

use libc::c_ulong;

use self::byteorder::{ByteOrder, ReadBytesExt, NativeEndian};

extern "C" {
    /// Invoke getauxval(3) if available. If it's not linked, or if invocation
    /// fails or the type is not found, sets success to false and returns 0.
    #[cfg(target_os="linux")]
    fn getauxval_wrapper(auxv_type: c_ulong, success: *mut c_ulong) -> i32;
}

#[derive(Debug, PartialEq)]
pub enum GetauxvalError {
    /// getauxval() is not available at runtime
    FunctionNotAvailable,
    /// getauxval() could not find the requested type
    NotFound,
    /// getauxval() encountered a different error
    UnknownError
}

pub trait GetauxvalProvider {
    /// Look up an entry in the auxiliary vector. See getauxval(3) in glibc.
    fn getauxval(&self, auxv_type: c_ulong) -> Result<c_ulong, GetauxvalError>;
}

/// A stub implementation that always returns NotFound.
/// This can be used when you want to use something reasonable (i.e. won't crash)
/// that's not `NativeGetauxvalProvider` on non-Linux systems.
pub struct NotFoundGetauxvalProvider {}

impl GetauxvalProvider for NotFoundGetauxvalProvider {
    fn getauxval(&self, _: c_ulong) -> Result<c_ulong, GetauxvalError> {
        Err(GetauxvalError::NotFound)
    }
}

/// Calls through to the underlying glibc `getauxval()`.
/// Unfortunately, prior to glibc 2.19, getauxval() returns 0 without
/// setting `errno` if the type is not found, so on such old systems
/// this will return `Ok(0)` rather than `Err(GetauxvalError::NotFound)`.
/// `getauxval` was first exposed in glibc 2.16 (released in 2012), so
/// ancient glibc systems will sto;; get `FunctionNotAvailable`.
#[cfg(target_os="linux")]
pub struct NativeGetauxvalProvider {}

#[cfg(target_os="linux")]
impl GetauxvalProvider for NativeGetauxvalProvider {
    /// Returns Some if the native invocation succeeds and the requested type was
    /// found, otherwise None.
    fn getauxval(&self, auxv_type: c_ulong)
                 -> Result<c_ulong, GetauxvalError> {

        let mut result = 0;
        unsafe {
            return match getauxval_wrapper(auxv_type, &mut result) {
                1 => Ok(result),
                0 => Err(GetauxvalError::NotFound),
                -1 => Err(GetauxvalError::FunctionNotAvailable),
                -2 => Err(GetauxvalError::UnknownError),
                x => panic!("getauxval_wrapper returned an unexpected value: {}", x)
            }
        }
    }
}

// from [linux]/include/uapi/linux/auxvec.h. First 32 bits of HWCAP
// even on platforms where unsigned long is 64 bits.
pub const AT_HWCAP: c_ulong = 16;
// currently only used by powerpc and arm64 AFAICT
pub const AT_HWCAP2: c_ulong = 26;

pub type ProcfsAuxVals = HashMap<c_ulong, c_ulong>;

#[derive(Debug, PartialEq)]
pub enum ProcfsAuxvError {
    /// an io error was encountered
    IoError,
    /// the auxv data is invalid
    InvalidFormat
}

/// Read from the procfs auxv file and look for the specified types.
///
/// aux_types: the types to look for
/// returns a map of types to values, only including entries for types that were
/// requested that also had values in the aux vector
pub fn search_procfs_auxv(aux_types: &[c_ulong])
        -> Result<ProcfsAuxVals, ProcfsAuxvError> {
    search_auxv_path::<NativeEndian>(&Path::new("/proc/self/auxv"), aux_types)
}

/// input: pairs of unsigned longs, as in /proc/self/auxv. The first of each
/// pair is the 'type' and the second is the 'value'.
fn search_auxv_path<B: ByteOrder>(path: &Path, aux_types: &[c_ulong])
        -> Result<ProcfsAuxVals, ProcfsAuxvError> {
    let mut input = File::open(path)
        .map_err(|_| ProcfsAuxvError::IoError)
        .map(|f| BufReader::new(f))?;

    let ulong_size = std::mem::size_of::<c_ulong>();
    let mut buf: Vec<u8> = Vec::with_capacity(2 * ulong_size);
    let mut result = HashMap::<c_ulong, c_ulong>::new();

    loop {
        buf.clear();
        // fill vec so we can slice into it
        for _ in 0 .. 2 * ulong_size {
            buf.push(0);
        }

        let mut read_bytes: usize = 0;
        while read_bytes < 2 * ulong_size {
            // read exactly buf's len of bytes.
            match input.read(&mut buf[read_bytes..]) {
                Ok(n) => {
                    if n == 0 {
                        // should not hit EOF before AT_NULL
                        return Err(ProcfsAuxvError::InvalidFormat)
                    }

                    read_bytes += n;
                }
                Err(_) => return Err(ProcfsAuxvError::IoError)
            }
        }

        let mut reader = &buf[..];
        let found_aux_type = read_long::<B>(&mut reader)
            .map_err(|_| ProcfsAuxvError::InvalidFormat)?;
        let aux_val = read_long::<B>(&mut reader)
            .map_err(|_| ProcfsAuxvError::InvalidFormat)?;

        if aux_types.contains(&found_aux_type) {
            let _ = result.insert(found_aux_type, aux_val);
        }

        // AT_NULL (0) signals the end of auxv
        if found_aux_type == 0 {
            return Ok(result);
        }
    }
}

fn read_long<B: ByteOrder> (reader: &mut Read) -> std::io::Result<c_ulong>{
    match std::mem::size_of::<c_ulong>() {
        4 => reader.read_u32::<B>().map(|u| u as c_ulong),
        8 => reader.read_u64::<B>().map(|u| u as c_ulong),
        x => panic!("Unexpected c_ulong width: {}", x)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    #[cfg(any(feature = "auxv-64bit-ulong",
    all(autodetect_c_ulong_64, not(feature = "auxv-32bit-ulong"))))]
    use super::ProcfsAuxvError;
    #[cfg(target_os="linux")]
    use super::search_procfs_auxv;
    use super::{search_auxv_path, AT_HWCAP, AT_HWCAP2};

    use byteorder::LittleEndian;
    use libc::c_ulong;
    
    // uid of program that read /proc/self/auxv
    const AT_UID: c_ulong = 11;

    // x86 hwcap bits from [linux]/arch/x86/include/asm/cpufeature.h
    const X86_FPU: u32 = 0 * 32 + 0;
    const X86_ACPI: u32 = 0 * 32 + 22;

    #[test]
    #[cfg(target_os="linux")]
    fn test_real_auxv_finds_hwcap() {
        let data = search_procfs_auxv(&[AT_HWCAP]).unwrap();
        assert!(*data.get(&AT_HWCAP).unwrap() > 0);
    }

    #[test]
    #[cfg(any(feature = "auxv-64bit-ulong",
    all(autodetect_c_ulong_64, not(feature = "auxv-32bit-ulong"))))]
    fn test_parse_auxv_virtualbox_linux() {
        let path = Path::new("src/test-data/macos-virtualbox-linux-x64-4850HQ.auxv");
        let vals = search_auxv_path::<LittleEndian>(path, &[AT_HWCAP, AT_HWCAP2, AT_UID])
            .unwrap();
        let hwcap = vals.get(&AT_HWCAP).unwrap();
        assert_eq!(&395049983, hwcap);

        assert_eq!(1, 1 << X86_FPU & hwcap);
        // virtualized, no acpi via msr I guess
        assert_eq!(0, 1 << X86_ACPI & hwcap);

        assert!(!vals.contains_key(&AT_HWCAP2));

        assert_eq!(&1000, vals.get(&AT_UID).unwrap());
    }

    #[test]
    #[cfg(any(feature = "auxv-32bit-ulong",
    all(autodetect_c_ulong_32, not(feature = "auxv-64bit-ulong"))))]
    fn test_parse_auxv_virtualbox_linux_32bit() {
        let path = Path::new("src/test-data/macos-virtualbox-linux-x86-4850HQ.auxv");
        let vals = search_auxv_path::<LittleEndian>(path, &[AT_HWCAP, AT_HWCAP2, AT_UID])
            .unwrap();
        let hwcap = vals.get(&AT_HWCAP).unwrap();
        assert_eq!(&126614527_u32, hwcap);

        assert_eq!(1, 1 << X86_FPU & hwcap);
        // virtualized, no acpi via msr I guess
        assert_eq!(0, 1 << X86_ACPI & hwcap);

        assert!(!vals.contains_key(&AT_HWCAP2));

        // this auxv was while running as root (unlike other auxv files)
        assert_eq!(&0_u32, vals.get(&AT_UID).unwrap());
    }

    #[test]
    #[cfg(any(feature = "auxv-64bit-ulong",
    all(autodetect_c_ulong_64, not(feature = "auxv-32bit-ulong"))))]
    fn test_parse_auxv_virtualbox_linux_32bit_in_64bit_mode_invalidformat() {
        let path = Path::new("src/test-data/macos-virtualbox-linux-x86-4850HQ.auxv");
        let vals = search_auxv_path::<LittleEndian>(path, &[AT_HWCAP, AT_HWCAP2, AT_UID]);

        assert_eq!(Err(ProcfsAuxvError::InvalidFormat), vals);
    }

    #[test]
    #[cfg(any(feature = "auxv-64bit-ulong",
    all(autodetect_c_ulong_64, not(feature = "auxv-32bit-ulong"))))]
    fn test_parse_auxv_real_linux() {
        let path = Path::new("src/test-data/linux-x64-i7-6850k.auxv");
        let vals = search_auxv_path::<LittleEndian>(path, &[AT_HWCAP, AT_HWCAP2, AT_UID])
            .unwrap();
        let hwcap = vals.get(&AT_HWCAP).unwrap();

        assert_eq!(&3219913727, hwcap);

        assert_eq!(1, 1 << X86_FPU & hwcap);
        assert_eq!(1 << X86_ACPI, 1 << X86_ACPI & hwcap);

        assert!(!vals.contains_key(&AT_HWCAP2));

        assert_eq!(&1000, vals.get(&AT_UID).unwrap());
    }

    #[test]
    #[cfg(any(feature = "auxv-64bit-ulong",
    all(autodetect_c_ulong_64, not(feature = "auxv-32bit-ulong"))))]
    fn test_parse_auxv_real_linux_half_of_trailing_null_missing_error() {
        let path = Path::new("src/test-data/linux-x64-i7-6850k-mangled-no-value-in-trailing-null.auxv");
        assert_eq!(ProcfsAuxvError::InvalidFormat,
            search_auxv_path::<LittleEndian>(path, &[555555555]).unwrap_err());
    }

    #[test]
    #[cfg(any(feature = "auxv-64bit-ulong",
    all(autodetect_c_ulong_64, not(feature = "auxv-32bit-ulong"))))]
    fn test_parse_auxv_real_linux_trailing_null_missing_error() {
        let path = Path::new("src/test-data/linux-x64-i7-6850k-mangled-no-trailing-null.auxv");
        assert_eq!(ProcfsAuxvError::InvalidFormat,
            search_auxv_path::<LittleEndian>(path, &[555555555]).unwrap_err());
    }

    #[test]
    #[cfg(any(feature = "auxv-64bit-ulong",
    all(autodetect_c_ulong_64, not(feature = "auxv-32bit-ulong"))))]
    fn test_parse_auxv_real_linux_truncated_entry_error() {
        let path = Path::new("src/test-data/linux-x64-i7-6850k-mangled-truncated-entry.auxv");
        assert_eq!(ProcfsAuxvError::InvalidFormat,
            search_auxv_path::<LittleEndian>(path, &[555555555]).unwrap_err());
    }

}
