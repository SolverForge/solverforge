use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::{AndBiFilter, BiFilter, FnBiFilter, TrueFilter};

/* O(1) flattened bi-constraint stream.

Pre-indexes C items by key for O(1) lookup.
*/
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
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) flatten: Flatten,
    pub(super) c_key_fn: CKeyFn,
    pub(super) a_lookup_fn: ALookup,
    pub(super) filter: F,
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
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Flatten: Fn(&B) -> &[C] + Send + Sync,
    CKeyFn: Fn(&C) -> CK + Send + Sync,
    ALookup: Fn(&A) -> CK + Send + Sync,
    Sc: Score + 'static,
{
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
    FlattenedBiConstraintStream<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, Sc>
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
    Sc: Score + 'static,
{
    // Adds a filter predicate to the stream.
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
        AndBiFilter<F, FnBiFilter<impl Fn(&S, &A, &C, usize, usize) -> bool + Send + Sync>>,
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
            filter: AndBiFilter::new(
                self.filter,
                FnBiFilter::new(move |_s: &S, a: &A, c: &C, _a_idx: usize, _b_idx: usize| {
                    predicate(a, c)
                }),
            ),
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
        f.debug_struct("FlattenedBiConstraintStream").finish()
    }
}
