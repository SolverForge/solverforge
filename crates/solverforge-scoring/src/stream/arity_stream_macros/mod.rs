//! Macros for generating arity-specific constraint streams.
//!
//! These macros reduce code duplication across Bi/Tri/Quad/Penta streams
//! which all follow the same pattern with different tuple sizes.

#[macro_use]
mod bi;
#[macro_use]
mod penta;
#[macro_use]
mod quad;
#[macro_use]
mod tri;

/// Generates the constraint stream struct, builder struct, and common methods.
///
/// Doctests and unique methods (like join_self) should be defined outside the macro
/// in the individual stream files.
macro_rules! impl_arity_stream {
    (bi, $stream:ident, $builder:ident, $constraint:ident) => {
        impl_bi_arity_stream!($stream, $builder, $constraint);
    };
    (tri, $stream:ident, $builder:ident, $constraint:ident) => {
        impl_tri_arity_stream!($stream, $builder, $constraint);
    };
    (quad, $stream:ident, $builder:ident, $constraint:ident) => {
        impl_quad_arity_stream!($stream, $builder, $constraint);
    };
    (penta, $stream:ident, $builder:ident, $constraint:ident) => {
        impl_penta_arity_stream!($stream, $builder, $constraint);
    };
}

pub(crate) use impl_arity_stream;
