use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::{AndBiFilter, BiFilter, FnBiFilter, TrueFilter};
use super::super::flattened_bi_stream::FlattenedBiConstraintStream;

/* Zero-erasure constraint stream over cross-entity pairs.

`CrossBiConstraintStream` joins entities from collection A with collection B,
accumulates filters on joined pairs, and finalizes into an
`IncrementalCrossBiConstraint` via `penalize()` or `reward()`.
*/
pub struct CrossBiConstraintStream<S, A, B, K, EA, EB, KA, KB, F, Sc>
where
    Sc: Score,
{
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) filter: F,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> K, fn() -> Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, Sc>
    CrossBiConstraintStream<S, A, B, K, EA, EB, KA, KB, TrueFilter, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Sc: Score + 'static,
{
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
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    Sc: Score + 'static,
{
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

    /* Adds a filter predicate to the stream. */
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

    /* Expands items from entity B into separate (A, C) pairs with O(1) lookup. */
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
        super::super::filter::TrueFilter,
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
