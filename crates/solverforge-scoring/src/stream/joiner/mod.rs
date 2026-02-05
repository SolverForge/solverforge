// Joiner functions for constraint stream joins.
//
// Joiners define matching conditions between entities in different streams
// (cross-joins) or the same stream (self-joins).
//
// # Self-joins
//
// Use [`equal()`] with a single extractor for self-joins:
//
// ```
// use solverforge_scoring::stream::joiner::{Joiner, equal};
//
// #[derive(Clone)]
// struct Shift { employee_id: usize, start: i64, end: i64 }
//
// // Match shifts with the same employee
// let same_employee = equal(|s: &Shift| s.employee_id);
// assert!(same_employee.matches(
//     &Shift { employee_id: 1, start: 0, end: 8 },
//     &Shift { employee_id: 1, start: 8, end: 16 }
// ));
// ```
//
// # Cross-joins
//
// Use [`equal_bi()`] for cross-joins between different types:
//
// ```
// use solverforge_scoring::stream::joiner::{Joiner, equal_bi};
//
// struct Employee { id: usize }
// struct Shift { employee_id: Option<usize> }
//
// let by_id = equal_bi(
//     |shift: &Shift| shift.employee_id,
//     |emp: &Employee| Some(emp.id)
// );
// ```

mod comparison;
mod equal;
mod filtering;
mod overlapping;

pub use comparison::{
    greater_than, greater_than_or_equal, less_than, less_than_or_equal, GreaterThanJoiner,
    GreaterThanOrEqualJoiner, LessThanJoiner, LessThanOrEqualJoiner,
};
pub use equal::{equal, equal_bi, EqualJoiner};
pub use filtering::{filtering, FilteringJoiner};
pub use overlapping::{overlapping, OverlappingJoiner};

// A joiner defines matching conditions between two entities.
//
// Joiners are used in stream operations like `join()` and `join_self()`
// to determine which entity pairs should be matched.
//
// # Example
//
// ```
// use solverforge_scoring::stream::joiner::{Joiner, equal};
//
// let joiner = equal(|x: &i32| *x % 10);
// assert!(joiner.matches(&15, &25));  // Both end in 5
// assert!(!joiner.matches(&15, &26)); // 5 != 6
// ```
pub trait Joiner<A, B>: Send + Sync {
    // Returns true if the two entities should be joined.
    fn matches(&self, a: &A, b: &B) -> bool;

    // Combines this joiner with another using AND semantics.
    //
    // The resulting joiner matches only if both joiners match.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::joiner::{Joiner, equal};
    //
    // #[derive(Clone)]
    // struct Item { category: u32, priority: u32 }
    //
    // let same_category = equal(|i: &Item| i.category);
    // let same_priority = equal(|i: &Item| i.priority);
    //
    // let combined = same_category.and(same_priority);
    //
    // let a = Item { category: 1, priority: 5 };
    // let b = Item { category: 1, priority: 5 };
    // let c = Item { category: 1, priority: 3 };
    //
    // assert!(combined.matches(&a, &b));  // Same category AND priority
    // assert!(!combined.matches(&a, &c)); // Same category but different priority
    // ```
    fn and<J>(self, other: J) -> AndJoiner<Self, J>
    where
        Self: Sized,
        J: Joiner<A, B>,
    {
        AndJoiner {
            first: self,
            second: other,
        }
    }
}

// A joiner that combines two joiners with AND semantics.
//
// Created by calling `joiner.and(other)`.
pub struct AndJoiner<J1, J2> {
    first: J1,
    second: J2,
}

impl<A, B, J1, J2> Joiner<A, B> for AndJoiner<J1, J2>
where
    J1: Joiner<A, B>,
    J2: Joiner<A, B>,
{
    #[inline]
    fn matches(&self, a: &A, b: &B) -> bool {
        self.first.matches(a, b) && self.second.matches(a, b)
    }
}

// A joiner wrapping a closure for testing and simple cases.
pub struct FnJoiner<F> {
    f: F,
}

impl<F> FnJoiner<F> {
    // Creates a joiner from a closure.
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<A, B, F> Joiner<A, B> for FnJoiner<F>
where
    F: Fn(&A, &B) -> bool + Send + Sync,
{
    #[inline]
    fn matches(&self, a: &A, b: &B) -> bool {
        (self.f)(a, b)
    }
}
