// Zero-erasure uni-constraint stream for single-entity constraint patterns.
//
// A `UniConstraintStream` operates on a single entity type and supports
// filtering, weighting, and constraint finalization. All type information
// is preserved at compile time - no Arc, no dyn, fully monomorphized.

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::incremental::IncrementalUniConstraint;

use crate::constraint::if_exists::ExistenceMode;

use super::balance_stream::BalanceConstraintStream;
use super::bi_stream::BiConstraintStream;
use super::collector::UniCollector;
use super::cross_bi_stream::CrossBiConstraintStream;
use super::filter::{AndUniFilter, FnUniFilter, TrueFilter, UniFilter, UniLeftBiFilter};
use super::grouped_stream::GroupedConstraintStream;
use super::if_exists_stream::IfExistsStream;
use super::joiner::EqualJoiner;

// Zero-erasure constraint stream over a single entity type.
//
// `UniConstraintStream` accumulates filters and can be finalized into
// an `IncrementalUniConstraint` via `penalize()` or `reward()`.
//
// All type parameters are concrete - no trait objects, no Arc allocations
// in the hot path.
//
// # Type Parameters
//
// - `S` - Solution type
// - `A` - Entity type
// - `E` - Extractor function type
// - `F` - Combined filter type
// - `Sc` - Score type
pub struct UniConstraintStream<S, A, E, F, Sc>
where
    Sc: Score,
{
    extractor: E,
    filter: F,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Sc)>,
}

impl<S, A, E, Sc> UniConstraintStream<S, A, E, TrueFilter, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    Sc: Score + 'static,
{
    // Creates a new uni-constraint stream with the given extractor.
    pub fn new(extractor: E) -> Self {
        Self {
            extractor,
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, E, F, Sc> UniConstraintStream<S, A, E, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    F: UniFilter<S, A>,
    Sc: Score + 'static,
{
    // Adds a filter predicate to the stream.
    //
    // Multiple filters are combined with AND semantics at compile time.
    // Each filter adds a new type layer, preserving zero-erasure.
    //
    // To access related entities, use shadow variables on your entity type
    // (e.g., `#[inverse_relation_shadow_variable]`) rather than solution traversal.
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> UniConstraintStream<
        S,
        A,
        E,
        AndUniFilter<F, FnUniFilter<impl Fn(&S, &A) -> bool + Send + Sync>>,
        Sc,
    >
    where
        P: Fn(&A) -> bool + Send + Sync + 'static,
    {
        UniConstraintStream {
            extractor: self.extractor,
            filter: AndUniFilter::new(
                self.filter,
                FnUniFilter::new(move |_s: &S, a: &A| predicate(a)),
            ),
            _phantom: PhantomData,
        }
    }

    // Joins this stream with itself to create pairs (zero-erasure).
    //
    // Requires an `EqualJoiner` to enable key-based indexing for O(k) lookups.
    // For self-joins, pairs are ordered (i < j) to avoid duplicates.
    //
    // Any filters accumulated on this stream are applied to both entities
    // individually before the join.
    pub fn join_self<K, KA, KB>(
        self,
        joiner: EqualJoiner<KA, KB, K>,
    ) -> BiConstraintStream<
        S,
        A,
        K,
        E,
        impl Fn(&S, &A, usize) -> K + Send + Sync,
        UniLeftBiFilter<F, A>,
        Sc,
    >
    where
        A: Hash + PartialEq,
        K: Eq + Hash + Clone + Send + Sync,
        KA: Fn(&A) -> K + Send + Sync,
        KB: Fn(&A) -> K + Send + Sync,
    {
        let (key_extractor, _) = joiner.into_keys();

        // Wrap key_extractor to match the new KE: Fn(&S, &A, usize) -> K signature.
        // The static stream API doesn't need solution/index, so ignore them.
        let wrapped_ke = move |_s: &S, a: &A, _idx: usize| key_extractor(a);

        // Convert uni-filter to bi-filter that applies to left entity
        let bi_filter = UniLeftBiFilter::new(self.filter);

        BiConstraintStream::new_self_join_with_filter(self.extractor, wrapped_ke, bi_filter)
    }

    // Joins this stream with another collection to create cross-entity pairs (zero-erasure).
    //
    // Requires an `EqualJoiner` to enable key-based indexing for O(1) lookups.
    // Unlike `join_self` which pairs entities within the same collection,
    // `join` creates pairs from two different collections (e.g., Shift joined
    // with Employee).
    //
    // Any filters accumulated on this stream are applied to the A entity
    // before the join.
    pub fn join<B, EB, K, KA, KB>(
        self,
        extractor_b: EB,
        joiner: EqualJoiner<KA, KB, K>,
    ) -> CrossBiConstraintStream<S, A, B, K, E, EB, KA, KB, UniLeftBiFilter<F, B>, Sc>
    where
        B: Clone + Send + Sync + 'static,
        EB: Fn(&S) -> &[B] + Send + Sync,
        K: Eq + Hash + Clone + Send + Sync,
        KA: Fn(&A) -> K + Send + Sync,
        KB: Fn(&B) -> K + Send + Sync,
    {
        let (key_a, key_b) = joiner.into_keys();

        // Convert uni-filter to bi-filter that applies to left entity only
        let bi_filter = UniLeftBiFilter::new(self.filter);

        CrossBiConstraintStream::new_with_filter(
            self.extractor,
            extractor_b,
            key_a,
            key_b,
            bi_filter,
        )
    }

    // Groups entities by key and aggregates with a collector.
    //
    // Returns a zero-erasure `GroupedConstraintStream` that can be penalized
    // or rewarded based on the aggregated result for each group.
    pub fn group_by<K, KF, C>(
        self,
        key_fn: KF,
        collector: C,
    ) -> GroupedConstraintStream<S, A, K, E, KF, C, Sc>
    where
        K: Clone + Eq + Hash + Send + Sync + 'static,
        KF: Fn(&A) -> K + Send + Sync,
        C: UniCollector<A> + Send + Sync + 'static,
        C::Accumulator: Send + Sync,
        C::Result: Clone + Send + Sync,
    {
        GroupedConstraintStream::new(self.extractor, key_fn, collector)
    }

    // Creates a balance constraint that penalizes uneven distribution across groups.
    //
    // Unlike `group_by` which scores each group independently, `balance` computes
    // a GLOBAL standard deviation across all group counts and produces a single score.
    //
    // The `key_fn` returns `Option<K>` to allow skipping entities (e.g., unassigned shifts).
    // Any filters accumulated on this stream are also applied.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SimpleScore;
    //
    // #[derive(Clone)]
    // struct Shift { employee_id: Option<usize> }
    //
    // #[derive(Clone)]
    // struct Solution { shifts: Vec<Shift> }
    //
    // let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    //     .for_each(|s: &Solution| &s.shifts)
    //     .balance(|shift: &Shift| shift.employee_id)
    //     .penalize(SimpleScore::of(1000))
    //     .as_constraint("Balance workload");
    //
    // let solution = Solution {
    //     shifts: vec![
    //         Shift { employee_id: Some(0) },
    //         Shift { employee_id: Some(0) },
    //         Shift { employee_id: Some(0) },
    //         Shift { employee_id: Some(1) },
    //     ],
    // };
    //
    // // Employee 0: 3 shifts, Employee 1: 1 shift
    // // std_dev = 1.0, penalty = -1000
    // assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1000));
    // ```
    pub fn balance<K, KF>(self, key_fn: KF) -> BalanceConstraintStream<S, A, K, E, F, KF, Sc>
    where
        K: Clone + Eq + Hash + Send + Sync + 'static,
        KF: Fn(&A) -> Option<K> + Send + Sync,
    {
        BalanceConstraintStream::new(self.extractor, self.filter, key_fn)
    }

    // Filters A entities based on whether a matching B entity exists.
    //
    // Use this when the B collection needs filtering (e.g., only vacationing employees).
    // The `extractor_b` returns a `Vec<B>` to allow for filtering.
    //
    // Any filters accumulated on this stream are applied to A entities.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::joiner::equal_bi;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SimpleScore;
    //
    // #[derive(Clone)]
    // struct Shift { id: usize, employee_idx: Option<usize> }
    //
    // #[derive(Clone)]
    // struct Employee { id: usize, on_vacation: bool }
    //
    // #[derive(Clone)]
    // struct Schedule { shifts: Vec<Shift>, employees: Vec<Employee> }
    //
    // // Penalize shifts assigned to employees who are on vacation
    // let constraint = ConstraintFactory::<Schedule, SimpleScore>::new()
    //     .for_each(|s: &Schedule| s.shifts.as_slice())
    //     .filter(|shift: &Shift| shift.employee_idx.is_some())
    //     .if_exists_filtered(
    //         |s: &Schedule| s.employees.iter().filter(|e| e.on_vacation).cloned().collect(),
    //         equal_bi(
    //             |shift: &Shift| shift.employee_idx,
    //             |emp: &Employee| Some(emp.id),
    //         ),
    //     )
    //     .penalize(SimpleScore::of(1))
    //     .as_constraint("Vacation conflict");
    //
    // let schedule = Schedule {
    //     shifts: vec![
    //         Shift { id: 0, employee_idx: Some(0) },  // assigned to vacationing emp
    //         Shift { id: 1, employee_idx: Some(1) },  // assigned to working emp
    //         Shift { id: 2, employee_idx: None },     // unassigned (filtered out)
    //     ],
    //     employees: vec![
    //         Employee { id: 0, on_vacation: true },
    //         Employee { id: 1, on_vacation: false },
    //     ],
    // };
    //
    // // Only shift 0 matches (assigned to employee 0 who is on vacation)
    // assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
    // ```
    pub fn if_exists_filtered<B, EB, K, KA, KB>(
        self,
        extractor_b: EB,
        joiner: EqualJoiner<KA, KB, K>,
    ) -> IfExistsStream<S, A, B, K, E, EB, KA, KB, F, Sc>
    where
        B: Clone + Send + Sync + 'static,
        EB: Fn(&S) -> Vec<B> + Send + Sync,
        K: Eq + Hash + Clone + Send + Sync,
        KA: Fn(&A) -> K + Send + Sync,
        KB: Fn(&B) -> K + Send + Sync,
    {
        let (key_a, key_b) = joiner.into_keys();
        IfExistsStream::new(
            ExistenceMode::Exists,
            self.extractor,
            extractor_b,
            key_a,
            key_b,
            self.filter,
        )
    }

    // Filters A entities based on whether NO matching B entity exists.
    //
    // Use this when the B collection needs filtering.
    // The `extractor_b` returns a `Vec<B>` to allow for filtering.
    //
    // Any filters accumulated on this stream are applied to A entities.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::joiner::equal_bi;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SimpleScore;
    //
    // #[derive(Clone)]
    // struct Task { id: usize, assignee: Option<usize> }
    //
    // #[derive(Clone)]
    // struct Worker { id: usize, available: bool }
    //
    // #[derive(Clone)]
    // struct Schedule { tasks: Vec<Task>, workers: Vec<Worker> }
    //
    // // Penalize tasks assigned to workers who are not available
    // let constraint = ConstraintFactory::<Schedule, SimpleScore>::new()
    //     .for_each(|s: &Schedule| s.tasks.as_slice())
    //     .filter(|task: &Task| task.assignee.is_some())
    //     .if_not_exists_filtered(
    //         |s: &Schedule| s.workers.iter().filter(|w| w.available).cloned().collect(),
    //         equal_bi(
    //             |task: &Task| task.assignee,
    //             |worker: &Worker| Some(worker.id),
    //         ),
    //     )
    //     .penalize(SimpleScore::of(1))
    //     .as_constraint("Unavailable worker");
    //
    // let schedule = Schedule {
    //     tasks: vec![
    //         Task { id: 0, assignee: Some(0) },  // worker 0 is unavailable
    //         Task { id: 1, assignee: Some(1) },  // worker 1 is available
    //         Task { id: 2, assignee: None },     // unassigned (filtered out)
    //     ],
    //     workers: vec![
    //         Worker { id: 0, available: false },
    //         Worker { id: 1, available: true },
    //     ],
    // };
    //
    // // Task 0's worker (id=0) is NOT in the available workers list
    // assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
    // ```
    pub fn if_not_exists_filtered<B, EB, K, KA, KB>(
        self,
        extractor_b: EB,
        joiner: EqualJoiner<KA, KB, K>,
    ) -> IfExistsStream<S, A, B, K, E, EB, KA, KB, F, Sc>
    where
        B: Clone + Send + Sync + 'static,
        EB: Fn(&S) -> Vec<B> + Send + Sync,
        K: Eq + Hash + Clone + Send + Sync,
        KA: Fn(&A) -> K + Send + Sync,
        KB: Fn(&B) -> K + Send + Sync,
    {
        let (key_a, key_b) = joiner.into_keys();
        IfExistsStream::new(
            ExistenceMode::NotExists,
            self.extractor,
            extractor_b,
            key_a,
            key_b,
            self.filter,
        )
    }

    // Penalizes each matching entity with a fixed weight.
    pub fn penalize(
        self,
        weight: Sc,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Copy,
    {
        // Detect if this is a hard constraint by checking if hard level is non-zero
        let is_hard = weight
            .to_level_numbers()
            .first()
            .map(|&h| h != 0)
            .unwrap_or(false);
        UniConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: move |_: &A| weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching entity with a dynamic weight.
    //
    // Note: For dynamic weights, use `penalize_hard_with` to explicitly mark as a hard constraint,
    // since the weight function cannot be evaluated at build time.
    pub fn penalize_with<W>(self, weight_fn: W) -> UniConstraintBuilder<S, A, E, F, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        UniConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: false, // Can't detect at build time; use penalize_hard_with for hard constraints
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching entity with a dynamic weight, explicitly marked as a hard constraint.
    pub fn penalize_hard_with<W>(self, weight_fn: W) -> UniConstraintBuilder<S, A, E, F, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        UniConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    // Rewards each matching entity with a fixed weight.
    pub fn reward(
        self,
        weight: Sc,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Copy,
    {
        // Detect if this is a hard constraint by checking if hard level is non-zero
        let is_hard = weight
            .to_level_numbers()
            .first()
            .map(|&h| h != 0)
            .unwrap_or(false);
        UniConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            impact_type: ImpactType::Reward,
            weight: move |_: &A| weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    // Rewards each matching entity with a dynamic weight.
    //
    // Note: For dynamic weights, use `reward_hard_with` to explicitly mark as a hard constraint,
    // since the weight function cannot be evaluated at build time.
    pub fn reward_with<W>(self, weight_fn: W) -> UniConstraintBuilder<S, A, E, F, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        UniConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            impact_type: ImpactType::Reward,
            weight: weight_fn,
            is_hard: false, // Can't detect at build time; use reward_hard_with for hard constraints
            _phantom: PhantomData,
        }
    }

    // Rewards each matching entity with a dynamic weight, explicitly marked as a hard constraint.
    pub fn reward_hard_with<W>(self, weight_fn: W) -> UniConstraintBuilder<S, A, E, F, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        UniConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            impact_type: ImpactType::Reward,
            weight: weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, E, F, Sc: Score> std::fmt::Debug for UniConstraintStream<S, A, E, F, Sc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UniConstraintStream").finish()
    }
}

// Zero-erasure builder for finalizing a uni-constraint.
pub struct UniConstraintBuilder<S, A, E, F, W, Sc>
where
    Sc: Score,
{
    extractor: E,
    filter: F,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Sc)>,
}

impl<S, A, E, F, W, Sc> UniConstraintBuilder<S, A, E, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    F: UniFilter<S, A>,
    W: Fn(&A) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    // Finalizes the builder into a zero-erasure `IncrementalUniConstraint`.
    pub fn as_constraint(
        self,
        name: &str,
    ) -> IncrementalUniConstraint<S, A, E, impl Fn(&S, &A) -> bool + Send + Sync, W, Sc> {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A| filter.test(s, a);

        IncrementalUniConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor,
            combined_filter,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, E, F, W, Sc: Score> std::fmt::Debug for UniConstraintBuilder<S, A, E, F, W, Sc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UniConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
