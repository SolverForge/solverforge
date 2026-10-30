mod scored;
pub(crate) use scored::QuadScored;

/* Canonical four-source fluent join: one operator tree, no per-arity engine.

A `Quad` owns `JoinNode<JoinNode<JoinNode<A,B>,C>, D>` over compiled condition
plans. Each successive relationship compiles fresh from its joiner, so every
left closure receives the whole borrowed row of what came before it and every
key domain stays independent. Terminal scoring is the generic
`OperatorTerminal`; explanations borrow the full row.
*/

use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::filter::{AndQuadFilter, FnQuadFilter, QuadAsPentaFilter, QuadFilter};
use super::joiner::plan::{CompileCondition, IndexedPlan};
use super::relational::operator::{CollectionNode, ExplainRow, JoinNode, Operator, Pair};
use super::relational::{HandleMap, Leaf, RowHandle};
use super::weighting_support::ConstraintWeight;
use crate::constraint::relational::OperatorTerminal;

/// The full quad tree: ((A ⋈ B) ⋈ C) ⋈ D, each step its own compiled plan.
pub(crate) type BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> = JoinNode<
    S,
    JoinNode<
        S,
        JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, <P1 as CompileCondition>::Plan>,
        CollectionNode<S, EC>,
        <P2 as CompileCondition>::Plan,
    >,
    CollectionNode<S, ED>,
    <P3 as CompileCondition>::Plan,
>;

/// The quad tree's borrowed row: `Pair<Pair<Pair<Leaf<A>, Leaf<B>>, Leaf<C>>, Leaf<D>>`.
pub(crate) type QuadRow<'a, S, EA, EB, EC, ED, P1, P2, P3> =
    <BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> as Operator<S>>::View<'a>;

/// Borrowed (A, B, C, D) entity quadruple off a quad row view.
pub(crate) trait QuadEntities<'a, A, B, C, D> {
    fn entities(&self) -> (&'a A, &'a B, &'a C, &'a D);
    fn indexes(&self) -> (usize, usize, usize, usize);
}

impl<'a, A, B, C, D> QuadEntities<'a, A, B, C, D>
    for Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>
{
    #[inline]
    fn entities(&self) -> (&'a A, &'a B, &'a C, &'a D) {
        (
            self.left.left.left.entity,
            self.left.left.right.entity,
            self.left.right.entity,
            self.right.entity,
        )
    }
    #[inline]
    fn indexes(&self) -> (usize, usize, usize, usize) {
        (
            self.left.left.left.index,
            self.left.left.right.index,
            self.left.right.index,
            self.right.index,
        )
    }
}

/// Four-collection join stream: `((A ⋈ B) ⋈ C) ⋈ D`, each relationship
/// compiled from its own joiner (`P1`, `P2`, `P3`), so each left key reads the
/// whole row joined so far.
pub struct Quad<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F, Sc>
where
    Sc: Score,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
{
    pub(crate) tree: BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>,
    pub(crate) filter: F,
    pub(crate) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> C,
        fn() -> D,
        fn() -> Sc,
    )>,
}

impl<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F, Sc>
    Quad<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    D: Clone + Send + Sync + 'static,
    EA: 'static,
    EB: 'static,
    EC: 'static,
    ED: 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    P3: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>: Operator<S>,
    F: QuadFilter<S, A, B, C, D>,
    Sc: Score + 'static,
{
    /// Adds a filter predicate to the stream.
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> Quad<
        S,
        A,
        B,
        C,
        D,
        EA,
        EB,
        EC,
        ED,
        P1,
        P2,
        P3,
        AndQuadFilter<
            F,
            FnQuadFilter<
                impl Fn(&S, &A, &B, &C, &D, usize, usize, usize, usize) -> bool + Send + Sync,
            >,
        >,
        Sc,
    >
    where
        P: Fn(&A, &B, &C, &D) -> bool + Send + Sync,
    {
        Quad {
            tree: self.tree,
            filter: AndQuadFilter::new(
                self.filter,
                FnQuadFilter::new(
                    move |_s: &S,
                          a: &A,
                          b: &B,
                          c: &C,
                          d: &D,
                          _ai: usize,
                          _bi: usize,
                          _ci: usize,
                          _di: usize| { predicate(a, b, c, d) },
                ),
            ),
            _phantom: PhantomData,
        }
    }

    /// Extends this quadruple with a fifth source E.
    ///
    /// Later joins of a chain take VIEW keys, not entity keys: the left key
    /// closure receives the whole borrowed (A, B, C, D) row, the right key
    /// receives `&Leaf<'_, E>` and reads fields via `.entity`. `equal_bi` with
    /// entity closures does NOT typecheck here (E0631) — use `equal_on`.
    #[allow(clippy::type_complexity)]
    pub fn join<E, EE, P4>(
        self,
        target: (EE, P4),
    ) -> crate::stream::cross_penta_stream::Penta<
        S,
        A,
        B,
        C,
        D,
        E,
        EA,
        EB,
        EC,
        ED,
        EE,
        P1,
        P2,
        P3,
        P4,
        QuadAsPentaFilter<F, A, B, C, D>,
        Sc,
    >
    where
        E: Clone + Send + Sync + 'static,
        EE: super::collection_extract::CollectionExtract<S, Item = E> + 'static,
        P4: CompileCondition + 'static,
        P4::Plan: IndexedPlan,
        for<'a> P4::Plan: super::joiner::plan::ExecutablePlan<
            Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
            Leaf<'a, E>,
        >,
        for<'a> BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>: Operator<
            S,
            View<'a> = Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
        >,
        for<'a> JoinNode<
            S,
            BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>,
            CollectionNode<S, EE>,
            <P4 as CompileCondition>::Plan,
        >: Operator<S>,
    {
        let (extractor_e, condition) = target;
        let tree = JoinNode::new(self.tree, CollectionNode::new(extractor_e, 4), condition);
        crate::stream::cross_penta_stream::Penta {
            tree,
            filter: QuadAsPentaFilter::new(self.filter),
            _phantom: PhantomData,
        }
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> QuadBuilder<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F, W, Sc>
    where
        W: Fn(&A, &B, &C, &D) -> Sc + Send + Sync,
    {
        QuadBuilder {
            tree: self.tree,
            filter: self.filter,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each match by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched entities returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn penalize<W>(
        self,
        weight: W,
    ) -> QuadBuilder<
        S,
        A,
        B,
        C,
        D,
        EA,
        EB,
        EC,
        ED,
        P1,
        P2,
        P3,
        F,
        impl Fn(&A, &B, &C, &D) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w B, &'w C, &'w D), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |a: &A, b: &B, c: &C, d: &D| weight.score((a, b, c, d)),
            is_hard,
        )
    }

    /// Rewards each match by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched entities returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn reward<W>(
        self,
        weight: W,
    ) -> QuadBuilder<
        S,
        A,
        B,
        C,
        D,
        EA,
        EB,
        EC,
        ED,
        P1,
        P2,
        P3,
        F,
        impl Fn(&A, &B, &C, &D) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w B, &'w C, &'w D), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |a: &A, b: &B, c: &C, d: &D| weight.score((a, b, c, d)),
            is_hard,
        )
    }
}

/// Zero-erasure builder for finalizing a canonical quad constraint.
pub struct QuadBuilder<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F, W, Sc>
where
    Sc: Score,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
{
    tree: BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>,
    filter: F,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> C,
        fn() -> D,
        fn() -> Sc,
    )>,
}

impl<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F, W, Sc>
    QuadBuilder<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + std::fmt::Debug + Send + Sync + 'static,
    B: Clone + std::fmt::Debug + Send + Sync + 'static,
    C: Clone + std::fmt::Debug + Send + Sync + 'static,
    D: Clone + std::fmt::Debug + Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    for<'a> BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>: Operator<
        S,
        View<'a> = Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
    >,
    for<'a> QuadRow<'a, S, EA, EB, EC, ED, P1, P2, P3>: ExplainRow,
    F: QuadFilter<S, A, B, C, D> + 'static,
    W: Fn(&A, &B, &C, &D) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Names the constraint and builds it.
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<
        S,
        QuadScored<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F>,
        impl Fn(&S, &QuadRow<'_, S, EA, EB, EC, ED, P1, P2, P3>) -> Sc + Send + Sync,
        Sc,
    > {
        let weight = self.weight;
        let is_hard = self.is_hard;
        let constraint_ref = ConstraintRef::new("", name);
        let scored = QuadScored {
            inner: self.tree,
            filter: self.filter,
            accepted: HandleMap::new(),
            child_removed: Vec::new(),
            child_inserted: Vec::new(),
            marker: PhantomData,
        };
        let weight_fn = move |_: &S, row: &QuadRow<'_, S, EA, EB, EC, ED, P1, P2, P3>| {
            let (a, b, c, d) = QuadEntities::entities(row);
            weight(a, b, c, d)
        };
        OperatorTerminal::new(constraint_ref, self.impact_type, scored, weight_fn, is_hard)
    }
}
