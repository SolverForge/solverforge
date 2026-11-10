//! Macros for generating arity-specific constraint streams.
//!
//! These macros reduce code duplication across Bi/Tri/Quad/Penta streams
//! which all follow the same pattern with different tuple sizes.

#[macro_use]
mod nary_stream;

#[macro_use]
mod dispatch;

pub(crate) use dispatch::impl_arity_stream;
