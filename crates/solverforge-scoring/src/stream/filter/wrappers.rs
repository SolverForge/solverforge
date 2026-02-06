// Filter wrappers for closures and constant filters.

use super::traits::{BiFilter, PentaFilter, QuadFilter, TriFilter, UniFilter};

// A filter that always returns true.
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
impl_true_filter!(TriFilter, A, B, C);
impl_true_filter!(QuadFilter, A, B, C, D);
impl_true_filter!(PentaFilter, A, B, C, D, E);

// BiFilter has extra index params, so implement manually.
impl<S, A, B> BiFilter<S, A, B> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B, _a_idx: usize, _b_idx: usize) -> bool {
        true
    }
}

macro_rules! impl_fn_filter {
    ($name:ident, $trait:ident, $doc:expr, $(($type_param:ident, $var:ident)),+) => {
        #[doc = $doc]
        pub struct $name<F> {
            f: F,
        }

        impl<F> $name<F> {
            // Creates a new filter wrapping the given closure.
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
// FnBiFilter: manual impl because BiFilter has extra index params.
/// A bi-filter wrapping a closure.
pub struct FnBiFilter<F> {
    f: F,
}

impl<F> FnBiFilter<F> {
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, B, F> BiFilter<S, A, B> for FnBiFilter<F>
where
    F: Fn(&S, &A, &B) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, _a_idx: usize, _b_idx: usize) -> bool {
        (self.f)(solution, a, b)
    }
}
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
