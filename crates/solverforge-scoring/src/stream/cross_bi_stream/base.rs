use std::marker::PhantomData;

use solverforge_core::score::Score;

/* The first-join plan as compiled from a `.join((target, condition))`.

The stream owns the compiled condition, not a raw key pair: an equality
target yields `BiUnaryPlan` (typed unary keys lifted onto borrowed leaf
views), a predicate target yields `FilteringJoiner` (explicit
opposite-input scan). No constant-key Cartesian shortcut exists.
*/

use super::super::collection_extract::CollectionExtract;
use super::super::filter::{AndBiFilter, BiFilter, FnBiFilter, TriAsBiFilter, TrueFilter};
use super::super::flattened_bi_stream::FlattenedBiConstraintStream;
use super::super::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use super::super::projected_stream::{JoinedSource, Stream};
use super::grouped::Grouped;
use super::scored::BiUnaryPlan;

/* Zero-erasure constraint stream over cross-entity pairs.

`Bi` joins entities from collection A with collection B, accumulates
filters on joined pairs, and finalizes into an operator terminal via
`penalize()` or `reward()`. `P` is the compiled first-relationship plan:
`BiUnaryPlan` for a keyed join, `FilteringJoiner` for a predicate join.
The finalize, join, and filter paths are generic over `P`; grouping,
projection, and flattening are specialized to `BiUnaryPlan`, where a
unary key pair exists to rederive.
*/
pub struct Bi<S, A, B, P, EA, EB, F, Sc>
where
    Sc: Score,
{
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) plan: P,
    pub(super) filter: F,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> Sc)>,
}

impl<S, A, B, K, KA, KB, EA, EB, Sc> Bi<S, A, B, BiUnaryPlan<K, KA, KB>, EA, EB, TrueFilter, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Sc: Score + 'static,
{
    pub fn new(extractor_a: EA, extractor_b: EB, key_a: KA, key_b: KB) -> Self {
        Bi {
            extractor_a,
            extractor_b,
            plan: BiUnaryPlan::new(key_a, key_b),
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, K, KA, KB, EA, EB, F, Sc> Bi<S, A, B, BiUnaryPlan<K, KA, KB>, EA, EB, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    Sc: Score + 'static,
{
    /* Builds a keyed cross-bi stream with an initial membership filter. */
    pub fn new_with_filter(
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
    ) -> Self {
        Bi {
            extractor_a,
            extractor_b,
            plan: BiUnaryPlan::new(key_a, key_b),
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, P, EA, EB, F, Sc> Bi<S, A, B, P, EA, EB, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    F: BiFilter<S, A, B>,
    Sc: Score + 'static,
{
    /* Builds a stream from a compiled first-relationship plan and filter.

    Used by the join-target dispatch: an equality target supplies a
    `BiUnaryPlan`, a predicate target a `FilteringJoiner`. Neither path
    fabricates a constant equality key.
    */
    pub(crate) fn from_condition(extractor_a: EA, extractor_b: EB, plan: P, filter: F) -> Self {
        Bi {
            extractor_a,
            extractor_b,
            plan,
            filter,
            _phantom: PhantomData,
        }
    }

    /* Adds a filter predicate to the stream. */
    pub fn filter<Q>(
        self,
        predicate: Q,
    ) -> Bi<
        S,
        A,
        B,
        P,
        EA,
        EB,
        AndBiFilter<F, FnBiFilter<impl Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync>>,
        Sc,
    >
    where
        Q: Fn(&A, &B) -> bool + Send + Sync,
    {
        Bi {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            plan: self.plan,
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

    One uniform `.join()`: the target tuple's condition type chooses
    execution through `CompileCondition`. A row-aware joiner (`equal_on`)
    relates the whole borrowed (A, B) row to C on its own key domain; a
    unary joiner (`equal_bi`) is adapted onto the row so the same
    independent-domain tree executes it. Shared-key retention is not
    involved at any depth.
    */
    pub fn join<C, EC, P2>(
        self,
        target: (EC, P2),
    ) -> super::super::cross_tri_stream::Tri<
        S,
        A,
        B,
        C,
        EA,
        EB,
        EC,
        P,
        P2,
        TriAsBiFilter<F, A, B>,
        Sc,
    >
    where
        C: Clone + Send + Sync + 'static,
        EC: CollectionExtract<S, Item = C> + 'static,
        P: CompileCondition + 'static,
        for<'a> P::Plan: ExecutablePlan<
            super::super::relational::Leaf<'a, A>,
            super::super::relational::Leaf<'a, B>,
        >,
        P::Plan: IndexedPlan<Indexes: Send + Sync>,
        P2: CompileCondition + 'static,
        P2::Plan: IndexedPlan,
        EA: 'static,
        EB: 'static,
        for<'a> P2::Plan: ExecutablePlan<
            super::super::relational::operator::Pair<
                super::super::relational::Leaf<'a, A>,
                super::super::relational::Leaf<'a, B>,
            >,
            super::super::relational::Leaf<'a, C>,
        >,
    {
        let (extractor_c, condition) = target;
        super::super::cross_tri_stream::assemble_tri(
            self.extractor_a,
            self.extractor_b,
            extractor_c,
            self.plan,
            condition,
            TriAsBiFilter::new(self.filter),
        )
    }
}

impl<S, A, B, K, KA, KB, EA, EB, F, Sc> Bi<S, A, B, BiUnaryPlan<K, KA, KB>, EA, EB, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    Sc: Score + 'static,
{
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
        TrueFilter,
        Sc,
    >
    where
        C: Clone + Send + Sync + 'static,
        CK: Eq + std::hash::Hash + Clone + Send + Sync,
        Flatten: Fn(&B) -> &[C] + Send + Sync,
        CKeyFn: Fn(&C) -> CK + Send + Sync,
        ALookup: Fn(&A) -> CK + Send + Sync,
    {
        let (key_a, key_b) = self.plan.into_keys();
        FlattenedBiConstraintStream::new(
            self.extractor_a,
            self.extractor_b,
            key_a,
            key_b,
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
        GK: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
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
        let (key_a, key_b) = self.plan.into_keys();
        Grouped {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a,
            key_b,
            filter: self.filter,
            group_key_fn,
            collector,
            _phantom: PhantomData,
        }
    }

    pub fn project<Out, Proj>(
        self,
        project: Proj,
    ) -> Stream<S, Out, JoinedSource<S, A, B, K, EA, EB, KA, KB, F, Proj, Out>, TrueFilter, Sc>
    where
        Out: Send + Sync + 'static,
        Proj: Fn(&A, &B) -> Out + Send + Sync + 'static,
    {
        let (key_a, key_b) = self.plan.into_keys();
        Stream::<S, Out, JoinedSource<S, A, B, K, EA, EB, KA, KB, F, Proj, Out>, TrueFilter, Sc>::new(
            JoinedSource::new(
                self.extractor_a,
                self.extractor_b,
                key_a,
                key_b,
                self.filter,
                project,
            ),
        )
    }
}

impl<S, A, B, P, EA, EB, F, Sc: Score> std::fmt::Debug for Bi<S, A, B, P, EA, EB, F, Sc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bi").finish()
    }
}
