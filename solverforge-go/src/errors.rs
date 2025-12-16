//! FFI error handling
//!
//! This module provides C-compatible error types that can be safely
//! passed across the FFI boundary.

use solverforge_core::SolverForgeError;
use std::ffi::CString;
use std::os::raw::c_char;

/// C-compatible error structure
#[repr(C)]
pub struct CError {
    /// Error message (owned C string)
    pub message: *mut c_char,
    /// Error code
    pub code: u32,
}

/// Error codes matching SolverForgeError variants
#[repr(u32)]
pub enum ErrorCode {
    Unknown = 0,
    Serialization = 1,
    Http = 2,
    Solver = 3,
    Wasm = 4,
    Bridge = 5,
    Validation = 6,
    Configuration = 7,
    Service = 8,
    Io = 9,
}

impl CError {
    /// Create a new CError from a message and code
    pub fn new(message: String, code: ErrorCode) -> Self {
        let c_message = CString::new(message).unwrap_or_else(|_| CString::new("Invalid error message").unwrap());
        CError {
            message: c_message.into_raw(),
            code: code as u32,
        }
    }

    /// Create a CError from a SolverForgeError
    pub fn from_rust_error(error: SolverForgeError) -> Self {
        let (message, code) = match &error {
            SolverForgeError::Serialization(_) => (error.to_string(), ErrorCode::Serialization),
            SolverForgeError::Http(_) => (error.to_string(), ErrorCode::Http),
            SolverForgeError::Solver(_) => (error.to_string(), ErrorCode::Solver),
            SolverForgeError::WasmGeneration(_) => (error.to_string(), ErrorCode::Wasm),
            SolverForgeError::Bridge(_) => (error.to_string(), ErrorCode::Bridge),
            SolverForgeError::Validation(_) => (error.to_string(), ErrorCode::Validation),
            SolverForgeError::Configuration(_) => (error.to_string(), ErrorCode::Configuration),
            SolverForgeError::Service(_) => (error.to_string(), ErrorCode::Service),
            SolverForgeError::Io(_) => (error.to_string(), ErrorCode::Io),
            SolverForgeError::Other(_) => (error.to_string(), ErrorCode::Unknown),
        };
        Self::new(message, code)
    }

    /// Free the error message memory
    ///
    /// # Safety
    ///
    /// This must only be called once per CError, and the CError must not be used after this call.
    pub unsafe fn free(self) {
        if !self.message.is_null() {
            let _ = CString::from_raw(self.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cerror_creation() {
        let error = CError::new("test error".to_string(), ErrorCode::Bridge);
        assert_eq!(error.code, ErrorCode::Bridge as u32);
        assert!(!error.message.is_null());
        unsafe { error.free(); }
    }

    #[test]
    fn test_from_rust_error() {
        let rust_error = SolverForgeError::Bridge("test bridge error".to_string());
        let c_error = CError::from_rust_error(rust_error);
        assert_eq!(c_error.code, ErrorCode::Bridge as u32);
        unsafe { c_error.free(); }
    }
}
