//! O(1) flattened bi-constraint stream.
//!
//! Provides O(1) lookup for flattened items by pre-indexing C items by key.

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::flattened_bi::FlattenedBiConstraint;

use super::filter::{AndBiFilter, BiFilter, FnBiFilter, TrueFilter};

/// O(1) flattened bi-constraint stream.
///
/// Pre-indexes C items by key for O(1) lookup.
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Entity type A (e.g., Shift)
/// - `B` - Entity type B (e.g., Employee)
/// - `C` - Flattened item type (e.g., NaiveDate)
/// - `K` - Join key type
/// - `CK` - C item key type for indexing
/// - `EA` - Extractor function for A entities
/// - `EB` - Extractor function for B entities
/// - `KA` - Key extractor for A
/// - `KB` - Key extractor for B
/// - `Flatten` - Function that extracts a slice from B
/// - `CKeyFn` - Function that extracts index key from C
/// - `ALookup` - Function that extracts lookup key from A
/// - `F` - Combined filter type over (A, C) pairs
/// - `Sc` - Score type
pub struct FlattenedBiConstraintStream<
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
    F,
    Sc,
> where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    flatten: Flatten,
    c_key_fn: CKeyFn,
    a_lookup_fn: ALookup,
    filter: F,
    _phantom: PhantomData<(S, A, B, C, K, CK, Sc)>,
}

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, Sc>
    FlattenedBiConstraintStream<
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
        TrueFilter,
        Sc,
    >
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    CK: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Flatten: Fn(&B) -> &[C] + Send + Sync,
    CKeyFn: Fn(&C) -> CK + Send + Sync,
    ALookup: Fn(&A) -> CK + Send + Sync,
    Sc: Score + 'static,
{
    /// Creates a new O(1) indexed flattened bi-constraint stream.
    ///
    /// This is called from `CrossBiConstraintStream::flatten_last_indexed()`.
    pub fn new(
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        flatten: Flatten,
        c_key_fn: CKeyFn,
        a_lookup_fn: ALookup,
    ) -> Self {
        Self {
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            flatten,
            c_key_fn,
            a_lookup_fn,
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, Sc>
    FlattenedBiConstraintStream<
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
        F,
        Sc,
    >
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    CK: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Flatten: Fn(&B) -> &[C] + Send + Sync,
    CKeyFn: Fn(&C) -> CK + Send + Sync,
    ALookup: Fn(&A) -> CK + Send + Sync,
    F: BiFilter<A, C>,
    Sc: Score + 'static,
{
    /// Adds a filter predicate to the stream.
    pub fn filter<P>(
        self,
        predicate: P,
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
        AndBiFilter<F, FnBiFilter<P>>,
        Sc,
    >
    where
        P: Fn(&A, &C) -> bool + Send + Sync,
    {
        FlattenedBiConstraintStream {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            flatten: self.flatten,
            c_key_fn: self.c_key_fn,
            a_lookup_fn: self.a_lookup_fn,
            filter: AndBiFilter::new(self.filter, FnBiFilter::new(predicate)),
            _phantom: PhantomData,
        }
    }

    /// Penalizes each matching (A, C) pair with a fixed weight.
    pub fn penalize(
        self,
        weight: Sc,
    ) -> FlattenedBiConstraintBuilder<
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
        F,
        impl Fn(&A, &C) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Clone,
    {
        let is_hard = weight
            .to_level_numbers()
            .first()
            .map(|&h| h != 0)
            .unwrap_or(false);
        FlattenedBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            flatten: self.flatten,
            c_key_fn: self.c_key_fn,
            a_lookup_fn: self.a_lookup_fn,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: move |_: &A, _: &C| weight.clone(),
            is_hard,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each matching (A, C) pair with a dynamic weight.
    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> FlattenedBiConstraintBuilder<
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
        F,
        W,
        Sc,
    >
    where
        W: Fn(&A, &C) -> Sc + Send + Sync,
    {
        FlattenedBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            flatten: self.flatten,
            c_key_fn: self.c_key_fn,
            a_lookup_fn: self.a_lookup_fn,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each matching (A, C) pair with a dynamic weight, explicitly marked as hard.
    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> FlattenedBiConstraintBuilder<
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
        F,
        W,
        Sc,
    >
    where
        W: Fn(&A, &C) -> Sc + Send + Sync,
    {
        FlattenedBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            flatten: self.flatten,
            c_key_fn: self.c_key_fn,
            a_lookup_fn: self.a_lookup_fn,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    /// Rewards each matching (A, C) pair with a fixed weight.
    pub fn reward(
        self,
        weight: Sc,
    ) -> FlattenedBiConstraintBuilder<
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
        F,
        impl Fn(&A, &C) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Clone,
    {
        let is_hard = weight
            .to_level_numbers()
            .first()
            .map(|&h| h != 0)
            .unwrap_or(false);
        FlattenedBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            flatten: self.flatten,
            c_key_fn: self.c_key_fn,
            a_lookup_fn: self.a_lookup_fn,
            filter: self.filter,
            impact_type: ImpactType::Reward,
            weight: move |_: &A, _: &C| weight.clone(),
            is_hard,
            _phantom: PhantomData,
        }
    }

    /// Rewards each matching (A, C) pair with a dynamic weight.
    pub fn reward_with<W>(
        self,
        weight_fn: W,
    ) -> FlattenedBiConstraintBuilder<
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
        F,
        W,
        Sc,
    >
    where
        W: Fn(&A, &C) -> Sc + Send + Sync,
    {
        FlattenedBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            flatten: self.flatten,
            c_key_fn: self.c_key_fn,
            a_lookup_fn: self.a_lookup_fn,
            filter: self.filter,
            impact_type: ImpactType::Reward,
            weight: weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, Sc: Score> std::fmt::Debug
    for FlattenedBiConstraintStream<
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
        F,
        Sc,
    >
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlattenedBiConstraintStream")
            .finish()
    }
}

/// Builder for finalizing an O(1) indexed flattened bi-constraint.
pub struct FlattenedBiConstraintBuilder<
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
    F,
    W,
    Sc,
> where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    flatten: Flatten,
    c_key_fn: CKeyFn,
    a_lookup_fn: ALookup,
    filter: F,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(S, A, B, C, K, CK, Sc)>,
}

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
    FlattenedBiConstraintBuilder<
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
        F,
        W,
        Sc,
    >
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    CK: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Flatten: Fn(&B) -> &[C] + Send + Sync,
    CKeyFn: Fn(&C) -> CK + Send + Sync,
    ALookup: Fn(&A) -> CK + Send + Sync,
    F: BiFilter<A, C>,
    W: Fn(&A, &C) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Finalizes the builder into an O(1) indexed constraint.
    pub fn as_constraint(
        self,
        name: &str,
    ) -> FlattenedBiConstraint<
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
        impl Fn(&A, &C) -> bool + Send + Sync,
        W,
        Sc,
    > {
        let filter = self.filter;
        let combined_filter = move |a: &A, c: &C| filter.test(a, c);

        FlattenedBiConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            self.flatten,
            self.c_key_fn,
            self.a_lookup_fn,
            combined_filter,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc: Score> std::fmt::Debug
    for FlattenedBiConstraintBuilder<
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
        F,
        W,
        Sc,
    >
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlattenedBiConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
