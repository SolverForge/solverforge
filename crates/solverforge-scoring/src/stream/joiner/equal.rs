// Equal joiner for matching on property equality.

use std::marker::PhantomData;

use super::Joiner;

// Creates a joiner that matches when a property is equal on both sides.
//
// This is the primary joiner for self-joins where you're matching
// entities from the same collection on a shared property.
//
// # Example
//
// ```
// use solverforge_scoring::stream::joiner::{Joiner, equal};
//
// #[derive(Clone)]
// struct Shift { employee_id: Option<usize>, start: i64 }
//
// // Match shifts with the same employee (self-join)
// let same_employee = equal(|s: &Shift| s.employee_id);
//
// let a = Shift { employee_id: Some(5), start: 0 };
// let b = Shift { employee_id: Some(5), start: 8 };
// let c = Shift { employee_id: Some(3), start: 16 };
//
// assert!(same_employee.matches(&a, &b));
// assert!(!same_employee.matches(&a, &c));
// ```
pub fn equal<A, T, F>(key: F) -> EqualJoiner<F, F, T>
where
    T: PartialEq,
    F: Fn(&A) -> T + Clone + Send + Sync,
{
    EqualJoiner {
        left: key.clone(),
        right: key,
        _phantom: PhantomData,
    }
}

// Creates a joiner that matches when extracted values are equal.
//
// Use this for cross-joins between different entity types.
//
// # Example
//
// ```
// use solverforge_scoring::stream::joiner::{Joiner, equal_bi};
//
// #[derive(Clone)]
// struct Employee { id: usize, department: String }
// #[derive(Clone)]
// struct Task { assigned_to: usize, name: String }
//
// // Match employees to their assigned tasks
// let by_id = equal_bi(
//     |e: &Employee| e.id,
//     |t: &Task| t.assigned_to
// );
//
// let emp = Employee { id: 5, department: "Engineering".into() };
// let task1 = Task { assigned_to: 5, name: "Review".into() };
// let task2 = Task { assigned_to: 3, name: "Test".into() };
//
// assert!(by_id.matches(&emp, &task1));
// assert!(!by_id.matches(&emp, &task2));
// ```
pub fn equal_bi<A, B, T, Fa, Fb>(left: Fa, right: Fb) -> EqualJoiner<Fa, Fb, T>
where
    T: PartialEq,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    EqualJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

// A joiner that matches when extracted values are equal.
//
// Created by the [`equal()`] or [`equal_bi()`] functions.
pub struct EqualJoiner<Fa, Fb, T> {
    left: Fa,
    right: Fb,
    _phantom: PhantomData<fn() -> T>,
}

impl<Fa, Fb, T> EqualJoiner<Fa, Fb, T> {
    // Extracts the join key from an A entity.
    #[inline]
    pub fn key_a<A>(&self, a: &A) -> T
    where
        Fa: Fn(&A) -> T,
    {
        (self.left)(a)
    }

    // Extracts the join key from a B entity.
    #[inline]
    pub fn key_b<B>(&self, b: &B) -> T
    where
        Fb: Fn(&B) -> T,
    {
        (self.right)(b)
    }

    // Consumes the joiner and returns the key extractors.
    //
    // This is useful for zero-erasure constraint creation where
    // the key extractors need to be stored as concrete types.
    #[inline]
    pub fn into_keys(self) -> (Fa, Fb) {
        (self.left, self.right)
    }

    // Returns references to the key extractors.
    #[inline]
    pub fn key_extractors(&self) -> (&Fa, &Fb) {
        (&self.left, &self.right)
    }
}

impl<A, B, T, Fa, Fb> Joiner<A, B> for EqualJoiner<Fa, Fb, T>
where
    T: PartialEq,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    #[inline]
    fn matches(&self, a: &A, b: &B) -> bool {
        (self.left)(a) == (self.right)(b)
    }
}
