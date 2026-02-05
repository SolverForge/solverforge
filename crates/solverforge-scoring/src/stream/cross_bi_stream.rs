// Zero-erasure cross-bi-constraint stream for cross-entity join patterns.
//
// A `CrossBiConstraintStream` operates on pairs of entities from different
// collections, such as (Shift, Employee) joins. All type information is
// preserved at compile time - no Arc, no dyn, fully monomorphized.

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::cross_bi_incremental::IncrementalCrossBiConstraint;

use super::filter::{AndBiFilter, BiFilter, FnBiFilter, TrueFilter};
use super::flattened_bi_stream::FlattenedBiConstraintStream;

// Zero-erasure constraint stream over cross-entity pairs.
//
// `CrossBiConstraintStream` joins entities from collection A with collection B,
// accumulates filters on joined pairs, and finalizes into an
// `IncrementalCrossBiConstraint` via `penalize()` or `reward()`.
//
// All type parameters are concrete - no trait objects, no Arc allocations.
//
// # Type Parameters
//
// - `S` - Solution type
// - `A` - Entity type A (e.g., Shift)
// - `B` - Entity type B (e.g., Employee)
// - `K` - Join key type
// - `EA` - Extractor function for A entities
// - `EB` - Extractor function for B entities
// - `KA` - Key extractor for A
// - `KB` - Key extractor for B
// - `F` - Combined filter type
// - `Sc` - Score type
pub struct CrossBiConstraintStream<S, A, B, K, EA, EB, KA, KB, F, Sc>
where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    _phantom: PhantomData<(S, A, B, K, Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, Sc>
    CrossBiConstraintStream<S, A, B, K, EA, EB, KA, KB, TrueFilter, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Sc: Score + 'static,
{
    // Creates a new zero-erasure cross-bi constraint stream.
    //
    // This is typically called from `UniConstraintStream::join()`.
    pub fn new(extractor_a: EA, extractor_b: EB, key_a: KA, key_b: KB) -> Self {
        Self {
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, Sc> CrossBiConstraintStream<S, A, B, K, EA, EB, KA, KB, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    Sc: Score + 'static,
{
    // Creates a new cross-bi constraint stream with an initial filter.
    //
    // This is called from `UniConstraintStream::join()` when there are
    // accumulated filters on the uni-stream.
    pub fn new_with_filter(
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
    ) -> Self {
        Self {
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter,
            _phantom: PhantomData,
        }
    }

    // Adds a filter predicate to the stream.
    //
    // Multiple filters are combined with AND semantics at compile time.
    // Each filter adds a new type layer, preserving zero-erasure.
    //
    // # Example
    //
    // ```text
    // // Chain multiple filters on a cross-bi stream
    // let filtered = stream
    //     .filter(|shift, emp| shift.employee_id.is_some())
    //     .filter(|shift, emp| !emp.available);
    // ```
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> CrossBiConstraintStream<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        AndBiFilter<F, FnBiFilter<impl Fn(&S, &A, &B) -> bool + Send + Sync>>,
        Sc,
    >
    where
        P: Fn(&A, &B) -> bool + Send + Sync,
    {
        CrossBiConstraintStream {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: AndBiFilter::new(
                self.filter,
                FnBiFilter::new(move |_s: &S, a: &A, b: &B| predicate(a, b)),
            ),
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching pair with a fixed weight.
    pub fn penalize(
        self,
        weight: Sc,
    ) -> CrossBiConstraintBuilder<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        F,
        impl Fn(&A, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        let is_hard = weight
            .to_level_numbers()
            .first()
            .map(|&h| h != 0)
            .unwrap_or(false);
        CrossBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: move |_: &A, _: &B| weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching pair with a dynamic weight.
    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        CrossBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching pair with a dynamic weight, explicitly marked as hard.
    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        CrossBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    // Rewards each matching pair with a fixed weight.
    pub fn reward(
        self,
        weight: Sc,
    ) -> CrossBiConstraintBuilder<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        F,
        impl Fn(&A, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        let is_hard = weight
            .to_level_numbers()
            .first()
            .map(|&h| h != 0)
            .unwrap_or(false);
        CrossBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            impact_type: ImpactType::Reward,
            weight: move |_: &A, _: &B| weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    // Rewards each matching pair with a dynamic weight.
    pub fn reward_with<W>(
        self,
        weight_fn: W,
    ) -> CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        CrossBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            impact_type: ImpactType::Reward,
            weight: weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    // Rewards each matching pair with a dynamic weight, explicitly marked as hard.
    pub fn reward_hard_with<W>(
        self,
        weight_fn: W,
    ) -> CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        CrossBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            impact_type: ImpactType::Reward,
            weight: weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    // Expands items from entity B into separate (A, C) pairs with O(1) lookup.
    //
    // Pre-indexes C items by key for O(1) lookup on entity changes.
    //
    // # Arguments
    //
    // * `flatten` - Extracts a slice of C items from B
    // * `c_key_fn` - Extracts the index key from each C item
    // * `a_lookup_fn` - Extracts the lookup key from A (must match c_key type)
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
    // struct Employee {
    //     id: usize,
    //     unavailable_days: Vec<u32>,
    // }
    //
    // #[derive(Clone)]
    // struct Shift {
    //     employee_id: Option<usize>,
    //     day: u32,
    // }
    //
    // #[derive(Clone)]
    // struct Schedule {
    //     shifts: Vec<Shift>,
    //     employees: Vec<Employee>,
    // }
    //
    // // O(1) lookup by indexing unavailable_days by day number
    // let constraint = ConstraintFactory::<Schedule, SimpleScore>::new()
    //     .for_each(|s: &Schedule| &s.shifts)
    //     .join(
    //         |s: &Schedule| &s.employees,
    //         equal_bi(|shift: &Shift| shift.employee_id, |emp: &Employee| Some(emp.id)),
    //     )
    //     .flatten_last(
    //         |emp: &Employee| emp.unavailable_days.as_slice(),
    //         |day: &u32| *day,       // C → index key
    //         |shift: &Shift| shift.day,  // A → lookup key
    //     )
    //     .filter(|shift: &Shift, day: &u32| shift.employee_id.is_some() && shift.day == *day)
    //     .penalize(SimpleScore::of(1))
    //     .as_constraint("Unavailable employee");
    //
    // let schedule = Schedule {
    //     shifts: vec![
    //         Shift { employee_id: Some(0), day: 5 },
    //         Shift { employee_id: Some(0), day: 10 },
    //     ],
    //     employees: vec![
    //         Employee { id: 0, unavailable_days: vec![5, 15] },
    //     ],
    // };
    //
    // // Day 5 shift matches via O(1) lookup
    // assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
    // ```
    pub fn flatten_last<C, CK, Flatten, CKeyFn, ALookup>(
        self,
        flatten: Flatten,
        c_key_fn: CKeyFn,
        a_lookup_fn: ALookup,
    ) -> FlattenedBiConstraintStream<
        S,
        A,
        B,
        C,
        K,
        CK,
        EA,
        EB,
        KA,
        KB,
        Flatten,
        CKeyFn,
        ALookup,
        super::filter::TrueFilter,
        Sc,
    >
    where
        C: Clone + Send + Sync + 'static,
        CK: Eq + Hash + Clone + Send + Sync,
        Flatten: Fn(&B) -> &[C] + Send + Sync,
        CKeyFn: Fn(&C) -> CK + Send + Sync,
        ALookup: Fn(&A) -> CK + Send + Sync,
    {
        FlattenedBiConstraintStream::new(
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            flatten,
            c_key_fn,
            a_lookup_fn,
        )
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, Sc: Score> std::fmt::Debug
    for CrossBiConstraintStream<S, A, B, K, EA, EB, KA, KB, F, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrossBiConstraintStream").finish()
    }
}

// Zero-erasure builder for finalizing a cross-bi constraint.
pub struct CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(S, A, B, K, Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Clone + Send + Sync,
    EB: Fn(&S) -> &[B] + Clone + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    W: Fn(&A, &B) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    // Finalizes the builder into a zero-erasure `IncrementalCrossBiConstraint`.
    //
    // The resulting constraint has all types fully monomorphized with
    // key-based indexing for O(1) lookups.
    pub fn as_constraint(
        self,
        name: &str,
    ) -> IncrementalCrossBiConstraint<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        impl Fn(&S, &A, &B) -> bool + Send + Sync,
        impl Fn(&S, usize, usize) -> Sc + Send + Sync,
        Sc,
    > {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A, b: &B| filter.test(s, a, b);

        // Adapt user's Fn(&A, &B) -> Sc to internal Fn(&S, usize, usize) -> Sc
        let extractor_a = self.extractor_a.clone();
        let extractor_b = self.extractor_b.clone();
        let weight = self.weight;
        let adapted_weight = move |s: &S, a_idx: usize, b_idx: usize| {
            let entities_a = extractor_a(s);
            let entities_b = extractor_b(s);
            let a = &entities_a[a_idx];
            let b = &entities_b[b_idx];
            weight(a, b)
        };

        IncrementalCrossBiConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            combined_filter,
            adapted_weight,
            self.is_hard,
        )
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc: Score> std::fmt::Debug
    for CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrossBiConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
