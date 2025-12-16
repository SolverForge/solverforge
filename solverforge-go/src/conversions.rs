//! Type conversions between Rust Value and C-compatible CValue
//!
//! This module provides FFI-safe representations of the Value enum
//! that can be safely passed across the C boundary to Go code.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// C-compatible value type tags
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CValueTag {
    Null = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    String = 4,
    Array = 5,
    Object = 6,
    ObjectRef = 7,
}

/// C-compatible array structure
#[repr(C)]
pub struct CArray {
    pub values: *mut CValue,
    pub len: usize,
    pub cap: usize,
}

impl Default for CArray {
    fn default() -> Self {
        Self::new()
    }
}

impl CArray {
    /// Create a new empty CArray
    pub fn new() -> Self {
        CArray {
            values: ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }

    /// Create a CArray from a Vec<CValue>
    pub fn from_vec(mut vec: Vec<CValue>) -> Self {
        let len = vec.len();
        let cap = vec.capacity();
        let values = vec.as_mut_ptr();
        std::mem::forget(vec); // Prevent Rust from freeing the memory
        CArray { values, len, cap }
    }

    /// Free the CArray memory
    ///
    /// # Safety
    ///
    /// This must only be called once per CArray
    pub unsafe fn free(self) {
        if !self.values.is_null() {
            let vec = Vec::from_raw_parts(self.values, self.len, self.cap);
            for value in vec {
                value.free();
            }
        }
    }
}

/// C-compatible object structure (key-value map)
#[repr(C)]
pub struct CObject {
    pub keys: *mut *mut c_char,
    pub values: *mut CValue,
    pub len: usize,
    pub cap: usize,
}

impl Default for CObject {
    fn default() -> Self {
        Self::new()
    }
}

impl CObject {
    /// Create a new empty CObject
    pub fn new() -> Self {
        CObject {
            keys: ptr::null_mut(),
            values: ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }

    /// Free the CObject memory
    ///
    /// # Safety
    ///
    /// This must only be called once per CObject
    pub unsafe fn free(self) {
        if !self.keys.is_null() && !self.values.is_null() {
            for i in 0..self.len {
                let key_ptr = *self.keys.add(i);
                if !key_ptr.is_null() {
                    let _ = CString::from_raw(key_ptr);
                }
                let value_ptr = self.values.add(i);
                let value = ptr::read(value_ptr);
                value.free();
            }
            let _ = Vec::from_raw_parts(self.keys, self.len, self.cap);
            let _ = Vec::from_raw_parts(self.values, self.len, self.cap);
        }
    }
}

/// C-compatible value union
#[repr(C)]
pub union CValueData {
    pub bool_val: bool,
    pub int_val: i64,
    pub float_val: f64,
    pub string_ptr: *mut c_char,
    pub array_ptr: *mut CArray,
    pub object_ptr: *mut CObject,
    pub handle_val: u64,
}

/// C-compatible value structure
#[repr(C)]
pub struct CValue {
    pub tag: u8,
    pub data: CValueData,
}

impl CValue {
    /// Create a null CValue
    pub fn null() -> Self {
        CValue {
            tag: CValueTag::Null as u8,
            data: CValueData { int_val: 0 },
        }
    }

    /// Create a bool CValue
    pub fn bool(b: bool) -> Self {
        CValue {
            tag: CValueTag::Bool as u8,
            data: CValueData { bool_val: b },
        }
    }

    /// Create an int CValue
    pub fn int(i: i64) -> Self {
        CValue {
            tag: CValueTag::Int as u8,
            data: CValueData { int_val: i },
        }
    }

    /// Create a float CValue
    pub fn float(f: f64) -> Self {
        CValue {
            tag: CValueTag::Float as u8,
            data: CValueData { float_val: f },
        }
    }

    /// Create a string CValue
    pub fn string(s: String) -> Self {
        let c_string = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
        CValue {
            tag: CValueTag::String as u8,
            data: CValueData {
                string_ptr: c_string.into_raw(),
            },
        }
    }

    /// Create an object ref CValue
    pub fn object_ref(handle: u64) -> Self {
        CValue {
            tag: CValueTag::ObjectRef as u8,
            data: CValueData { handle_val: handle },
        }
    }

    /// Free the CValue memory
    ///
    /// # Safety
    ///
    /// This must only be called once per CValue
    pub unsafe fn free(self) {
        match self.tag {
            t if t == CValueTag::String as u8 => {
                if !self.data.string_ptr.is_null() {
                    let _ = CString::from_raw(self.data.string_ptr);
                }
            }
            t if t == CValueTag::Array as u8 => {
                if !self.data.array_ptr.is_null() {
                    let array = Box::from_raw(self.data.array_ptr);
                    array.free();
                }
            }
            t if t == CValueTag::Object as u8 => {
                if !self.data.object_ptr.is_null() {
                    let object = Box::from_raw(self.data.object_ptr);
                    object.free();
                }
            }
            _ => {} // Primitives don't need cleanup
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvalue_primitives() {
        let null = CValue::null();
        assert_eq!(null.tag, CValueTag::Null as u8);

        let bool_val = CValue::bool(true);
        assert_eq!(bool_val.tag, CValueTag::Bool as u8);
        unsafe {
            assert!(bool_val.data.bool_val);
        }

        let int_val = CValue::int(42);
        assert_eq!(int_val.tag, CValueTag::Int as u8);
        unsafe {
            assert_eq!(int_val.data.int_val, 42);
        }

        let float_val = CValue::float(42.5);
        assert_eq!(float_val.tag, CValueTag::Float as u8);
        unsafe {
            assert_eq!(float_val.data.float_val, 42.5);
        }

        unsafe {
            null.free();
            bool_val.free();
            int_val.free();
            float_val.free();
        }
    }
}
