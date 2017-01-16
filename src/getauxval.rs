//! Read auxv entries one at a time via `getauxval`.
//!
//! Because the underlying `getauxval` C function is weakly linked, and only available on Linux,
//! access to it is done via the trait `Getauxval` to provide some indirection. On
//! `target_os="linux"`, the struct `NativeGetauxval` will be available, and that will call through
//! to `getauxval` if it is available and return an appropriate error if it is not. That means it
//! should be safe to try it if you're not sure your glibc has the function, etc.
//!
//! On all OSs, if you want a no-op sort of implementation (for use on non-Linux OSs, etc), you can
//! use `NotAvailableGetauxval`. It (surprise!) always returns the error that indicates that
//! `getauxval` function was not found. Of course, you can also use write your own stub
//! implementation of the trait for testing.

use super::AuxvType;

extern "C" {
    /// Invoke getauxval(3) if available.
    #[cfg(target_os="linux")]
    fn getauxval_wrapper(key: AuxvType, success: *mut AuxvType) -> i32;
}
/// Errors from invoking `getauxval`.
#[derive(Debug, PartialEq)]
pub enum GetauxvalError {
    /// getauxval() is not available at runtime
    FunctionNotAvailable,
    /// getauxval() could not find the requested key
    NotFound,
    /// getauxval() encountered a different error
    UnknownError
}

/// On Linux, you will probably want `NativeGetauxval`. If you're not
/// on Linux but want to use the same `getauxv`-based logic, you could
/// conditionally use `NotAvailableGetauxval` instead.
pub trait Getauxval {
    /// Look up an entry in the auxiliary vector. See getauxval(3) in glibc.
    fn getauxval(&self, key: AuxvType) -> Result<AuxvType, GetauxvalError>;
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
    fn getauxval(&self, key: AuxvType)
                 -> Result<AuxvType, GetauxvalError> {

        let mut result = 0;
        unsafe {
            return match getauxval_wrapper(key, &mut result) {
                1 => Ok(result),
                0 => Err(GetauxvalError::NotFound),
                -1 => Err(GetauxvalError::FunctionNotAvailable),
                -2 => Err(GetauxvalError::UnknownError),
                x => panic!("getauxval_wrapper returned an unexpected value: {}", x)
            }
        }
    }
}
