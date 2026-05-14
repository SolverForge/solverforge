use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::flattened_bi::FlattenedBiConstraint;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::BiFilter;

// Builder for finalizing an O(1) indexed flattened bi-constraint.
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
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) flatten: Flatten,
    pub(super) c_key_fn: CKeyFn,
    pub(super) a_lookup_fn: ALookup,
    pub(super) filter: F,
    pub(super) impact_type: ImpactType,
    pub(super) weight: W,
    pub(super) is_hard: bool,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> C,
        fn() -> K,
        fn() -> CK,
        fn() -> Sc,
    )>,
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
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Flatten: Fn(&B) -> &[C] + Send + Sync,
    CKeyFn: Fn(&C) -> CK + Send + Sync,
    ALookup: Fn(&A) -> CK + Send + Sync,
    F: BiFilter<S, A, C>,
    W: Fn(&A, &C) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
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
        impl Fn(&S, &A, &C, usize, usize) -> bool + Send + Sync,
        W,
        Sc,
    > {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A, c: &C, a_idx: usize, b_idx: usize| {
            filter.test(s, a, c, a_idx, b_idx)
        };

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
