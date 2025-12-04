//! SolverForge Core - Language-agnostic constraint solver library
//!
//! This crate provides the core functionality for SolverForge, a Rust-based
//! constraint solver library that uses WASM modules and HTTP communication
//! to solve constraint satisfaction and optimization problems by acting as a
//! bridge towards the Timefold JVM.
//!
//! # Architecture
//!
//! SolverForge is designed with a layered architecture:
//!
//! 1. **Core Types** (this crate) - Language-agnostic types and abstractions
//! 2. **Language Bindings** - Language-specific implementations (Python, JS, etc.)
//! 3. **Solver Service** - HTTP service that executes the solving
//!
//! # Key Components
//!
//! - [`Value`] - Language-agnostic value representation
//! - [`ObjectHandle`] / [`FunctionHandle`] - Opaque handles to host language objects
//! - [`SolverForgeError`] / [`SolverForgeResult`] - Error handling types
//!
//! # Example
//!
//! ```
//! use solverforge_core::{Value, ObjectHandle, FunctionHandle};
//!
//! // Create values
//! let int_val = Value::from(42i64);
//! let str_val = Value::from("hello");
//!
//! // Create handles
//! let obj_handle = ObjectHandle::new(1);
//! let func_handle = FunctionHandle::new(2);
//!
//! // Use in a map
//! use std::collections::HashMap;
//! let mut map = HashMap::new();
//! map.insert(obj_handle, "object 1");
//! ```

mod bridge;
pub mod constraints;
pub mod domain;
mod error;
mod handles;
pub mod score;
mod value;

pub use bridge::{ClassInfo, FieldInfo, LanguageBridge};
pub use constraints::{Collector, Joiner, WasmFunction};
pub use domain::{
    ConstraintConfiguration, ConstraintWeight, DeepPlanningClone, PlanningAnnotation,
    ShadowAnnotation,
};
pub use error::{SolverForgeError, SolverForgeResult};
pub use handles::{FunctionHandle, ObjectHandle};
pub use score::{
    BendableDecimalScore, BendableScore, HardMediumSoftDecimalScore, HardMediumSoftScore,
    HardSoftDecimalScore, HardSoftScore, Score, SimpleDecimalScore, SimpleScore,
};
pub use value::Value;
