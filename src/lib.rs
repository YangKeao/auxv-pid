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
//! - See `include/uapi/linux/auxvec.h` in the Linux source (or `getauxval(3)`) for defined keys,
//!   as well as other header files for architecture-specific types.
//! - See `fs/binfmt_elf.c` in the Linux source for how the vector is generated.
//! - Searching for `AT_` in your OS of choice is likely to yield some good leads on the available
//!   constants and how it's generated.
//!
//! # Reading the auxiliary vector
//!
//! Unfortunately, there is no one best option for how to access the aux vector.
//!
//! - [`getauxval(3)`](https://linux.die.net/man/3/getauxval) is available in glibc 2.16+, Android
//!   libc (Bionic) since Android 4,3, and musl 1.1.0+. Since it is a non-standard extension, if
//!   you're not using those libc implementations (e.g. you're using musl, uclibc, etc), this will
//!   not be available. Also, if you're on glibc older than 2.19, or Bionic before March 2015,
//!   `getauxval` is unable to express the concept of "not found" and will instead "find" the value 0.
//! - `/proc/self/auxv` exposes the contents of the aux vector, but it only exists on Linux.
//!   Furthermore, the OS may be configured to not allow access to it (see `proc(5)`).
//! - Navigating the ELF stack layout manually is also (sometimes) possible. There isn't a
//!   standardized way of jumping directly to auxv in the stack, but we can start at the `environ`
//!   pointer (which is specified in POSIX) and navigate from there. This will work on any ELF
//!   OS, but it is `unsafe` and only is possible if the environment has not been modified since
//!   the process started.
//!
//! This library lets you use all of these options, so chances are pretty good that at least one of
//! them will work in any given host. See each submodule for details on how and when to use it.
//!
//! For most users, it would be best practice to try the `getauxval` way first, and then try the
//! procfs way if `getauxval` is not available at runtime. You should only try the stack crawling
//! way if you are sure that it is safe; see its docs for details.
//!
//! See the `examples` dir for examples of each way of accessing auxv.
//!
//! ## Auxv type width
//!
//! `AuxvType` is selected at compile time to be either `u32` or `u64` depending on the pointer
//! width of the system. This type is used for the key and value.

// The key/value pairs in auxv are either Elf32_auxv_t or Elf64_auxv_t.
// If this is an LP64 system (a "long" is 64 bits) then it seems that
// these entries will be Elf64_auxv_t (2x 64 bits). Fortunately,
// Unixen in general are LP64 (when on 64 bit), and ELF only exists
// on Unixen, which means we can simply use pointer width to detect 32
// vs 64 bit. Furthermore, some of the things auxv holds are pointers
// (e.g. AT_BASe and AT_EXECFN), so value has to be able to hold a
// pointer, and the key is always the same width as the value.
/// The type used in auxv keys and values.
#[cfg(target_pointer_width="32")]
pub type AuxvType = u32;
/// The type used in auxv keys and values.
#[cfg(target_pointer_width="64")]
pub type AuxvType = u64;

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

pub mod getauxval;
pub mod procfs;
pub mod stack;
