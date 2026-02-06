//! Dynamic constraint system using expression trees with true incremental scoring.

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

// Module organization: types, closures, factories, and stream operations
mod closures_cross;
mod closures_extract;
mod closures_flattened;
mod closures_penta;
mod closures_quad;
mod closures_tri;
mod factory_cross;
mod factory_self;
mod factory_self_higher;
mod factory_uni;
mod stream_ops;
mod stream_parser;
mod types;

// Re-export stream operations
pub use stream_ops::{build_from_stream_ops, StreamOp};
