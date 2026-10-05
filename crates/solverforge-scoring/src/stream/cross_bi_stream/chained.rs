/* Chained fluent stream: two independent equi-joins with their own key types.

`ChainedTri` is the fluent counterpart of `stream::relational::ChainedJoin`.
The first join relates A to B on `K1`; the second relates the whole left
row to C on `K2` through a row-aware left closure. Heterogeneous successive
keys stay in separate domains because each relationship owns its type —
never unified into one `K` the way the legacy shared-key `Tri` requires.

Finalizes into `constraint::relational::TripleTerminal` via
`penalize()`/`reward()` + `named()`. All function types stay concrete
generics: no trait objects, no Arc, fully monomorphized.
*/

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::super::collection_extract::{ChangeSource, CollectionExtract};
use super::super::filter::{AndTriFilter, BiFilter, FnTriFilter, TriFilter};
use super::super::relational::{Concat, Leaf};
use super::super::weighting_support::ConstraintWeight;
use crate::constraint::relational::TripleTerminal;

/* Fluent stream over a chained (A, B) + row-to-C join with independent keys. */
pub struct ChainedTri<S, A, B, C, K1, K2, EA, EB, EC, KA, KB, LK, KC, F1, F2, Sc>
where
    Sc: Score,
{
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) extractor_c: EC,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) left_key: LK,
    pub(super) right_key: KC,
    pub(super) filter_ab: F1,
    pub(super) filter: F2,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> C,
        fn() -> K1,
        fn() -> K2,
        fn() -> Sc,
    )>,
}

impl<S, A, B, C, K1, K2, EA, EB, EC, KA, KB, LK, KC, F1, F2, Sc>
    ChainedTri<S, A, B, C, K1, K2, EA, EB, EC, KA, KB, LK, KC, F1, F2, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K1: Eq + Hash + Clone + Send + Sync,
    K2: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    KA: Fn(&A) -> K1 + Send + Sync,
    KB: Fn(&B) -> K1 + Send + Sync,
    LK: for<'r> Fn(&Concat<Leaf<'r, A>, B>) -> K2 + Send + Sync,
    KC: Fn(&C) -> K2 + Send + Sync,
    F1: BiFilter<S, A, B>,
    F2: TriFilter<S, A, B, C>,
    Sc: Score + 'static,
{
    /* Adds a filter predicate over the triple. */
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> ChainedTri<
        S,
        A,
        B,
        C,
        K1,
        K2,
        EA,
        EB,
        EC,
        KA,
        KB,
        LK,
        KC,
        F1,
        AndTriFilter<
            F2,
            FnTriFilter<impl Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync>,
        >,
        Sc,
    >
    where
        P: Fn(&A, &B, &C) -> bool + Send + Sync,
    {
        ChainedTri {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            extractor_c: self.extractor_c,
            key_a: self.key_a,
            key_b: self.key_b,
            left_key: self.left_key,
            right_key: self.right_key,
            filter_ab: self.filter_ab,
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
    ) -> ChainedBuilder<S, A, B, C, K1, K2, EA, EB, EC, KA, KB, LK, KC, F1, F2, W, Sc>
    where
        W: Fn(&A, &B, &C) -> Sc + Send + Sync,
    {
        ChainedBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            extractor_c: self.extractor_c,
            key_a: self.key_a,
            key_b: self.key_b,
            left_key: self.left_key,
            right_key: self.right_key,
            filter_ab: self.filter_ab,
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
    ) -> ChainedBuilder<
        S,
        A,
        B,
        C,
        K1,
        K2,
        EA,
        EB,
        EC,
        KA,
        KB,
        LK,
        KC,
        F1,
        F2,
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
    ) -> ChainedBuilder<
        S,
        A,
        B,
        C,
        K1,
        K2,
        EA,
        EB,
        EC,
        KA,
        KB,
        LK,
        KC,
        F1,
        F2,
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

/* Zero-erasure builder finalizing a chained triple constraint. */
pub struct ChainedBuilder<S, A, B, C, K1, K2, EA, EB, EC, KA, KB, LK, KC, F1, F2, W, Sc>
where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    extractor_c: EC,
    key_a: KA,
    key_b: KB,
    left_key: LK,
    right_key: KC,
    filter_ab: F1,
    filter: F2,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> C,
        fn() -> K1,
        fn() -> K2,
        fn() -> Sc,
    )>,
}

impl<S, A, B, C, K1, K2, EA, EB, EC, KA, KB, LK, KC, F1, F2, W, Sc>
    ChainedBuilder<S, A, B, C, K1, K2, EA, EB, EC, KA, KB, LK, KC, F1, F2, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K1: Eq + Hash + Clone + Send + Sync,
    K2: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    KA: Fn(&A) -> K1 + Send + Sync,
    KB: Fn(&B) -> K1 + Send + Sync,
    LK: for<'r> Fn(&Concat<Leaf<'r, A>, B>) -> K2 + Send + Sync,
    KC: Fn(&C) -> K2 + Send + Sync,
    F1: BiFilter<S, A, B>,
    F2: TriFilter<S, A, B, C>,
    W: Fn(&A, &B, &C) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /* Reads the descriptor index owned by one extractor.

    Macro-generated solution sources carry `Descriptor(i)`; raw fn
    extractors are `Unknown` and static sources never change. Unknown and
    static inputs keep the checked-in contract: they react to every
    descriptor through `assert_localizes`, so the operator maps them to a
    sentinel no real descriptor owns.
    */
    fn descriptor_of<E>(extractor: &E) -> usize
    where
        E: CollectionExtract<S>,
    {
        match extractor.change_source() {
            ChangeSource::Descriptor(i) => i,
            ChangeSource::Unknown | ChangeSource::Static => usize::MAX,
        }
    }

    pub fn named(
        self,
        name: &str,
    ) -> TripleTerminal<
        S,
        A,
        B,
        C,
        EA,
        EB,
        EC,
        K1,
        KA,
        KB,
        K2,
        LK,
        KC,
        impl Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
        impl Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync,
        W,
        Sc,
    > {
        // The AB filter travels with the first join: ChainedJoin and every
        // TripleTerminal stateless path apply `first.filter()` themselves,
        // so the chained filter keeps only the post-join predicates here.
        let filter = self.filter;
        let combined_ab = move |s: &S, a: &A, b: &B, a_idx: usize, b_idx: usize| {
            self.filter_ab.test(s, a, b, a_idx, b_idx)
        };
        let combined =
            move |s: &S, a: &A, b: &B, c: &C, a_idx: usize, b_idx: usize, c_idx: usize| {
                filter.test(s, a, b, c, a_idx, b_idx, c_idx)
            };
        let left_a_descriptor = Self::descriptor_of(&self.extractor_a);
        let left_b_descriptor = Self::descriptor_of(&self.extractor_b);
        let right_descriptor = Self::descriptor_of(&self.extractor_c);

        TripleTerminal::new(
            ConstraintRef::new("", name),
            self.impact_type,
            crate::stream::relational::EquiJoin::new(
                self.extractor_a,
                self.extractor_b,
                self.key_a,
                self.key_b,
                combined_ab,
                left_a_descriptor,
                left_b_descriptor,
                name,
            ),
            self.extractor_c,
            self.left_key,
            self.right_key,
            combined,
            self.weight,
            self.is_hard,
            left_a_descriptor,
            left_b_descriptor,
            right_descriptor,
            name,
        )
    }
}

impl<S, A, B, C, K1, K2, EA, EB, EC, KA, KB, LK, KC, F1, F2, W, Sc: Score> std::fmt::Debug
    for ChainedBuilder<S, A, B, C, K1, K2, EA, EB, EC, KA, KB, LK, KC, F1, F2, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainedBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
