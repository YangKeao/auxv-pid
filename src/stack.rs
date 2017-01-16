use std;

use super::{AuxvPair, AuxvType};

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
