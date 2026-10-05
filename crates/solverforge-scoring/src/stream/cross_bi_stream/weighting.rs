use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::super::collection_extract::CollectionExtract;
use super::super::filter::BiFilter;
use super::super::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use super::super::relational::operator::{CollectionNode, JoinNode, Operator, Pair};
use super::super::relational::Leaf;
use super::super::weighting_support::ConstraintWeight;
use super::base::Bi;
use super::scored::{BiEntities, BiScored};
use crate::constraint::relational::OperatorTerminal;

impl<S, A, B, P, EA, EB, F, Sc> Bi<S, A, B, P, EA, EB, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    P: CompileCondition + 'static,
    for<'a> P::Plan: ExecutablePlan<Leaf<'a, A>, Leaf<'a, B>>,
    P::Plan: IndexedPlan<Indexes: Send + Sync>,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    F: BiFilter<S, A, B>,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> Builder<S, A, B, P, EA, EB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        Builder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            plan: self.plan,
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
    ) -> Builder<S, A, B, P, EA, EB, F, impl Fn(&A, &B) -> Sc + Send + Sync, Sc>
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w B), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |a: &A, b: &B| weight.score((a, b)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> Builder<S, A, B, P, EA, EB, F, impl Fn(&A, &B) -> Sc + Send + Sync, Sc>
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w B), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |a: &A, b: &B| weight.score((a, b)),
            is_hard,
        )
    }
}

// Zero-erasure builder for finalizing a cross-bi constraint.
pub struct Builder<S, A, B, P, EA, EB, F, W, Sc>
where
    Sc: Score,
    P: CompileCondition,
{
    extractor_a: EA,
    extractor_b: EB,
    plan: P,
    filter: F,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> Sc)>,
}

impl<S, A, B, P, EA, EB, F, W, Sc> Builder<S, A, B, P, EA, EB, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    P: CompileCondition + 'static,
    for<'a> P::Plan: ExecutablePlan<Leaf<'a, A>, Leaf<'a, B>>,
    P::Plan: IndexedPlan<Indexes: Send + Sync>,
    EA: CollectionExtract<S, Item = A> + 'static,
    EB: CollectionExtract<S, Item = B> + 'static,
    F: BiFilter<S, A, B> + 'static,
    W: Fn(&A, &B) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /* Finalizes into the generic operator terminal.

    The compiled first-relationship plan becomes the `JoinNode`'s condition;
    the authored bi filter runs inside the scored operator so evaluate,
    match_count, initialize, and mutations honor it exactly once.
    */
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<
        S,
        BiScored<S, A, B, EA, EB, P::Plan, F>,
        impl Fn(&S, &Pair<Leaf<'_, A>, Leaf<'_, B>>) -> Sc + Send + Sync,
        Sc,
    >
    where
        for<'a> JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, P::Plan>:
            Operator<S, View<'a> = Pair<Leaf<'a, A>, Leaf<'a, B>>>,
    {
        let tree = JoinNode::new(
            CollectionNode::new(self.extractor_a, 1),
            CollectionNode::new(self.extractor_b, 0),
            self.plan,
        );
        let scored = BiScored::new(tree, self.filter);
        let weight = self.weight;
        let weight_fn = move |_: &S, row: &Pair<Leaf<'_, A>, Leaf<'_, B>>| {
            let (a, b) = row.entities();
            weight(a, b)
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

impl<S, A, B, P, EA, EB, F, W, Sc: Score> std::fmt::Debug for Builder<S, A, B, P, EA, EB, F, W, Sc>
where
    P: CompileCondition,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
