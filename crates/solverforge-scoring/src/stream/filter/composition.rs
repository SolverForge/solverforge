//! Filter composition types for combining filters with AND semantics.

use super::traits::{BiFilter, PentaFilter, QuadFilter, TriFilter, UniFilter};

macro_rules! impl_and_filter {
    ($name:ident, $trait:ident, $($param:ident),+) => {
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

        impl<S, $($param,)+ F1, F2> $trait<S, $($param),+> for $name<F1, F2>
        where
            F1: $trait<S, $($param),+>,
            F2: $trait<S, $($param),+>,
        {
            #[inline]
            fn test(&self, solution: &S, $($param: &$param),+) -> bool {
                self.first.test(solution, $($param),+) && self.second.test(solution, $($param),+)
            }
        }
    };
}

/// Combines two uni-filters with AND semantics.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::filter::{FnUniFilter, AndUniFilter, UniFilter};
///
/// let f1 = FnUniFilter::new(|_: &(), x: &i32| *x > 5);
/// let f2 = FnUniFilter::new(|_: &(), x: &i32| *x < 15);
/// let combined = AndUniFilter::new(f1, f2);
///
/// assert!(combined.test(&(), &10));
/// assert!(!combined.test(&(), &3));
/// assert!(!combined.test(&(), &20));
/// ```
impl_and_filter!(AndUniFilter, UniFilter, A);

/// Combines two bi-filters with AND semantics.
impl_and_filter!(AndBiFilter, BiFilter, A, B);

/// Combines two tri-filters with AND semantics.
impl_and_filter!(AndTriFilter, TriFilter, A, B, C);

/// Combines two quad-filters with AND semantics.
impl_and_filter!(AndQuadFilter, QuadFilter, A, B, C, D);

/// Combines two penta-filters with AND semantics.
impl_and_filter!(AndPentaFilter, PentaFilter, A, B, C, D, E);
