//! SolverForge Service - JVM lifecycle management
//!
//! This crate manages the lifecycle of the solverforge-wasm-service Java process,
//! providing an embedded service option for SolverForge users who don't want
//! to manage the solver service separately.

mod config;
mod error;
mod jar;
mod service;
mod util;

pub use config::ServiceConfig;
pub use error::{ServiceError, ServiceResult};
pub use jar::JarManager;
pub use service::EmbeddedService;

pub use solverforge_core::{HttpSolverService, SolverService};
