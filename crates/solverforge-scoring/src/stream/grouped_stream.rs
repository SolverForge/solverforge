// Zero-erasure grouped constraint stream for group-by constraint patterns.
//
// A `GroupedConstraintStream` operates on groups of entities and supports
// filtering, weighting, and constraint finalization.
// All type information is preserved at compile time - no Arc, no dyn.

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::collector::UniCollector;
use super::complemented_stream::ComplementedConstraintStream;
use crate::constraint::grouped::GroupedUniConstraint;

// Zero-erasure constraint stream over grouped entities.
//
// `GroupedConstraintStream` is created by `UniConstraintStream::group_by()`
// and operates on (key, collector_result) tuples.
//
// All type parameters are concrete - no trait objects, no Arc allocations.
//
// # Type Parameters
//
// - `S` - Solution type
// - `A` - Entity type
// - `K` - Group key type
// - `E` - Extractor function for entities
// - `KF` - Key function
// - `C` - Collector type
// - `Sc` - Score type
//
// # Example
//
// ```
// use solverforge_scoring::stream::ConstraintFactory;
// use solverforge_scoring::stream::collector::count;
// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
// use solverforge_core::score::SimpleScore;
//
// #[derive(Clone, Hash, PartialEq, Eq)]
// struct Shift { employee_id: usize }
//
// #[derive(Clone)]
// struct Solution { shifts: Vec<Shift> }
//
// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
//     .for_each(|s: &Solution| &s.shifts)
//     .group_by(|shift: &Shift| shift.employee_id, count())
//     .penalize_with(|count: &usize| SimpleScore::of((*count * *count) as i64))
//     .as_constraint("Balanced workload");
//
// let solution = Solution {
//     shifts: vec![
//         Shift { employee_id: 1 },
//         Shift { employee_id: 1 },
//         Shift { employee_id: 1 },
//         Shift { employee_id: 2 },
//     ],
// };
//
// // Employee 1: 3² = 9, Employee 2: 1² = 1, Total: -10
// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-10));
// ```
pub struct GroupedConstraintStream<S, A, K, E, KF, C, Sc>
where
    Sc: Score,
{
    extractor: E,
    key_fn: KF,
    collector: C,
    _phantom: PhantomData<(S, A, K, Sc)>,
}

impl<S, A, K, E, KF, C, Sc> GroupedConstraintStream<S, A, K, E, KF, C, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    KF: Fn(&A) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    Sc: Score + 'static,
{
    // Creates a new zero-erasure grouped constraint stream.
    pub(crate) fn new(extractor: E, key_fn: KF, collector: C) -> Self {
        Self {
            extractor,
            key_fn,
            collector,
            _phantom: PhantomData,
        }
    }

    // Penalizes each group with a weight based on the collector result.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::collector::count;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SimpleScore;
    //
    // #[derive(Clone, Hash, PartialEq, Eq)]
    // struct Task { priority: u32 }
    //
    // #[derive(Clone)]
    // struct Solution { tasks: Vec<Task> }
    //
    // let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    //     .for_each(|s: &Solution| &s.tasks)
    //     .group_by(|t: &Task| t.priority, count())
    //     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
    //     .as_constraint("Priority distribution");
    //
    // let solution = Solution {
    //     tasks: vec![
    //         Task { priority: 1 },
    //         Task { priority: 1 },
    //         Task { priority: 2 },
    //     ],
    // };
    //
    // // Priority 1: 2 tasks, Priority 2: 1 task, Total: -3
    // assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-3));
    // ```
    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedConstraintBuilder<S, A, K, E, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedConstraintBuilder {
            extractor: self.extractor,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type: ImpactType::Penalty,
            weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    // Penalizes each group with a weight, explicitly marked as hard constraint.
    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedConstraintBuilder<S, A, K, E, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedConstraintBuilder {
            extractor: self.extractor,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type: ImpactType::Penalty,
            weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    // Rewards each group with a weight based on the collector result.
    pub fn reward_with<W>(self, weight_fn: W) -> GroupedConstraintBuilder<S, A, K, E, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedConstraintBuilder {
            extractor: self.extractor,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type: ImpactType::Reward,
            weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    // Rewards each group with a weight, explicitly marked as hard constraint.
    pub fn reward_hard_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedConstraintBuilder<S, A, K, E, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedConstraintBuilder {
            extractor: self.extractor,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type: ImpactType::Reward,
            weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    // Adds complement entities with default values for missing keys.
    //
    // This ensures all keys from the complement source are represented,
    // using the grouped value if present, or the default value otherwise.
    //
    // **Note:** The key function for A entities wraps the original key to
    // return `Some(K)`. For filtering (skipping entities without valid keys),
    // use `complement_filtered` instead.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::collector::count;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SimpleScore;
    //
    // #[derive(Clone, Hash, PartialEq, Eq)]
    // struct Employee { id: usize }
    //
    // #[derive(Clone, Hash, PartialEq, Eq)]
    // struct Shift { employee_id: usize }
    //
    // #[derive(Clone)]
    // struct Schedule {
    //     employees: Vec<Employee>,
    //     shifts: Vec<Shift>,
    // }
    //
    // // Count shifts per employee, including employees with 0 shifts
    // let constraint = ConstraintFactory::<Schedule, SimpleScore>::new()
    //     .for_each(|s: &Schedule| &s.shifts)
    //     .group_by(|shift: &Shift| shift.employee_id, count())
    //     .complement(
    //         |s: &Schedule| s.employees.as_slice(),
    //         |emp: &Employee| emp.id,
    //         |_emp: &Employee| 0usize,
    //     )
    //     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
    //     .as_constraint("Shift count");
    //
    // let schedule = Schedule {
    //     employees: vec![Employee { id: 0 }, Employee { id: 1 }],
    //     shifts: vec![
    //         Shift { employee_id: 0 },
    //         Shift { employee_id: 0 },
    //     ],
    // };
    //
    // // Employee 0: 2, Employee 1: 0 → Total: -2
    // assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-2));
    // ```
    pub fn complement<B, EB, KB, D>(
        self,
        extractor_b: EB,
        key_b: KB,
        default_fn: D,
    ) -> ComplementedConstraintStream<
        S,
        A,
        B,
        K,
        E,
        EB,
        impl Fn(&A) -> Option<K> + Send + Sync,
        KB,
        C,
        D,
        Sc,
    >
    where
        B: Clone + Send + Sync + 'static,
        EB: Fn(&S) -> &[B] + Send + Sync,
        KB: Fn(&B) -> K + Send + Sync,
        D: Fn(&B) -> C::Result + Send + Sync,
    {
        let key_fn = self.key_fn;
        let wrapped_key_fn = move |a: &A| Some((key_fn)(a));
        ComplementedConstraintStream::new(
            self.extractor,
            extractor_b,
            wrapped_key_fn,
            key_b,
            self.collector,
            default_fn,
        )
    }

    // Adds complement entities with a custom key function for filtering.
    //
    // Like `complement`, but allows providing a custom key function for A entities
    // that returns `Option<K>`. Entities returning `None` are skipped.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::collector::count;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SimpleScore;
    //
    // #[derive(Clone, Hash, PartialEq, Eq)]
    // struct Employee { id: usize }
    //
    // #[derive(Clone, Hash, PartialEq, Eq)]
    // struct Shift { employee_id: Option<usize> }
    //
    // #[derive(Clone)]
    // struct Schedule {
    //     employees: Vec<Employee>,
    //     shifts: Vec<Shift>,
    // }
    //
    // // Count shifts per employee, skipping unassigned shifts
    // // The group_by key is ignored; complement_with_key provides its own
    // let constraint = ConstraintFactory::<Schedule, SimpleScore>::new()
    //     .for_each(|s: &Schedule| &s.shifts)
    //     .group_by(|_shift: &Shift| 0usize, count())  // Placeholder key, will be overridden
    //     .complement_with_key(
    //         |s: &Schedule| s.employees.as_slice(),
    //         |shift: &Shift| shift.employee_id,  // Option<usize>
    //         |emp: &Employee| emp.id,            // usize
    //         |_emp: &Employee| 0usize,
    //     )
    //     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
    //     .as_constraint("Shift count");
    //
    // let schedule = Schedule {
    //     employees: vec![Employee { id: 0 }, Employee { id: 1 }],
    //     shifts: vec![
    //         Shift { employee_id: Some(0) },
    //         Shift { employee_id: Some(0) },
    //         Shift { employee_id: None },  // Skipped
    //     ],
    // };
    //
    // // Employee 0: 2, Employee 1: 0 → Total: -2
    // assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-2));
    // ```
    pub fn complement_with_key<B, EB, KA2, KB, D>(
        self,
        extractor_b: EB,
        key_a: KA2,
        key_b: KB,
        default_fn: D,
    ) -> ComplementedConstraintStream<S, A, B, K, E, EB, KA2, KB, C, D, Sc>
    where
        B: Clone + Send + Sync + 'static,
        EB: Fn(&S) -> &[B] + Send + Sync,
        KA2: Fn(&A) -> Option<K> + Send + Sync,
        KB: Fn(&B) -> K + Send + Sync,
        D: Fn(&B) -> C::Result + Send + Sync,
    {
        ComplementedConstraintStream::new(
            self.extractor,
            extractor_b,
            key_a,
            key_b,
            self.collector,
            default_fn,
        )
    }
}

impl<S, A, K, E, KF, C, Sc: Score> std::fmt::Debug
    for GroupedConstraintStream<S, A, K, E, KF, C, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedConstraintStream").finish()
    }
}

// Zero-erasure builder for finalizing a grouped constraint.
pub struct GroupedConstraintBuilder<S, A, K, E, KF, C, W, Sc>
where
    Sc: Score,
{
    extractor: E,
    key_fn: KF,
    collector: C,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    _phantom: PhantomData<(S, A, K, Sc)>,
}

impl<S, A, K, E, KF, C, W, Sc> GroupedConstraintBuilder<S, A, K, E, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    KF: Fn(&A) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    // Finalizes the builder into a zero-erasure `GroupedUniConstraint`.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::collector::count;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SimpleScore;
    //
    // #[derive(Clone, Hash, PartialEq, Eq)]
    // struct Item { category: u32 }
    //
    // #[derive(Clone)]
    // struct Solution { items: Vec<Item> }
    //
    // let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    //     .for_each(|s: &Solution| &s.items)
    //     .group_by(|i: &Item| i.category, count())
    //     .penalize_with(|n: &usize| SimpleScore::of(*n as i64))
    //     .as_constraint("Category penalty");
    //
    // assert_eq!(constraint.name(), "Category penalty");
    // ```
    pub fn as_constraint(self, name: &str) -> GroupedUniConstraint<S, A, K, E, KF, C, W, Sc> {
        GroupedUniConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor,
            self.key_fn,
            self.collector,
            self.weight_fn,
            self.is_hard,
        )
    }
}

impl<S, A, K, E, KF, C, W, Sc: Score> std::fmt::Debug
    for GroupedConstraintBuilder<S, A, K, E, KF, C, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
