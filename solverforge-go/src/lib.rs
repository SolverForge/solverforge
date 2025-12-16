//! SolverForge Go Bindings - FFI Layer
//!
//! This crate provides C-compatible FFI exports that enable Go programs
//! to interact with the SolverForge constraint solver library.
//!
//! # Architecture
//!
//! ```text
//! Go Code → CGO → C FFI (this crate) → solverforge-core (Rust)
//! ```
//!
//! The FFI layer:
//! - Exports C-compatible functions for all LanguageBridge operations
//! - Manages GoBridge instances that implement LanguageBridge
//! - Handles type conversions between C representations and Rust types
//! - Provides memory-safe operations across the FFI boundary
//!
//! # Safety
//!
//! All exported functions follow these safety rules:
//! - Never panic across FFI boundary (return error codes instead)
//! - Always validate pointer parameters for null
//! - Use proper memory ownership and cleanup
//! - Document all unsafe operations

mod bridge;
mod conversions;
mod errors;
mod ffi;
mod registry;

pub use bridge::GoBridge;
pub use conversions::{CValue, CArray, CObject};
pub use errors::CError;
pub use ffi::*;
