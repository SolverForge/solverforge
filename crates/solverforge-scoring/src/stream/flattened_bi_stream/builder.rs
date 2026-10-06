use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::super::collection_extract::CollectionExtract;
use super::super::filter::BiFilter;
use super::super::relational::operator::FlattenView;
use super::super::relational::operator::{CollectionNode, FlattenNode, Pair};
use super::super::relational::view_plan::{EntityKey, FlattenedPairKey, PairKey, ViewEqualPlan};
use super::super::relational::Leaf;
use super::scored::FlattenedBiScored;
use crate::constraint::relational::OperatorTerminal;

/* Finalizes a flattened bi-constraint onto the shared operator tree: a
`JoinNode` between the A collection and a `FlattenNode` over the B collection,
joined on `(join key, lookup key) == (owner key, child key)`. Scoring is the
generic `OperatorTerminal`. */
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
    A: Clone + Send + Sync + std::fmt::Debug + 'static,
    B: Clone + Send + Sync + 'static,
    C: 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    CK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync + 'static,
    EB: CollectionExtract<S, Item = B> + Send + Sync + 'static,
    KA: Fn(&A) -> K + Send + Sync + 'static,
    KB: Fn(&B) -> K + Send + Sync + 'static,
    Flatten: for<'a> Fn(&'a B) -> &'a [C] + Send + Sync + 'static,
    CKeyFn: Fn(&C) -> CK + Send + Sync + 'static,
    ALookup: Fn(&A) -> CK + Send + Sync + 'static,
    F: BiFilter<S, A, C> + 'static,
    W: Fn(&A, &C) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<
        S,
        FlattenedBiScored<
            S,
            A,
            B,
            C,
            super::super::relational::operator::JoinNode<
                S,
                CollectionNode<S, EA>,
                FlattenNode<
                    CollectionNode<S, EB>,
                    super::super::relational::operator::ParentFlatten<Flatten>,
                >,
                ViewEqualPlan<
                    (K, CK),
                    PairKey<EntityKey<KA>, EntityKey<ALookup>>,
                    FlattenedPairKey<KB, CKeyFn>,
                >,
            >,
            F,
        >,
        impl for<'a> Fn(&S, &Pair<Leaf<'a, A>, FlattenView<'a, Leaf<'a, B>, C>>) -> Sc + Send + Sync,
        Sc,
    > {
        let plan = ViewEqualPlan::new(
            PairKey::new(EntityKey::new(self.key_a), EntityKey::new(self.a_lookup_fn)),
            FlattenedPairKey::new(self.key_b, self.c_key_fn),
        );
        let left = CollectionNode::new(self.extractor_a, 1);
        let right = FlattenNode::new(
            CollectionNode::new(self.extractor_b, 0),
            super::super::relational::operator::ParentFlatten::new(self.flatten),
        );
        let tree = super::super::relational::operator::JoinNode::new(left, right, plan);
        let scored = FlattenedBiScored::new(tree, self.filter);
        let weight = self.weight;
        let weight_fn = move |_: &S, row: &Pair<Leaf<'_, A>, FlattenView<'_, Leaf<'_, B>, C>>| {
            let (a, c) = super::scored::FlattenedBiEntities::entities(row);
            weight(a, c)
        };
        OperatorTerminal::new(
            ConstraintRef::new("", name),
            self.impact_type,
            scored,
            weight_fn,
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
