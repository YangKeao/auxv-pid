//! # Safety
//! Only use this code when all of the following are satisfied:
//!
//! - In an ELF binary. In practice, this means Linux, FreeBSD, and other Unix
//!   variants (but not macOS).
//! - No other threads are manipulating the environment (setenv, putenv, or
//!   equivalent). In practice this means do it at startup before spawning
//!   any other threads.
//!
//! This works by navigating the stack at runtime to find the auxv
//! entries, so it is `unsafe`. **It's up to you to only invoke it in binaries
//! that are ELF.** In other words, only search for auxv entries on Linux,
//! FreeBSD, or other systems that you have verified to be compatible. It
//! will probably segfault or produce bogus output when run on non-ELF systems.
//!
//! Rust's environment manipulation code has a mutex to make `set_var` and `var`
//! thread safe, but this code can't access that mutex, so you are not protected
//! from shooting yourself in the foot even when using pure Rust. Also, that mutex
//! doesn't help against other C code screwing things up anyway. This code works
//! by navigating the stack starting at the environment, so it needs the environment
//! data to be quiescent in order to safely traverse it.
//!
//! The sole public function is `iterate_auxv`. It iterates across
//! the entries, exposing them via `AuxvPair`. The two fields in `AuxvPair`
//! will be of type `AuxvType`, which will be either `u64` or `u32` depending
//! on the system's pointer width.
//!
//! Here's how you could look for `AT_HWCAP` assuming you were only targeting Linux:
//!
//! ```
//! #[cfg(target_os="linux")]
//! use auxv::{iterate_auxv, AT_HWCAP};
//!
//! #[cfg(target_os="linux")]
//! fn match_auxv() {
//!     unsafe {
//!         match iterate_auxv().filter(|p| p.key == AT_HWCAP).next() {
//!             Some(p) => println!("Got value {}", p.value),
//!             None => println!("No HWCAP")
//!         }
//!     }
//! }
//! ```
//!
//! `iterate_auxv` is not available on Windows since they use a different (non-POSIX)
//! environment pointer name. And, of course, it wouldn't work even if it compiled.

// The type/value pairs in auxv are either Elf32_auxv_t or Elf64_auxv_t.
// If this is an LP64 system (a "long" is 64 bits) then it seems that
// these entries will be Elf64_auxv_t (2x 64 bits). Fortunately,
// Unixen in general are LP64 (when on 64 bit), and ELF only exists
// on Unixen, which means we can simply use pointer width to detect 32
// vs 64 bit. Furthermore, some of the things auxv holds are pointers
// (e.g. AT_BASe and AT_EXECFN), so value has to be able to hold a
// pointer, and the type is always the same width as the value.
#[cfg(target_pointer_width="32")]
pub type AuxvType = u32;
#[cfg(target_pointer_width="64")]
pub type AuxvType = u64;

/// Returns an iterator across the auxv entries.
#[cfg(not(target_os="windows"))]
pub unsafe fn iterate_auxv() -> StackAuxvIter {
    StackAuxvIter {
        auxv_type_ptr: get_auxv_ptr()
    }
}

pub struct StackAuxvIter {
    auxv_type_ptr: *const AuxvType,
}

impl Iterator for StackAuxvIter {
    type Item = AuxvPair;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if *self.auxv_type_ptr == 0 {
                // found AT_NULL, exit
                return None;
            };

            let key = *self.auxv_type_ptr;
            let value = *(self.auxv_type_ptr.offset(1));

            self.auxv_type_ptr = self.auxv_type_ptr.offset(2);

            Some(AuxvPair {
                key: key,
                value: value
            })
        }
    }
}

extern "C" {
    // pointer to start of env
    // env is a sequence of pointers to "NAME=value" strings. However, we don't
    // care that they're strings; we only care that they're pointers, and all
    // pointers are the same size, so we just use u8 as a dummy type here.
    // On windows it's `_environ` and they don't use ELF anyway.
    #[cfg(not(target_os="windows"))]
    static environ: *const *const u8;
}

/// returns a pointer to the first entry in the auxv table
/// (specifically, the type in the first type / value pair)
#[cfg(not(target_os="windows"))]
unsafe fn get_auxv_ptr() -> *const AuxvType {
    let mut env_entry_ptr = environ;

    while !(*env_entry_ptr).is_null() {
        // skip the pointers to environment strings
        env_entry_ptr = env_entry_ptr.offset(1);
    };

    // env_entry_ptr now points at the null entry after the environment listing
    // advance it one more to point at first entry of auxv
    env_entry_ptr = env_entry_ptr.offset(1);

    return std::mem::transmute::<*const *const u8, *const AuxvType>(env_entry_ptr);
}

// from [linux]/include/uapi/linux/auxvec.h. First 32 bits of HWCAP
// even on platforms where unsigned long is 64 bits.
pub const AT_HWCAP: AuxvType = 16;
pub const AT_HWCAP2: AuxvType = 26;

#[derive(Debug, PartialEq)]
pub struct AuxvPair {
    pub key: AuxvType,
    pub value: AuxvType,
}

#[cfg(test)]
mod tests {
    extern crate byteorder;
    extern crate libc;

    use std::io::{BufReader, Read};
    use std::fs::File;
    use std::path::Path;
    use std::vec::Vec;
    use std;

    #[cfg(any(target_os="linux", target_os="freebsd"))]
    use super::{AT_HWCAP, iterate_auxv};
    use super::{AuxvType, AuxvPair};

    use tests::byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
    #[cfg(target_os="linux")]
    use tests::byteorder::NativeEndian;

    #[test]
    #[cfg(target_os="linux")]
    fn auxv_via_stack_equals_auxv_via_procfs() {
        let procfs: Vec<AuxvPair> = iterate_procfs_auxv().unwrap()
            .map(|r| r.unwrap())
            .collect();
        unsafe {
            let stack: Vec<AuxvPair> = iterate_auxv()
                .collect();
            assert_eq!(procfs, stack);
        }
    }

    #[test]
    #[cfg(any(target_os="linux", target_os="freebsd"))]
    fn test_iterate_stack_finds_hwcap() {
        unsafe {
            let iter = iterate_auxv();

            assert_eq!(1, iter.filter(|p| p.key == AT_HWCAP).count());
        }
    }

    #[test]
    #[cfg(target_os="linux")]
    fn test_stack_auxv_uid_matches_libc_uid() {
        // AT_UID not populated on FreeBSD, so this is linux only
        unsafe {
            // AT_UID = 11
            let auxv_uid = iterate_auxv().filter(|p| p.key == 11)
                .map(|p| p.value)
                .next()
                .unwrap();

            let libc_uid = libc::getuid();
            assert_eq!(libc_uid as u64, auxv_uid as u64);
        }
    }

    #[test]
    #[cfg(target_os="linux")]
    fn test_iterate_procfs_finds_hwcap() {
        let iter = iterate_procfs_auxv().unwrap();

        assert_eq!(1, iter.map(|r| r.unwrap())
            .filter(|p| p.key == AT_HWCAP)
            .count());
    }

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
        // entry for type 15 is missing its value
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

    #[derive(Debug, PartialEq)]
    pub enum ProcfsAuxvError {
        /// an io error was encountered
        IoError,
        /// the auxv data is invalid
        InvalidFormat
    }

    /// procfs auxv impl as a sanity check for the stack crawling impl
    #[cfg(target_os="linux")]
    fn iterate_procfs_auxv() -> Result<ProcfsAuxvIter<NativeEndian, File>, ProcfsAuxvError> {
        iterate_path::<NativeEndian>(&Path::new("/proc/self/auxv"))
    }

    struct ProcfsAuxvIter<B: ByteOrder, R: Read> {
        pair_size: usize,
        buf: Vec<u8>,
        input: BufReader<R>,
        keep_going: bool,
        phantom_byteorder: std::marker::PhantomData<B>
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
            phantom_byteorder: std::marker::PhantomData
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
            let found_aux_type = match read_long::<B>(&mut reader) {
                Ok(x) => x,
                Err(_) => return Some(Err(ProcfsAuxvError::InvalidFormat))
            };
            let aux_val = match read_long::<B>(&mut reader) {
                Ok(x) => x,
                Err(_) => return Some(Err(ProcfsAuxvError::InvalidFormat))
            };

            // AT_NULL (0) signals the end of auxv
            if found_aux_type == 0 {
                return None;
            }

            self.keep_going = true;
            Some(Ok(AuxvPair {
                key: found_aux_type,
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
}
