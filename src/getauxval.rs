use super::AuxvType;

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
