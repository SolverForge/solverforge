//! FFI exports for Go bindings
//!
//! This module provides C-compatible functions that can be called from Go via CGO.
//! All functions follow these safety rules:
//! - Never panic (catch all panics and return error codes)
//! - Validate all pointer parameters
//! - Document memory ownership clearly

use crate::bridge::GoBridge;
use crate::errors::CError;
use std::panic;
use std::ptr;

/// Create a new GoBridge instance
///
/// Returns a pointer to a new GoBridge, or null on failure.
///
/// # Safety
///
/// The returned pointer must be freed with `solverforge_bridge_free`.
#[no_mangle]
pub extern "C" fn solverforge_bridge_new() -> *mut GoBridge {
    match panic::catch_unwind(|| Box::into_raw(Box::new(GoBridge::new()))) {
        Ok(ptr) => ptr,
        Err(_) => ptr::null_mut(),
    }
}

/// Free a GoBridge instance
///
/// # Safety
///
/// - `bridge` must be a valid pointer returned from `solverforge_bridge_new`
/// - `bridge` must not be used after this call
/// - This function must only be called once per bridge
#[no_mangle]
pub unsafe extern "C" fn solverforge_bridge_free(bridge: *mut GoBridge) {
    if bridge.is_null() {
        return;
    }

    let _ = panic::catch_unwind(|| {
        let _ = Box::from_raw(bridge);
    });
}

/// Register a Go object and get its handle ID
///
/// # Parameters
///
/// - `bridge`: The GoBridge instance
/// - `go_ref_id`: The Go-side object reference ID
/// - `out_handle`: Output parameter for the handle ID
/// - `out_error`: Output parameter for error (null on success)
///
/// Returns `true` on success, `false` on failure.
///
/// # Safety
///
/// - `bridge` must be a valid GoBridge pointer
/// - `out_handle` must be a valid pointer to u64
/// - `out_error` must be a valid pointer to *mut CError
#[no_mangle]
pub unsafe extern "C" fn solverforge_register_object(
    bridge: *mut GoBridge,
    go_ref_id: u64,
    out_handle: *mut u64,
    out_error: *mut *mut CError,
) -> bool {
    if bridge.is_null() || out_handle.is_null() || out_error.is_null() {
        return false;
    }

    match panic::catch_unwind(|| {
        let bridge_ref = &*bridge;
        let handle = bridge_ref.register_object(go_ref_id);
        *out_handle = handle.id();
        *out_error = ptr::null_mut();
    }) {
        Ok(_) => true,
        Err(_) => {
            *out_error = Box::into_raw(Box::new(CError::new(
                "Panic in solverforge_register_object".to_string(),
                crate::errors::ErrorCode::Bridge,
            )));
            false
        }
    }
}

/// Free a CError instance
///
/// # Safety
///
/// - `error` must be a valid CError pointer
/// - `error` must not be used after this call
/// - This function must only be called once per error
#[no_mangle]
pub unsafe extern "C" fn solverforge_error_free(error: *mut CError) {
    if error.is_null() {
        return;
    }

    let _ = panic::catch_unwind(|| {
        let error_box = Box::from_raw(error);
        error_box.free();
    });
}

/// Get the number of registered objects in the bridge
///
/// This is primarily for testing and debugging.
///
/// # Safety
///
/// - `bridge` must be a valid GoBridge pointer
#[no_mangle]
pub unsafe extern "C" fn solverforge_bridge_object_count(bridge: *mut GoBridge) -> usize {
    if bridge.is_null() {
        return 0;
    }

    panic::catch_unwind(|| {
        let bridge_ref = &*bridge;
        bridge_ref.registry().object_count()
    })
    .unwrap_or_default()
}

/// Get the number of registered functions in the bridge
///
/// This is primarily for testing and debugging.
///
/// # Safety
///
/// - `bridge` must be a valid GoBridge pointer
#[no_mangle]
pub unsafe extern "C" fn solverforge_bridge_function_count(bridge: *mut GoBridge) -> usize {
    if bridge.is_null() {
        return 0;
    }

    panic::catch_unwind(|| {
        let bridge_ref = &*bridge;
        bridge_ref.registry().function_count()
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_lifecycle() {
        unsafe {
            let bridge = solverforge_bridge_new();
            assert!(!bridge.is_null());

            solverforge_bridge_free(bridge);
        }
    }

    #[test]
    fn test_register_object() {
        unsafe {
            let bridge = solverforge_bridge_new();
            assert!(!bridge.is_null());

            let mut handle: u64 = 0;
            let mut error: *mut CError = ptr::null_mut();

            let success = solverforge_register_object(bridge, 100, &mut handle, &mut error);
            assert!(success);
            assert_eq!(error, ptr::null_mut());
            assert_ne!(handle, 0);

            assert_eq!(solverforge_bridge_object_count(bridge), 1);

            solverforge_bridge_free(bridge);
        }
    }

    #[test]
    fn test_null_pointer_safety() {
        unsafe {
            // Test with null bridge
            solverforge_bridge_free(ptr::null_mut());

            let mut handle: u64 = 0;
            let mut error: *mut CError = ptr::null_mut();
            let success =
                solverforge_register_object(ptr::null_mut(), 100, &mut handle, &mut error);
            assert!(!success);

            assert_eq!(solverforge_bridge_object_count(ptr::null_mut()), 0);
            assert_eq!(solverforge_bridge_function_count(ptr::null_mut()), 0);
        }
    }
}
