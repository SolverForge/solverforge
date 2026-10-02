/* Zero-erasure cross-tri-constraint stream for three-source join patterns.

A `Tri` extends a cross-joined (A, B) pair with a third source C, such as
(Shift, Employee, DayOff) joins reached through
`.join((day_offs, equal_bi(...)))` on a cross Bi stream. All type
information is preserved at compile time - no Arc, no dyn, fully
monomorphized.
*/

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::{AndTriFilter, FnTriFilter, TriFilter, TrueFilter};
use super::super::weighting_support::ConstraintWeight;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::cross_tri_incremental::TripleWeight;

pub struct Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, Sc>
where
    Sc: Score,
{
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) extractor_c: EC,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) key_c: KC,
    pub(super) filter: F,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> C,
        fn() -> K,
        fn() -> Sc,
    )>,
}

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, Sc>
    Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, TrueFilter, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    KC: Fn(&C) -> K + Send + Sync,
    Sc: Score + 'static,
{
    pub fn new(
        extractor_a: EA,
        extractor_b: EB,
        extractor_c: EC,
        key_a: KA,
        key_b: KB,
        key_c: KC,
    ) -> Self {
        Self {
            extractor_a,
            extractor_b,
            extractor_c,
            key_a,
            key_b,
            key_c,
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, Sc> Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    KC: Fn(&C) -> K + Send + Sync,
    F: TriFilter<S, A, B, C>,
    Sc: Score + 'static,
{
    pub fn new_with_filter(
        extractor_a: EA,
        extractor_b: EB,
        extractor_c: EC,
        key_a: KA,
        key_b: KB,
        key_c: KC,
        filter: F,
    ) -> Self {
        Self {
            extractor_a,
            extractor_b,
            extractor_c,
            key_a,
            key_b,
            key_c,
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, Sc> Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    KC: Fn(&C) -> K + Send + Sync,
    F: TriFilter<S, A, B, C>,
    Sc: Score + 'static,
{
    /* Adds a filter predicate to the stream. */
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> Tri<
        S,
        A,
        B,
        C,
        K,
        EA,
        EB,
        EC,
        KA,
        KB,
        KC,
        AndTriFilter<
            F,
            FnTriFilter<impl Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync>,
        >,
        Sc,
    >
    where
        P: Fn(&A, &B, &C) -> bool + Send + Sync,
    {
        Tri {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            extractor_c: self.extractor_c,
            key_a: self.key_a,
            key_b: self.key_b,
            key_c: self.key_c,
            filter: AndTriFilter::new(
                self.filter,
                FnTriFilter::new(
                    move |_s: &S,
                          a: &A,
                          b: &B,
                          c: &C,
                          _a_idx: usize,
                          _b_idx: usize,
                          _c_idx: usize| { predicate(a, b, c) },
                ),
            ),
            _phantom: PhantomData,
        }
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> Builder<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
    where
        W: Fn(&A, &B, &C) -> Sc + Send + Sync,
    {
        Builder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            extractor_c: self.extractor_c,
            key_a: self.key_a,
            key_b: self.key_b,
            key_c: self.key_c,
            filter: self.filter,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> Builder<
        S,
        A,
        B,
        C,
        K,
        EA,
        EB,
        EC,
        KA,
        KB,
        KC,
        F,
        impl Fn(&A, &B, &C) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w B, &'w C), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |a: &A, b: &B, c: &C| weight.score((a, b, c)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> Builder<
        S,
        A,
        B,
        C,
        K,
        EA,
        EB,
        EC,
        KA,
        KB,
        KC,
        F,
        impl Fn(&A, &B, &C) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w B, &'w C), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |a: &A, b: &B, c: &C| weight.score((a, b, c)),
            is_hard,
        )
    }
}

// Zero-erasure builder for finalizing a cross-tri constraint.
pub struct Builder<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    extractor_c: EC,
    key_a: KA,
    key_b: KB,
    key_c: KC,
    filter: F,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> C,
        fn() -> K,
        fn() -> Sc,
    )>,
}

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
    Builder<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    KC: Fn(&C) -> K + Send + Sync,
    F: TriFilter<S, A, B, C>,
    W: Fn(&A, &B, &C) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> crate::constraint::cross_tri_incremental::Tri<
        S,
        A,
        B,
        C,
        K,
        EA,
        EB,
        EC,
        KA,
        KB,
        KC,
        impl Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync,
        TripleWeight<W>,
        Sc,
    > {
        let filter = self.filter;
        let combined_filter =
            move |s: &S, a: &A, b: &B, c: &C, a_idx: usize, b_idx: usize, c_idx: usize| {
                filter.test(s, a, b, c, a_idx, b_idx, c_idx)
            };

        crate::constraint::cross_tri_incremental::Tri::new_triple_weight(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor_a,
            self.extractor_b,
            self.extractor_c,
            self.key_a,
            self.key_b,
            self.key_c,
            combined_filter,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc: Score> std::fmt::Debug
    for Builder<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
