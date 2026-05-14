// Filter composition types for combining filters with AND semantics.

use super::traits::{BiFilter, PentaFilter, QuadFilter, TriFilter, UniFilter};

macro_rules! impl_and_filter {
    ($name:ident, $trait:ident, $doc:expr, $(($type_param:ident, $var:ident)),+) => {
        #[doc = $doc]
        pub struct $name<F1, F2> {
            first: F1,
            second: F2,
        }

        impl<F1, F2> $name<F1, F2> {
            // Creates a new combined filter from two filters.
            #[inline]
            pub fn new(first: F1, second: F2) -> Self {
                Self { first, second }
            }
        }

        impl<S, $($type_param,)+ F1, F2> $trait<S, $($type_param),+> for $name<F1, F2>
        where
            F1: $trait<S, $($type_param),+>,
            F2: $trait<S, $($type_param),+>,
        {
            #[inline]
            fn test(&self, solution: &S, $($var: &$type_param),+) -> bool {
                self.first.test(solution, $($var),+) && self.second.test(solution, $($var),+)
            }
        }
    };
}

impl_and_filter!(
    AndUniFilter,
    UniFilter,
    "Combines two uni-filters with AND semantics.",
    (A, a)
);

macro_rules! impl_indexed_and_filter {
    (
        $name:ident,
        $trait:ident,
        $doc:expr,
        entities = [$(($type_param:ident, $var:ident)),+],
        indices = [$($idx:ident),+]
    ) => {
        #[doc = $doc]
        pub struct $name<F1, F2> {
            first: F1,
            second: F2,
        }

        impl<F1, F2> $name<F1, F2> {
            #[inline]
            pub fn new(first: F1, second: F2) -> Self {
                Self { first, second }
            }
        }

        impl<S, $($type_param,)+ F1, F2> $trait<S, $($type_param),+> for $name<F1, F2>
        where
            F1: $trait<S, $($type_param),+>,
            F2: $trait<S, $($type_param),+>,
        {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            fn test(
                &self,
                solution: &S,
                $($var: &$type_param,)+
                $($idx: usize),+
            ) -> bool {
                self.first.test(solution, $($var),+, $($idx),+)
                    && self.second.test(solution, $($var),+, $($idx),+)
            }
        }
    };
}

impl_indexed_and_filter!(
    AndBiFilter,
    BiFilter,
    "Combines two bi-filters with AND semantics.",
    entities = [(A, a), (B, b)],
    indices = [a_idx, b_idx]
);
impl_indexed_and_filter!(
    AndTriFilter,
    TriFilter,
    "Combines two tri-filters with AND semantics.",
    entities = [(A, a), (B, b), (C, c)],
    indices = [a_idx, b_idx, c_idx]
);
impl_indexed_and_filter!(
    AndQuadFilter,
    QuadFilter,
    "Combines two quad-filters with AND semantics.",
    entities = [(A, a), (B, b), (C, c), (D, d)],
    indices = [a_idx, b_idx, c_idx, d_idx]
);
impl_indexed_and_filter!(
    AndPentaFilter,
    PentaFilter,
    "Combines two penta-filters with AND semantics.",
    entities = [(A, a), (B, b), (C, c), (D, d), (E, e)],
    indices = [a_idx, b_idx, c_idx, d_idx, e_idx]
);
