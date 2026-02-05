// Core joiner traits and types for defining match conditions.

// A joiner defines matching conditions between two entities.
//
// Joiners are used in stream operations like `join()` and `join_self()`
// to determine which entity pairs should be matched.
pub trait Joiner<A, B>: Send + Sync {
    // Returns true if the two entities should be joined.
    fn matches(&self, a: &A, b: &B) -> bool;

    // Combines this joiner with another using AND semantics.
    //
    // The resulting joiner matches only if both joiners match.
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
