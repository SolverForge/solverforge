use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::{AndBiFilter, BiFilter, FnBiFilter, TriAsBiFilter, TrueFilter};
use super::super::flattened_bi_stream::FlattenedBiConstraintStream;
use super::super::projected_stream::{JoinedSource, Stream};
use super::grouped::Grouped;

/* Zero-erasure constraint stream over cross-entity pairs.

`Bi` joins entities from collection A with collection B,
accumulates filters on joined pairs, and finalizes into an
`Bi` via `penalize()` or `reward()`.
*/
pub struct Bi<S, A, B, K, EA, EB, KA, KB, F, Sc>
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

impl<S, A, B, K, EA, EB, KA, KB, Sc> Bi<S, A, B, K, EA, EB, KA, KB, TrueFilter, Sc>
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

impl<S, A, B, K, EA, EB, KA, KB, F, Sc> Bi<S, A, B, K, EA, EB, KA, KB, F, Sc>
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
    ) -> Bi<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        AndBiFilter<F, FnBiFilter<impl Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync>>,
        Sc,
    >
    where
        P: Fn(&A, &B) -> bool + Send + Sync,
    {
        Bi {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: AndBiFilter::new(
                self.filter,
                FnBiFilter::new(move |_s: &S, a: &A, b: &B, _a_idx: usize, _b_idx: usize| {
                    predicate(a, b)
                }),
            ),
            _phantom: PhantomData,
        }
    }

    /* Extends the joined (A, B) pairs with a third source C.

    The retained tri rows satisfy key_a(a) == key_b(b) == key_c(c); the
    bi stream's own keys stay authoritative for A and B, and `key_c`
    positions the new source in the shared key domain.
    */
    pub fn join<C, EC, KC>(
        self,
        target: (EC, KC),
    ) -> super::super::cross_tri_stream::Tri<
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
        TriAsBiFilter<F, A, B>,
        Sc,
    >
    where
        C: Clone + Send + Sync + 'static,
        EC: CollectionExtract<S, Item = C>,
        KC: Fn(&C) -> K + Send + Sync,
    {
        super::super::cross_tri_stream::Tri::new_with_filter(
            self.extractor_a,
            self.extractor_b,
            target.0,
            self.key_a,
            self.key_b,
            target.1,
            TriAsBiFilter::new(self.filter),
        )
    }

    /* Extends the joined (A, B) pairs with a third source C on its own key type.

    Unlike [`Bi::join`], the second relationship owns an independent key
    domain `K2`: the left closure receives the whole left row
    (`&Concat<Leaf<A>, B>`) so it can inspect any earlier binding, and the
    right closure sees only the new C entity. Pass the pair as
    `(extractor_c, equal_on(left_row_key, right_key))`.
    */
    pub fn join_on<C, EC, K2, LK, KC>(
        self,
        target: (
            EC,
            super::super::joiner::EqualJoiner<LK, KC, K2, super::super::joiner::Directed>,
        ),
    ) -> super::chained::ChainedTri<S, A, B, C, K, K2, EA, EB, EC, KA, KB, LK, KC, F, TrueFilter, Sc>
    where
        C: Clone + Send + Sync + 'static,
        EC: CollectionExtract<S, Item = C>,
        K2: Eq + Hash + Clone + Send + Sync,
        LK: for<'r> Fn(
                &super::super::relational::Concat<super::super::relational::Leaf<'r, A>, B>,
            ) -> K2
            + Send
            + Sync,
        KC: Fn(&C) -> K2 + Send + Sync,
    {
        let (extractor_c, joiner) = target;
        let (left_key, right_key) = joiner.into_keys();
        super::chained::ChainedTri {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            extractor_c,
            key_a: self.key_a,
            key_b: self.key_b,
            left_key,
            right_key,
            filter_ab: self.filter,
            filter: TrueFilter,
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

    pub fn group_by<GK, GF, C, V, R, Acc>(
        self,
        group_key_fn: GF,
        collector: C,
    ) -> Grouped<S, A, B, K, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, Sc>
    where
        GK: Eq + Hash + Clone + Send + Sync + 'static,
        GF: Fn(&A, &B) -> GK + Send + Sync,
        C: for<'i> super::super::collector::Collector<
                (&'i A, &'i B),
                Value = V,
                Result = R,
                Accumulator = Acc,
            > + Send
            + Sync
            + 'static,
        V: Send + Sync + 'static,
        R: Send + Sync + 'static,
        Acc: super::super::collector::Accumulator<V, R> + Send + Sync + 'static,
    {
        Grouped {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            group_key_fn,
            collector,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, Sc> Bi<S, A, B, K, EA, EB, KA, KB, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    Sc: Score + 'static,
{
    pub fn project<Out, P>(
        self,
        project: P,
    ) -> Stream<S, Out, JoinedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out>, TrueFilter, Sc>
    where
        Out: Send + Sync + 'static,
        P: Fn(&A, &B) -> Out + Send + Sync + 'static,
    {
        Stream::<S, Out, JoinedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out>, TrueFilter, Sc>::new(
            JoinedSource::new(
                self.extractor_a,
                self.extractor_b,
                self.key_a,
                self.key_b,
                self.filter,
                project,
            ),
        )
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, Sc: Score> std::fmt::Debug
    for Bi<S, A, B, K, EA, EB, KA, KB, F, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bi").finish()
    }
}
