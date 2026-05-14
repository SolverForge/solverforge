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

impl<S, A, B> BiFilter<S, A, B> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B, _a_idx: usize, _b_idx: usize) -> bool {
        true
    }
}

impl<S, A, B, C> TriFilter<S, A, B, C> for TrueFilter {
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn test(
        &self,
        _: &S,
        _: &A,
        _: &B,
        _: &C,
        _a_idx: usize,
        _b_idx: usize,
        _c_idx: usize,
    ) -> bool {
        true
    }
}

impl<S, A, B, C, D> QuadFilter<S, A, B, C, D> for TrueFilter {
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn test(
        &self,
        _: &S,
        _: &A,
        _: &B,
        _: &C,
        _: &D,
        _a_idx: usize,
        _b_idx: usize,
        _c_idx: usize,
        _d_idx: usize,
    ) -> bool {
        true
    }
}

impl<S, A, B, C, D, E> PentaFilter<S, A, B, C, D, E> for TrueFilter {
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn test(
        &self,
        _: &S,
        _: &A,
        _: &B,
        _: &C,
        _: &D,
        _: &E,
        _a_idx: usize,
        _b_idx: usize,
        _c_idx: usize,
        _d_idx: usize,
        _e_idx: usize,
    ) -> bool {
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

macro_rules! filter_index_type {
    ($_idx:ident) => {
        usize
    };
}

impl_fn_filter!(
    FnUniFilter,
    UniFilter,
    "A uni-filter wrapping a closure.",
    (A, a)
);

macro_rules! impl_indexed_fn_filter {
    (
        $name:ident,
        $trait:ident,
        $doc:expr,
        entities = [$(($type_param:ident, $var:ident)),+],
        indices = [$($idx:ident),+]
    ) => {
        #[doc = $doc]
        pub struct $name<F> {
            f: F,
        }

        impl<F> $name<F> {
            #[inline]
            pub fn new(f: F) -> Self {
                Self { f }
            }
        }

        impl<S, $($type_param,)+ F> $trait<S, $($type_param),+> for $name<F>
        where
            F: Fn(&S, $(&$type_param),+, $(filter_index_type!($idx)),+) -> bool + Send + Sync,
        {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            fn test(
                &self,
                solution: &S,
                $($var: &$type_param,)+
                $($idx: usize),+
            ) -> bool {
                (self.f)(solution, $($var),+, $($idx),+)
            }
        }
    };
}

impl_indexed_fn_filter!(
    FnBiFilter,
    BiFilter,
    "A bi-filter wrapping an index-aware closure.",
    entities = [(A, a), (B, b)],
    indices = [a_idx, b_idx]
);
impl_indexed_fn_filter!(
    FnTriFilter,
    TriFilter,
    "A tri-filter wrapping an index-aware closure.",
    entities = [(A, a), (B, b), (C, c)],
    indices = [a_idx, b_idx, c_idx]
);
impl_indexed_fn_filter!(
    FnQuadFilter,
    QuadFilter,
    "A quad-filter wrapping an index-aware closure.",
    entities = [(A, a), (B, b), (C, c), (D, d)],
    indices = [a_idx, b_idx, c_idx, d_idx]
);
impl_indexed_fn_filter!(
    FnPentaFilter,
    PentaFilter,
    "A penta-filter wrapping an index-aware closure.",
    entities = [(A, a), (B, b), (C, c), (D, d), (E, e)],
    indices = [a_idx, b_idx, c_idx, d_idx, e_idx]
);
