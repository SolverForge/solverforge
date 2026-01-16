//! Zero-erasure filter composition for constraint streams.
//!
//! Filters are composed at compile time using nested generic types,
//! avoiding dynamic dispatch and Arc allocations.

mod adapters;
mod composition;
mod traits;
mod wrappers;

pub use adapters::{UniBiFilter, UniLeftBiFilter};
pub use composition::{AndBiFilter, AndPentaFilter, AndQuadFilter, AndTriFilter, AndUniFilter};
pub use traits::{BiFilter, PentaFilter, QuadFilter, TriFilter, UniFilter};
pub use wrappers::{FnBiFilter, FnPentaFilter, FnQuadFilter, FnTriFilter, FnUniFilter, TrueFilter};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true_filter() {
        let f = TrueFilter;
        assert!(UniFilter::<(), i32>::test(&f, &(), &42));
        assert!(BiFilter::<(), i32, i32>::test(&f, &(), &1, &2));
    }

    #[test]
    fn test_fn_uni_filter() {
        let f = FnUniFilter::new(|_: &(), x: &i32| *x > 10);
        assert!(f.test(&(), &15));
        assert!(!f.test(&(), &5));
    }

    #[test]
    fn test_fn_bi_filter() {
        let f = FnBiFilter::new(|_: &(), a: &i32, b: &i32| a + b > 10);
        assert!(f.test(&(), &7, &8));
        assert!(!f.test(&(), &3, &4));
    }

    #[test]
    fn test_and_uni_filter() {
        let f1 = FnUniFilter::new(|_: &(), x: &i32| *x > 5);
        let f2 = FnUniFilter::new(|_: &(), x: &i32| *x < 15);
        let combined = AndUniFilter::new(f1, f2);
        assert!(combined.test(&(), &10));
        assert!(!combined.test(&(), &3));
        assert!(!combined.test(&(), &20));
    }

    #[test]
    fn test_and_bi_filter() {
        let f1 = FnBiFilter::new(|_: &(), a: &i32, _b: &i32| *a > 0);
        let f2 = FnBiFilter::new(|_: &(), _a: &i32, b: &i32| *b > 0);
        let combined = AndBiFilter::new(f1, f2);
        assert!(combined.test(&(), &1, &2));
        assert!(!combined.test(&(), &-1, &2));
        assert!(!combined.test(&(), &1, &-2));
    }
}
