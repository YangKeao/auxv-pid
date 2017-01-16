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
