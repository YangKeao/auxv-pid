//! Read auxv entries via Linux procfs.
//!
//! Since it's just doing file I/O and not odd linkage tricks, the code to work with procfs is
//! available on all OSs but of course will return an error on non-Linux since it won't be able to
//! find `/proc/self/auxv` (or anything else in `/proc`).
//!
//! If you want a convenient way to query for just a handful of keys, `search_procfs_auxv` is a
//! good choice. You provide a slice of keys to look for, and it builds a map of key
//! to value for the keys you specify.
//!
//! If, on the other hand, you want to inspect everything in the aux vector, `iterate_procfs_auxv`
//! is what you want. It will let you iterate over every key/value pair in the aux vector. A minor
//! wrinkle is that there are two layers of `Result`: one for around the initial `Iterator`, and
//! another around each key/value pair. That's just the way I/O is...


extern crate byteorder;

use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::fs::File;
use std::path::Path;
use std::marker::PhantomData;
use std;

use self::byteorder::{ByteOrder, ReadBytesExt, NativeEndian};

use super::{AuxvPair, AuxvType};

/// Read from the procfs auxv file and look for the specified keys.
///
/// keys: the keys to look for
/// returns a map of keys to values, only including entries for keys that were
/// requested that also had values in the aux vector
pub fn search_procfs_auxv(keys: &[AuxvType])
                          -> Result<HashMap<AuxvType, AuxvType>, ProcfsAuxvError> {
    let mut result = HashMap::<AuxvType, AuxvType>::new();

    for r in iterate_path::<NativeEndian>(&Path::new("/proc/self/auxv"))? {

        let pair = match r {
            Ok(p) => p,
            Err(e) => return Err(e)
        };

        if keys.contains(&pair.key) {
            let _ = result.insert(pair.key, pair.value);
        }
    }

    return Ok(result);

}

/// Iterate over the contents of the procfs auxv file..
///
/// Note that the type iterated over is also a Result because further I/O errors
/// could occur at any time.
pub fn iterate_procfs_auxv() -> Result<ProcfsAuxvIter<NativeEndian, File>, ProcfsAuxvError> {
    iterate_path::<NativeEndian>(&Path::new("/proc/self/auxv"))
}

/// Errors from reading `/proc/self/auxv`.
#[derive(Debug, PartialEq)]
pub enum ProcfsAuxvError {
    /// an io error was encountered
    IoError,
    /// the auxv data is invalid
    InvalidFormat
}

/// An iterator across auxv pairs froom procfs.
pub struct ProcfsAuxvIter<B: ByteOrder, R: Read> {
    pair_size: usize,
    buf: Vec<u8>,
    input: BufReader<R>,
    keep_going: bool,
    phantom_byteorder: PhantomData<B>
}

fn iterate_path<B: ByteOrder>(path: &Path)
                              -> Result<ProcfsAuxvIter<B, File>, ProcfsAuxvError> {
    let input = File::open(path)
        .map_err(|_| ProcfsAuxvError::IoError)
        .map(|f| BufReader::new(f))?;

    let pair_size = 2 * std::mem::size_of::<AuxvType>();
    let buf: Vec<u8> = Vec::with_capacity(pair_size);

    Ok(ProcfsAuxvIter::<B, File> {
        pair_size: pair_size,
        buf: buf,
        input: input,
        keep_going: true,
        phantom_byteorder: PhantomData
    })
}


impl<B: ByteOrder, R: Read> Iterator for ProcfsAuxvIter<B, R> {
    type Item = Result<AuxvPair, ProcfsAuxvError>;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.keep_going {
            return None
        }
        // assume something will fail
        self.keep_going = false;

        self.buf.clear();
        // fill vec so we can slice into it
        for _ in 0 .. self.pair_size {
            self.buf.push(0);
        }

        let mut read_bytes: usize = 0;
        while read_bytes < self.pair_size {
            // read exactly buf's len of bytes.
            match self.input.read(&mut self.buf[read_bytes..]) {
                Ok(n) => {
                    if n == 0 {
                        // should not hit EOF before AT_NULL
                        return Some(Err(ProcfsAuxvError::InvalidFormat))
                    }

                    read_bytes += n;
                }
                Err(_) => return Some(Err(ProcfsAuxvError::IoError))
            }
        }

        let mut reader = &self.buf[..];
        let aux_key = match read_long::<B>(&mut reader) {
            Ok(x) => x,
            Err(_) => return Some(Err(ProcfsAuxvError::InvalidFormat))
        };
        let aux_val = match read_long::<B>(&mut reader) {
            Ok(x) => x,
            Err(_) => return Some(Err(ProcfsAuxvError::InvalidFormat))
        };

        // AT_NULL (0) signals the end of auxv
        if aux_key == 0 {
            return None;
        }

        self.keep_going = true;
        Some(Ok(AuxvPair {
            key: aux_key,
            value: aux_val
        }))
    }
}

fn read_long<B: ByteOrder> (reader: &mut Read) -> std::io::Result<AuxvType>{
    match std::mem::size_of::<AuxvType>() {
        4 => reader.read_u32::<B>().map(|u| u as AuxvType),
        8 => reader.read_u64::<B>().map(|u| u as AuxvType),
        x => panic!("Unexpected type width: {}", x)
    }
}


#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::iterate_path;
    #[cfg(target_pointer_width="64")]
    use super::ProcfsAuxvError;
    use super::byteorder::*;
    use super::super::AuxvPair;

    #[test]
    #[cfg(target_pointer_width="64")]
    fn test_iterate_auxv_path_real_linux_x64() {
        let path = Path::new("src/test-data/linux-x64-i7-6850k.auxv");
        let mut iter = iterate_path::<LittleEndian>(path).unwrap();
        // x86 AT_SYSINFO_EHDR
        assert_eq!(AuxvPair { key: 33, value: 140724395515904 }, iter.next().unwrap().unwrap());
        // AT_HWCAP
        assert_eq!(AuxvPair { key: 16, value: 3219913727 }, iter.next().unwrap().unwrap());
        // AT_PAGESZ
        assert_eq!(AuxvPair { key: 6, value: 4096 }, iter.next().unwrap().unwrap());
        // AT_CLKTCK
        assert_eq!(AuxvPair { key: 17, value: 100 }, iter.next().unwrap().unwrap());
        // AT_PHDR
        assert_eq!(AuxvPair { key: 3, value: 4194368 }, iter.next().unwrap().unwrap());
        // AT_PHENT
        assert_eq!(AuxvPair { key: 4, value: 56 }, iter.next().unwrap().unwrap());
        // AT_PHNUM
        assert_eq!(AuxvPair { key: 5, value: 10 }, iter.next().unwrap().unwrap());
        // AT_BASE
        assert_eq!(AuxvPair { key: 7, value: 139881368498176 }, iter.next().unwrap().unwrap());
        // AT_FLAGS
        assert_eq!(AuxvPair { key: 8, value: 0 }, iter.next().unwrap().unwrap());
        // AT_ENTRY
        assert_eq!(AuxvPair { key: 9, value: 4204128 }, iter.next().unwrap().unwrap());
        // AT_UID
        assert_eq!(AuxvPair { key: 11, value: 1000 }, iter.next().unwrap().unwrap());
        // AT_EUID
        assert_eq!(AuxvPair { key: 12, value: 1000 }, iter.next().unwrap().unwrap());
        // AT_GID
        assert_eq!(AuxvPair { key: 13, value: 1000 }, iter.next().unwrap().unwrap());
        // AT_EGID
        assert_eq!(AuxvPair { key: 14, value: 1000 }, iter.next().unwrap().unwrap());
        // AT_SECURE
        assert_eq!(AuxvPair { key: 23, value: 0 }, iter.next().unwrap().unwrap());
        // AT_RANDOM
        assert_eq!(AuxvPair { key: 25, value: 140724393842889 }, iter.next().unwrap().unwrap());
        // AT_EXECFN
        assert_eq!(AuxvPair { key: 31, value: 140724393852911 }, iter.next().unwrap().unwrap());
        // AT_PLATFORM
        assert_eq!(AuxvPair { key: 15, value: 140724393842905 }, iter.next().unwrap().unwrap());
        assert_eq!(None, iter.next());
    }

    #[test]
    #[cfg(target_pointer_width="32")]
    fn test_iterate_auxv_path_virtualbox_linux_x86() {
        let path = Path::new("src/test-data/macos-virtualbox-linux-x86-4850HQ.auxv");
        let mut iter = iterate_path::<LittleEndian>(path).unwrap();
        // x86 AT_SYSINFO
        assert_eq!(AuxvPair { key: 32, value: 3078061308 }, iter.next().unwrap().unwrap());
        // x86 AT_SYSINFO_EHDR
        assert_eq!(AuxvPair { key: 33, value: 3078057984 }, iter.next().unwrap().unwrap());
        // AT_HWCAP
        assert_eq!(AuxvPair { key: 16, value: 126614527 }, iter.next().unwrap().unwrap());
        // AT_PAGESZ
        assert_eq!(AuxvPair { key: 6, value: 4096}, iter.next().unwrap().unwrap());
        // AT_CLKTCK
        assert_eq!(AuxvPair { key: 17, value: 100}, iter.next().unwrap().unwrap());
        // AT_PHDR
        assert_eq!(AuxvPair { key: 3, value: 134512692 }, iter.next().unwrap().unwrap());
        // AT_PHENT
        assert_eq!(AuxvPair { key: 4, value: 32 }, iter.next().unwrap().unwrap());
        // AT_PHNUM
        assert_eq!(AuxvPair { key: 5, value: 9 }, iter.next().unwrap().unwrap());
        // AT_BASE
        assert_eq!(AuxvPair { key: 7, value: 3078066176 }, iter.next().unwrap().unwrap());
        // AT_FLAGS
        assert_eq!(AuxvPair { key: 8, value: 0 }, iter.next().unwrap().unwrap());
        // AT_ENTRY
        assert_eq!(AuxvPair { key: 9, value: 134520424 }, iter.next().unwrap().unwrap());
        // AT_UID
        assert_eq!(AuxvPair { key: 11, value: 0 }, iter.next().unwrap().unwrap());
        // AT_EUID
        assert_eq!(AuxvPair { key: 12, value: 0 }, iter.next().unwrap().unwrap());
        // AT_GID
        assert_eq!(AuxvPair { key: 13, value: 0 }, iter.next().unwrap().unwrap());
        // AT_EGID
        assert_eq!(AuxvPair { key: 14, value: 0 }, iter.next().unwrap().unwrap());
        // AT_SECURE
        assert_eq!(AuxvPair { key: 23, value: 0 }, iter.next().unwrap().unwrap());
        // AT_RANDOM
        assert_eq!(AuxvPair { key: 25, value: 3219671659 }, iter.next().unwrap().unwrap());
        // AT_EXECFN
        assert_eq!(AuxvPair { key: 31, value: 3219677171 }, iter.next().unwrap().unwrap());
        // AT_PLATFORM
        assert_eq!(AuxvPair { key: 15, value: 3219671675 }, iter.next().unwrap().unwrap());
        assert_eq!(None, iter.next());
    }

    #[test]
    #[cfg(target_pointer_width="64")]
    fn test_iterate_auxv_path_real_linux_no_value_in_trailing_null() {
        let path = Path::new("src/test-data/linux-x64-i7-6850k-mangled-no-value-in-trailing-null.auxv");
        let mut iter = iterate_path::<LittleEndian>(path).unwrap();
        assert_eq!(AuxvPair { key: 33, value: 140724395515904 }, iter.next().unwrap().unwrap());
        // skip the middle ones
        let mut skipped = iter.skip(16);

        assert_eq!(AuxvPair { key: 15, value: 140724393842905 }, skipped.next().unwrap().unwrap());
        assert_eq!(ProcfsAuxvError::InvalidFormat, skipped.next().unwrap().unwrap_err());
        assert_eq!(None, skipped.next());
    }

    #[test]
    #[cfg(target_pointer_width="64")]
    fn test_iterate_auxv_path_real_linux_truncated_entry() {
        let path = Path::new("src/test-data/linux-x64-i7-6850k-mangled-truncated-entry.auxv");
        let mut iter = iterate_path::<LittleEndian>(path).unwrap();
        // x86 AT_SYSINFO_EHDR
        assert_eq!(AuxvPair { key: 33, value: 140724395515904 }, iter.next().unwrap().unwrap());
        // skip the middle ones, one fewer this time
        let mut skipped = iter.skip(15);
        assert_eq!(AuxvPair { key: 31, value: 140724393852911 }, skipped.next().unwrap().unwrap());
        // entry for key 15 is missing its value
        assert_eq!(ProcfsAuxvError::InvalidFormat, skipped.next().unwrap().unwrap_err());
        assert_eq!(None, skipped.next());
    }

    #[test]
    #[cfg(target_pointer_width="64")]
    fn test_iterate_auxv_path_real_linux_no_trailing_null() {
        let path = Path::new("src/test-data/linux-x64-i7-6850k-mangled-no-trailing-null.auxv");
        let mut iter = iterate_path::<LittleEndian>(path).unwrap();
        assert_eq!(AuxvPair { key: 33, value: 140724395515904 }, iter.next().unwrap().unwrap());
        // skip the middle ones
        let mut skipped = iter.skip(16);

        assert_eq!(AuxvPair { key: 15, value: 140724393842905 }, skipped.next().unwrap().unwrap());
        assert_eq!(ProcfsAuxvError::InvalidFormat, skipped.next().unwrap().unwrap_err());
        assert_eq!(None, skipped.next());
    }

    #[test]
    #[cfg(target_pointer_width="64")]
    fn test_parse_auxv_path_virtualbox_linux_32bit_in_64bit_mode_invalidformat() {
        let path = Path::new("src/test-data/macos-virtualbox-linux-x86-4850HQ.auxv");
        let mut iter = iterate_path::<LittleEndian>(path).unwrap();

        // 19 entries + null in that x86 auxv, so 10 total if you're in 64 bit mode

        for _ in 0..10 {
            assert!(iter.next().unwrap().is_ok());
        }

        assert_eq!(ProcfsAuxvError::InvalidFormat, iter.next().unwrap().unwrap_err());
    }

    #[test]
    #[cfg(target_pointer_width="32")]
    fn test_iterate_auxv_path_rpi3_arm() {
        let path = Path::new("src/test-data/linux-rpi3.auxv");
        let mut iter = iterate_path::<LittleEndian>(path).unwrap();
        // x86 AT_SYSINFO_EHDR
        assert_eq!(AuxvPair { key: 33, value: 2122829824 }, iter.next().unwrap().unwrap());
        // AT_HWCAP
        assert_eq!(AuxvPair { key: 16, value: 4174038 }, iter.next().unwrap().unwrap());
        // AT_PAGESZ
        assert_eq!(AuxvPair { key: 6, value: 4096}, iter.next().unwrap().unwrap());
        // AT_CLKTCK
        assert_eq!(AuxvPair { key: 17, value: 100}, iter.next().unwrap().unwrap());
        // AT_PHDR
        assert_eq!(AuxvPair { key: 3, value: 65588 }, iter.next().unwrap().unwrap());
        // AT_PHENT
        assert_eq!(AuxvPair { key: 4, value: 32 }, iter.next().unwrap().unwrap());
        // AT_PHNUM
        assert_eq!(AuxvPair { key: 5, value: 9 }, iter.next().unwrap().unwrap());
        // AT_BASE
        assert_eq!(AuxvPair { key: 7, value: 1995284480 }, iter.next().unwrap().unwrap());
        // AT_FLAGS
        assert_eq!(AuxvPair { key: 8, value: 0 }, iter.next().unwrap().unwrap());
        // AT_ENTRY
        assert_eq!(AuxvPair { key: 9, value: 72569 }, iter.next().unwrap().unwrap());
        // AT_UID
        assert_eq!(AuxvPair { key: 11, value: 1000 }, iter.next().unwrap().unwrap());
        // AT_EUID
        assert_eq!(AuxvPair { key: 12, value: 1000 }, iter.next().unwrap().unwrap());
        // AT_GID
        assert_eq!(AuxvPair { key: 13, value: 1000 }, iter.next().unwrap().unwrap());
        // AT_EGID
        assert_eq!(AuxvPair { key: 14, value: 1000 }, iter.next().unwrap().unwrap());
        // AT_SECURE
        assert_eq!(AuxvPair { key: 23, value: 0 }, iter.next().unwrap().unwrap());
        // AT_RANDOM
        assert_eq!(AuxvPair { key: 25, value: 2122731163 }, iter.next().unwrap().unwrap());
        // AT_HWCAP2
        assert_eq!(AuxvPair { key: 26, value: 16 }, iter.next().unwrap().unwrap());
        // AT_EXECFN
        assert_eq!(AuxvPair { key: 31, value: 2122731507 }, iter.next().unwrap().unwrap());
        // AT_PLATFORM
        assert_eq!(AuxvPair { key: 15, value: 2122731179 }, iter.next().unwrap().unwrap());
        assert_eq!(None, iter.next());
    }

}
