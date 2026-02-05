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
impl_and_filter!(
    AndBiFilter,
    BiFilter,
    "Combines two bi-filters with AND semantics.",
    (A, a),
    (B, b)
);
impl_and_filter!(
    AndTriFilter,
    TriFilter,
    "Combines two tri-filters with AND semantics.",
    (A, a),
    (B, b),
    (C, c)
);
impl_and_filter!(
    AndQuadFilter,
    QuadFilter,
    "Combines two quad-filters with AND semantics.",
    (A, a),
    (B, b),
    (C, c),
    (D, d)
);
impl_and_filter!(
    AndPentaFilter,
    PentaFilter,
    "Combines two penta-filters with AND semantics.",
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e)
);
