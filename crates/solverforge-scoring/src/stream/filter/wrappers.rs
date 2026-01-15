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
    ($name:ident, $trait:ident, $($param:ident),+) => {
        pub struct $name<F> {
            f: F,
        }

        impl<F> $name<F> {
            #[inline]
            pub fn new(f: F) -> Self {
                Self { f }
            }
        }

        impl<S, $($param,)+ F> $trait<S, $($param),+> for $name<F>
        where
            F: Fn(&S, $(&$param),+) -> bool + Send + Sync,
        {
            #[inline]
            fn test(&self, solution: &S, $($param: &$param),+) -> bool {
                (self.f)(solution, $($param),+)
            }
        }
    };
}

/// A uni-filter wrapping a closure.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::filter::{FnUniFilter, UniFilter};
///
/// struct Schedule {
///     max_hours: i32,
/// }
///
/// let filter = FnUniFilter::new(|schedule: &Schedule, hours: &i32| {
///     *hours <= schedule.max_hours
/// });
///
/// let schedule = Schedule { max_hours: 40 };
/// assert!(filter.test(&schedule, &35));
/// assert!(!filter.test(&schedule, &50));
/// ```
impl_fn_filter!(FnUniFilter, UniFilter, A);

/// A bi-filter wrapping a closure.
impl_fn_filter!(FnBiFilter, BiFilter, A, B);

/// A tri-filter wrapping a closure.
impl_fn_filter!(FnTriFilter, TriFilter, A, B, C);

/// A quad-filter wrapping a closure.
impl_fn_filter!(FnQuadFilter, QuadFilter, A, B, C, D);

/// A penta-filter wrapping a closure.
impl_fn_filter!(FnPentaFilter, PentaFilter, A, B, C, D, E);
