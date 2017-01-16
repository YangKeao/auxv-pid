//! Read auxv entries by chasing pointers in the ELF stack layout.
//!
//! The sole public function is `iterate_stack_auxv`. It iterates across the entries, exposing them via
//! `AuxvPair`. The two fields in `AuxvPair` will be of type `AuxvType`.
//!
//! **Only use this code when all of the following are satisfied**:
//!
//! - In an ELF binary. In practice, this means Linux, FreeBSD, and other Unix
//!   variants (but not macOS).
//! - No other threads have manipulating the environment (setenv, putenv, or equivalent). In
//!   practice this means do it as the very first stuff in `main` before touching the environment
//!   or spawning any other threads.
//!
//! This works by navigating the stack at runtime to find the auxv entries, so it is `unsafe`.
//! It may segfault or produce bogus output when run on non-ELF systems or when the environment
//! has been modified. It relies on navigating from the original destination of the `environ`
//! pointer in the ELF stack layout to auxv.
//!
//! `iterate_stack_auxv` is not available on Windows since they use a different (non-POSIX)
//! environment pointer name. And, of course, it wouldn't work even if it compiled.
//!
//! If you're on a system with ELF executables (Linux, FreeBSD, other Unixes), run the example
//! that shows its own auxv keys and values: `cargo run --example elf_stack_show_auxv`.
//! It should print a short table of a dozen or two numbers. On macOS, it tends to produce garbage
//! numbers for a while before mercifully exiting normally. On Windows, the function is not
//! available because their names are not POSIX compatible so it wouldn't even compile, and so the
//! example prints nothing.

use std;

use super::{AuxvPair, AuxvType};

/// Returns an iterator across the auxv entries.
#[cfg(not(target_os="windows"))]
pub unsafe fn iterate_stack_auxv() -> StackAuxvIter {
    StackAuxvIter {
        auxv_key_ptr: get_auxv_ptr()
    }
}

/// An iterator across auxv pairs from crawling the ELF stack.
pub struct StackAuxvIter {
    auxv_key_ptr: *const AuxvType,
}

impl Iterator for StackAuxvIter {
    type Item = AuxvPair;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if *self.auxv_key_ptr == 0 {
                // found AT_NULL, exit
                return None;
            };

            let key = *self.auxv_key_ptr;
            let value = *(self.auxv_key_ptr.offset(1));

            self.auxv_key_ptr = self.auxv_key_ptr.offset(2);

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
/// (specifically, the key in the first key / value pair)
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
