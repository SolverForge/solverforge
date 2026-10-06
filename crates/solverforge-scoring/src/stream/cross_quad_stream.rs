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
use super::relational::operator::{
    CollectionNode, ExplainRow, JoinNode, Operator, Pair, RowChanges,
};
use super::relational::{HandleMap, Leaf, RowHandle};
use super::weighting_support::ConstraintWeight;
use crate::constraint::relational::OperatorTerminal;

/* The full quad tree: ((A ⋈ B) ⋈ C) ⋈ D, each step its own compiled plan. */
pub type BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> = JoinNode<
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

/* The quad tree's borrowed row: `Pair<Pair<Pair<Leaf<A>, Leaf<B>>, Leaf<C>>, Leaf<D>>`. */
pub type QuadRow<'a, S, EA, EB, EC, ED, P1, P2, P3> =
    <BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> as Operator<S>>::View<'a>;

/* Borrowed (A, B, C, D) entity quadruple off a quad row view. */
pub trait QuadEntities<'a, A, B, C, D> {
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
    /* Adds a filter predicate to the stream. */
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

    /* Extends this quadruple with a fifth source E. */
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

/* Zero-erasure builder for finalizing a canonical quad constraint. */
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
            marker: PhantomData,
        };
        let weight_fn = move |_: &S, row: &QuadRow<'_, S, EA, EB, EC, ED, P1, P2, P3>| {
            let (a, b, c, d) = QuadEntities::entities(row);
            weight(a, b, c, d)
        };
        OperatorTerminal::new(constraint_ref, self.impact_type, scored, weight_fn, is_hard)
    }
}

/* Scored quad operator: the tree plus the authored filter as one row producer. */
pub struct QuadScored<
    S,
    A,
    B,
    C,
    D,
    EA,
    EB,
    EC,
    ED,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    F,
> where
    BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>: Operator<S>,
    F: QuadFilter<S, A, B, C, D>,
{
    inner: BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>,
    filter: F,
    accepted: HandleMap<()>,
    marker: PhantomData<fn() -> (S, A, B, C, D)>,
}

impl<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F> Operator<S>
    for QuadScored<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    D: Clone + Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    P3: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>: Operator<
        S,
        View<'a> = Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
    >,
    F: QuadFilter<S, A, B, C, D> + 'static,
{
    type View<'a> = QuadRow<'a, S, EA, EB, EC, ED, P1, P2, P3>;
    type Evaluation = <BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> as Operator<S>>::Evaluation;

    #[inline]
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        self.inner.prepare_evaluation(solution)
    }

    #[inline]
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        self.inner
            .visit_evaluation(solution, evaluation, &mut |row| {
                let (a, b, c, d) = row.entities();
                let (ai, bi, ci, di) = row.indexes();
                if self.filter.test(solution, a, b, c, d, ai, bi, ci, di) {
                    visitor(row);
                }
            });
    }

    fn clear(&mut self) {
        self.inner.clear();
        self.accepted.clear();
    }

    fn initialize(&mut self, solution: &S) {
        self.inner.initialize(solution);
        self.accepted.clear();
        for handle in self.inner.handles() {
            let row = self.inner.resolve(solution, handle).expect("live quad row");
            if self.test(solution, row) {
                self.accepted.insert(handle, ());
            }
        }
    }

    fn handles(&self) -> Vec<RowHandle> {
        self.inner
            .handles()
            .into_iter()
            .filter(|h| self.accepted.get(*h).is_some())
            .collect()
    }

    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        self.accepted.get(handle)?;
        self.inner.resolve(solution, handle)
    }

    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if self.accepted.get(handle).is_some() {
            self.inner.visit_provenance(handle, visitor);
        }
    }

    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        let mut changes = self.inner.retract(solution, descriptor, index);
        changes
            .removed
            .retain(|h| self.accepted.remove(*h).is_some());
        changes
    }

    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        let mut changes = self.inner.insert(solution, descriptor, index);
        let mut accepted = Vec::with_capacity(changes.inserted.len());
        for handle in changes.inserted.drain(..) {
            let row = self.inner.resolve(solution, handle).expect("live quad row");
            if self.test(solution, row) {
                self.accepted.insert(handle, ());
                accepted.push(handle);
            }
        }
        changes.inserted = accepted;
        changes
    }
}

impl<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F>
    QuadScored<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F>
where
    S: Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>: Operator<S>,
    F: QuadFilter<S, A, B, C, D>,
{
    #[inline]
    fn test<'a>(
        &self,
        solution: &S,
        row: <BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> as Operator<S>>::View<'a>,
    ) -> bool
    where
        A: 'a,
        B: 'a,
        C: 'a,
        D: 'a,
        <BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> as Operator<S>>::View<'a>:
            QuadEntities<'a, A, B, C, D>,
    {
        let (a, b, c, d) = row.entities();
        let (ai, bi, ci, di) = row.indexes();
        self.filter.test(solution, a, b, c, d, ai, bi, ci, di)
    }
}
