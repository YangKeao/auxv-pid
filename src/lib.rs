//! # Just what is the auxiliary vector?
//!
//! The auxiliary vector (aka auxv) is some memory near the start of a running ELF program's stack.
//! Specifically, it's a sequence of pairs of either 64 bit or 32 bit unsigned ints. The two
//! components of the pair form a key and a value. This data is mostly there to help things
//! like runtime linkers, but sometimes it's useful for other reasons. It is ELF-specific; it does
//! not exist in, say, Mach-O.
//!
//! On most Unixy systems, you can have the linker print out the contents of the aux vector by
//! setting an environment variable when running a command like `LD_SHOW_AUXV=1 cat /dev/null`.
//!
//! The keys used in the aux vector are defined in various header files and typically prefixed with
//! `AT_`. Some of the data there is not available from any other source, like `AT_HWCAP` and
//! `AT_HWCAP2`. These expose bit vectors of architecture-dependent hardware capability information.
//! On ARM, for instance, the bit `1 << 12` in the value for `AT_HWCAP` will be set if the CPU
//! supports NEON, and `1 << 3` will be set in the value for `AT_HWCAP2` if the CPU supports
//! SHA-256 acceleration. Handy, if you're doing that sort of thing.
//!
//! Other keys are typically not used directly by programs, like `AT_UID`: the real user id is
//! great and all, but you'd pobably call [`getuid(2)`](https://linux.die.net/man/2/getuid) in C or
//! `libc::getuid` from Rust instead.
//!
//! For most people, probably the most interesting data in auxv is for `AT_HWCAP` or `AT_HWCAP2`
//! so those have constants defined in `auxv`, but you can of course use any other key as well;
//! you'll just have to look up the appropriate number.
//!
//! More info on the auxiliary vector:
//!
//! - http://articles.manugarg.com/aboutelfauxiliaryvectors.html
//! - http://phrack.org/issues/58/5.html
//! - See `include/uapi/linux/auxvec.h` in the Linux source (or `getauxval(3)`) for defined types,
//!   as well as other header files for architecture-specific types.
//! - See `fs/binfmt_elf.c` in the Linux source for how the vector is generated.
//! - Searching for `AT_` in your OS of choice is likely to yield some good leads on the available
//!   constants and how it's generated.
//!
//! # Reading the auxiliary vector
//!
//! Unfortunately, there is no one best option for how to access the aux vector.
//!
//! - [`getauxval(3)`](https://linux.die.net/man/3/getauxval) is available in glibc 2.16+ and Bionic
//!   (Android's libc) since 2013. Since it is a non-standard extension, if you're not using those
//!   libc implementations (e.g. you're using musl, uclibc, etc), this will not be available. Also,
//!   if you're on glibc older than 2.19, or Bionic before March 2015, `getauxval` is unable to
//!   express the concept of "not found" and will instead "find" the value 0.
//! - `/proc/self/auxv` exposes the contents of the aux vector, but it only exists on Linux.
//!   Furthermore, the OS may be configured to not allow access to it (see `proc(5)`).
//! - Navigating the ELF stack layout manually is also (sometimes) possible. There isn't a
//!   standardized way of jumping directly to auxv in the stack, but we can start at the `environ`
//!   pointer (which is specified in POSIX) and navigate from there. This will work on any ELF
//!   OS, but it is `unsafe` and only is possible if the environment has not been modified since
//!   the process started.
//!
//! This library lets you use all of these options, so chances are pretty good that at least one of
//! them will work in any given host. For most users, it would be best practice to try the
//! `getauxval` way first, and then try the procfs way if `getauxval` is not available at runtime.
//! You should only try the stack crawling way if you are sure that it is safe; see its docs for
//! details.
//!
//! ## Auxv type width
//!
//! `AuxvType` is selected at compile time to be either `u32` or `u64` depending on the pointer
//! width of the system. This type is used for all of the different ways to acccess auxv.
//!
//! ## Using `getauxval`
//!
//! Because the underlying `getauxval` C function is weakly linked, and only available on Linux,
//! access to it is done via the trait `Getauxval` to provide some indirection. On
//! `target_os="linux"`, the struct `NativeGetauxval` will be available, and that will call through
//! to `getauxval` if it is available and return an appropriate error if it is not. That means it
//! should be safe to try it if you're not sure your glibc has the function, etc.
//!
//! On all OSs, if you want a no-op sort of implementation (for use on non-Linux OSs, etc), you can
//! use `NotAvailableGetauxval`. It (surprise!) always returns the error that indicates that the
//! requested type was not found. Of course, you can also use write your own stub implementation of
//! the trait for testing.
//!
//! ## Using procfs
//!
//! Since it's just doing file I/O and not odd linkage tricks, the code to work with procfs is
//! available on all OSs but of course will return an error on non-Linux since it won't be able to
//! find `/proc/self/auxv` (or anything else in `/proc`).
//!
//! If you want a convenient way to query for just a handful of types, `search_procfs_auxv` is a
//! good choice. You provide a slice of `c_ulong` types to look for, and it builds a map of type
//! to value for the types you specify.
//!
//! If, on the other hand, you want to inspect everything in the aux vector, `iterate_procfs_auxv`
//! is what you want. It will let you iterate over every type/value pair in the aux vector. A minor
//! wrinkle is that there are two layers of `Result`: one for around the initial `Iterator`, and
//! another around each type/value pair. That's just the way I/O is...
//!
//! ## Using stack crawling
//!
//! The sole public function is `iterate_stack_auxv`. It iterates across the entries, exposing them via
//! `AuxvPair`. The two fields in `AuxvPair` will be of type `AuxvType`.
//!
//! Only use this code when all of the following are satisfied:
//!
//! - In an ELF binary. In practice, this means Linux, FreeBSD, and other Unix
//!   variants (but not macOS).
//! - No other threads have manipulating the environment (setenv, putenv, or equivalent). In
//!   practice this means do it as the very first stuff in `main` before touching the environment
//!   or spawning any other threads.
//!
//! This works by navigating the stack at runtime to find the auxv
//! entries, so it is `unsafe`. **It's up to you to only invoke it in binaries
//! that are ELF.** In other words, only search for auxv entries on Linux,
//! FreeBSD, or other systems that you have verified to be compatible. It
//! will probably segfault or produce bogus output when run on non-ELF systems.
//! `iterate_stack_auxv` is not available on Windows since they use a different (non-POSIX)
//! environment pointer name. And, of course, it wouldn't work even if it compiled.

//!
//! If you're on a system with ELF executables (Linux, FreeBSD, other Unixes), run the example
//! that shows its own auxv keys and values: `cargo run --example elf_stack_show_auxv`.
//! It should print a short table of a dozen or two numbers. On macOS, it tends to produce garbage
//! numbers for a while before mercifully exiting normally. On Windows, the function is not
//! available because their names are not POSIX compatible so it wouldn't even compile, and so the
//! example prints nothing.
//!
//! Rust's environment manipulation code has a mutex to make `set_var` and `var`
//! thread safe, but this code can't access that mutex, so you are not protected
//! from shooting yourself in the foot even when using pure Rust. Also, that mutex
//! doesn't help against other C code screwing things up anyway. This code works
//! by navigating the stack starting at the environment, so it needs the environment
//! data to be quiescent in order to safely traverse it.
//!


extern crate byteorder;

use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::fs::File;
use std::path::Path;
use std::vec::Vec;

use self::byteorder::{ByteOrder, ReadBytesExt, NativeEndian};

// The type/value pairs in auxv are either Elf32_auxv_t or Elf64_auxv_t.
// If this is an LP64 system (a "long" is 64 bits) then it seems that
// these entries will be Elf64_auxv_t (2x 64 bits). Fortunately,
// Unixen in general are LP64 (when on 64 bit), and ELF only exists
// on Unixen, which means we can simply use pointer width to detect 32
// vs 64 bit. Furthermore, some of the things auxv holds are pointers
// (e.g. AT_BASe and AT_EXECFN), so value has to be able to hold a
// pointer, and the type is always the same width as the value.
/// The type used in auxv keys and values.
#[cfg(target_pointer_width="32")]
pub type AuxvType = u32;
/// The type used in auxv keys and values.
#[cfg(target_pointer_width="64")]
pub type AuxvType = u64;

/// Returns an iterator across the auxv entries.
#[cfg(not(target_os="windows"))]
pub unsafe fn iterate_stack_auxv() -> StackAuxvIter {
    StackAuxvIter {
        auxv_type_ptr: get_auxv_ptr()
    }
}

/// An iterator across auxv pairs from crawling the ELF stack.
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

/// An auxv key-value pair.
#[derive(Debug, PartialEq)]
pub struct AuxvPair {
    pub key: AuxvType,
    pub value: AuxvType,
}

extern "C" {
    /// Invoke getauxval(3) if available. If it's not linked, or if invocation
    /// fails or the type is not found, sets success to false and returns 0.
    #[cfg(target_os="linux")]
    fn getauxval_wrapper(auxv_type: AuxvType, success: *mut AuxvType) -> i32;
}

/// Errors from invoking `getauxval`.
#[derive(Debug, PartialEq)]
pub enum GetauxvalError {
    /// getauxval() is not available at runtime
    FunctionNotAvailable,
    /// getauxval() could not find the requested type
    NotFound,
    /// getauxval() encountered a different error
    UnknownError
}

/// On Linux, you will probably want `NativeGetauxval`. If you're not
/// on Linux but want to use the same `getauxv`-based logic, you could
/// conditionally use `NotAvailableGetauxval` instead.
pub trait Getauxval {
    /// Look up an entry in the auxiliary vector. See getauxval(3) in glibc.
    fn getauxval(&self, auxv_type: AuxvType) -> Result<AuxvType, GetauxvalError>;
}

/// A stub implementation that always returns `FunctionNotAvailable`.
/// This can be used when you want to use something reasonable (i.e. won't crash or fail to
/// compile) that's not `NativeGetauxval` on non-Linux systems.
pub struct NotAvailableGetauxval {}

impl Getauxval for NotAvailableGetauxval {
    fn getauxval(&self, _: AuxvType) -> Result<AuxvType, GetauxvalError> {
        Err(GetauxvalError::FunctionNotAvailable)
    }
}

/// Calls through to the underlying glibc or Bionic `getauxval()`.
#[cfg(target_os="linux")]
pub struct NativeGetauxval {}

#[cfg(target_os="linux")]
impl Getauxval for NativeGetauxval {
    fn getauxval(&self, auxv_type: AuxvType)
                 -> Result<AuxvType, GetauxvalError> {

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

/// Read from the procfs auxv file and look for the specified types.
///
/// aux_types: the types to look for
/// returns a map of types to values, only including entries for types that were
/// requested that also had values in the aux vector
pub fn search_procfs_auxv(aux_types: &[AuxvType])
                          -> Result<HashMap<AuxvType, AuxvType>, ProcfsAuxvError> {
    search_auxv_path::<NativeEndian>(&Path::new("/proc/self/auxv"), aux_types)
}

/// Iterate over the contents of the procfs auxv file..
///
/// Note that the type iterated over is also a Result because further I/O errors
/// could occur at any time.
pub fn iterate_procfs_auxv() -> Result<ProcfsAuxvIter<NativeEndian, File>, ProcfsAuxvError> {
    iterate_path::<NativeEndian>(&Path::new("/proc/self/auxv"))
}

/// input: pairs of unsigned longs, as in /proc/self/auxv. The first of each
/// pair is the 'type' and the second is the 'value'.
fn search_auxv_path<B: ByteOrder>(path: &Path, aux_types: &[AuxvType])
                                  -> Result<HashMap<AuxvType, AuxvType>, ProcfsAuxvError> {
    let mut result = HashMap::<AuxvType, AuxvType>::new();

    for r in iterate_path::<B>(path)? {

        let pair = match r {
            Ok(p) => p,
            Err(e) => return Err(e)
        };

        if aux_types.contains(&pair.key) {
            let _ = result.insert(pair.key, pair.value);
        }
    }

    return Ok(result);
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

#[cfg(test)]
mod tests {
    extern crate libc;

    use std::path::Path;
    use std::vec::Vec;

    use super::*;
    use super::iterate_path;
    use super::byteorder::*;

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


}
