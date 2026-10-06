/* Macros for generating arity-specific constraint streams.

These macros reduce code duplication across Bi/Tri/Quad/Penta streams
which all follow the same pattern with different tuple sizes.
*/

#[macro_use]
mod nary_stream;

/* Generates the constraint stream struct, builder struct, and common methods.

Doctests and unique methods (should be defined outside the macro
in the individual stream files.
*/
macro_rules! impl_arity_stream {
    (bi, $stream:ident, $builder:ident) => {
        impl_bi_arity_stream!($stream, $builder);
    };
    (tri, $stream:ident, $builder:ident) => {
        impl_tri_arity_stream!($stream, $builder);
    };
    (quad, $stream:ident, $builder:ident) => {
        impl_quad_arity_stream!($stream, $builder);
    };
    (penta, $stream:ident, $builder:ident) => {
        impl_penta_arity_stream!($stream, $builder);
    };
}

pub(crate) use impl_arity_stream;
