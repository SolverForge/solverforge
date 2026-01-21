//! Filter wrappers for closures and constant filters.

use super::traits::{BiFilter, PentaFilter, QuadFilter, TriFilter, UniFilter};

/// A filter that always returns true.
#[derive(Debug, Clone, Copy, Default)]
pub struct TrueFilter;

macro_rules! impl_true_filter {
    ($trait:ident, $($param:ident),+) => {
        impl<S, $($param),+> $trait<S, $($param),+> for TrueFilter {
            #[inline]
            fn test(&self, _: &S, $(_: &$param),+) -> bool {
                true
            }
        }
    };
}

impl_true_filter!(UniFilter, A);
impl_true_filter!(BiFilter, A, B);
impl_true_filter!(TriFilter, A, B, C);
impl_true_filter!(QuadFilter, A, B, C, D);
impl_true_filter!(PentaFilter, A, B, C, D, E);

macro_rules! impl_fn_filter {
    ($name:ident, $trait:ident, $doc:expr, $(($type_param:ident, $var:ident)),+) => {
        #[doc = $doc]
        pub struct $name<F> {
            f: F,
        }

        impl<F> $name<F> {
            /// Creates a new filter wrapping the given closure.
            #[inline]
            pub fn new(f: F) -> Self {
                Self { f }
            }
        }

        impl<S, $($type_param,)+ F> $trait<S, $($type_param),+> for $name<F>
        where
            F: Fn(&S, $(&$type_param),+) -> bool + Send + Sync,
        {
            #[inline]
            fn test(&self, solution: &S, $($var: &$type_param),+) -> bool {
                (self.f)(solution, $($var),+)
            }
        }
    };
}

impl_fn_filter!(
    FnUniFilter,
    UniFilter,
    "A uni-filter wrapping a closure.",
    (A, a)
);
impl_fn_filter!(
    FnBiFilter,
    BiFilter,
    "A bi-filter wrapping a closure.",
    (A, a),
    (B, b)
);
impl_fn_filter!(
    FnTriFilter,
    TriFilter,
    "A tri-filter wrapping a closure.",
    (A, a),
    (B, b),
    (C, c)
);
impl_fn_filter!(
    FnQuadFilter,
    QuadFilter,
    "A quad-filter wrapping a closure.",
    (A, a),
    (B, b),
    (C, c),
    (D, d)
);
impl_fn_filter!(
    FnPentaFilter,
    PentaFilter,
    "A penta-filter wrapping a closure.",
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e)
);
