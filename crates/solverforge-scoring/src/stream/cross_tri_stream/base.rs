/* Canonical three-source fluent join: one operator tree, no per-arity engine.

A `Tri` owns `JoinNode<JoinNode<A,B>, C>` over compiled condition plans.
The first relationship keeps the Bi stream's typed keys; the second
relationship is compiled fresh from its joiner, so the left closure
receives the whole borrowed (A, B) row (`Concat<Leaf<A>, B>`) and the
key domain is independent of the first. Terminal scoring is the generic
`OperatorTerminal`; explanations borrow the full row.
*/

use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::super::collection_extract::CollectionExtract;
use super::super::filter::{AndTriFilter, FnTriFilter, TriAsQuadFilter, TriFilter};
use super::super::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use super::super::relational::operator::{
    CollectionNode, ExplainRow, JoinNode, Operator, Pair, RowChanges,
};
use super::super::relational::{HandleMap, Leaf, RowHandle};
use super::super::weighting_support::ConstraintWeight;
use crate::constraint::relational::OperatorTerminal;

/* The full tri tree: (A ⋈ B on the AB plan) ⋈ C on P2. Entity types ride
inside the extractors and the AB plan's key closures. */
pub type BuiltTri<S, EA, EB, EC, P1, P2> = JoinNode<
    S,
    JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, <P1 as CompileCondition>::Plan>,
    CollectionNode<S, EC>,
    <P2 as CompileCondition>::Plan,
>;

/* The tri tree's borrowed row: `Pair<Pair<Leaf<A>, Leaf<B>>, Leaf<C>>`. */
pub type TriRow<'a, S, EA, EB, EC, P1, P2> =
    <BuiltTri<S, EA, EB, EC, P1, P2> as Operator<S>>::View<'a>;

/* Assembles the tri tree from extractors, unary first keys, and a compiled
second-relationship plan. Shared by `Bi::join` dispatch. */
#[allow(clippy::too_many_arguments)]
pub(super) fn build_tree<S, A, B, C, EA, EB, EC, P1, P2>(
    extractor_a: EA,
    extractor_b: EB,
    extractor_c: EC,
    ab_plan: P1,
    second_plan: P2,
) -> BuiltTri<S, EA, EB, EC, P1, P2>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + 'static,
    EB: CollectionExtract<S, Item = B> + 'static,
    EC: CollectionExtract<S, Item = C> + 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> <P1 as CompileCondition>::Plan: ExecutablePlan<Leaf<'a, A>, Leaf<'a, B>>,
    for<'a> <P2 as CompileCondition>::Plan:
        ExecutablePlan<
            <JoinNode<
                S,
                CollectionNode<S, EA>,
                CollectionNode<S, EB>,
                <P1 as CompileCondition>::Plan,
            > as Operator<S>>::View<'a>,
            Leaf<'a, C>,
        >,
    JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, <P1 as CompileCondition>::Plan>:
        Operator<S>,
    BuiltTri<S, EA, EB, EC, P1, P2>: Operator<S>,
{
    let left = JoinNode::new(
        CollectionNode::new(extractor_a, 1),
        CollectionNode::new(extractor_b, 0),
        ab_plan,
    );
    JoinNode::new(left, CollectionNode::new(extractor_c, 2), second_plan)
}

/* Assembles a `Tri` from a Bi stream's parts plus the compiled second
condition. Single construction point for `Bi::join` dispatch. */
#[allow(clippy::too_many_arguments)]
pub fn assemble_tri<S, A, B, C, EA, EB, EC, P1, P2, F, Sc>(
    extractor_a: EA,
    extractor_b: EB,
    extractor_c: EC,
    ab_plan: P1,
    second: P2,
    filter: F,
) -> Tri<S, A, B, C, EA, EB, EC, P1, P2, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + 'static,
    EB: CollectionExtract<S, Item = B> + 'static,
    EC: CollectionExtract<S, Item = C> + 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> <P1 as CompileCondition>::Plan: ExecutablePlan<Leaf<'a, A>, Leaf<'a, B>>,
    for<'a> <P2 as CompileCondition>::Plan:
        ExecutablePlan<
            <JoinNode<
                S,
                CollectionNode<S, EA>,
                CollectionNode<S, EB>,
                <P1 as CompileCondition>::Plan,
            > as Operator<S>>::View<'a>,
            Leaf<'a, C>,
        >,
    JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, <P1 as CompileCondition>::Plan>:
        Operator<S>,
    BuiltTri<S, EA, EB, EC, P1, P2>: Operator<S>,
    F: TriFilter<S, A, B, C>,
    Sc: Score,
{
    let tree = build_tree(extractor_a, extractor_b, extractor_c, ab_plan, second);
    Tri {
        tree,
        filter,
        _phantom: PhantomData,
    }
}

pub struct Tri<S, A, B, C, EA, EB, EC, P1, P2, F, Sc>
where
    Sc: Score,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
{
    pub(super) tree: BuiltTri<S, EA, EB, EC, P1, P2>,
    pub(super) filter: F,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> C, fn() -> Sc)>,
}

impl<S, A, B, C, EA, EB, EC, P1, P2, F, Sc> Tri<S, A, B, C, EA, EB, EC, P1, P2, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EA: 'static,
    EB: 'static,
    EC: 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> <P1 as CompileCondition>::Plan: ExecutablePlan<Leaf<'a, A>, Leaf<'a, B>>,
    for<'a> <P2 as CompileCondition>::Plan:
        ExecutablePlan<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>,
    JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, <P1 as CompileCondition>::Plan>:
        Operator<S>,
    BuiltTri<S, EA, EB, EC, P1, P2>: Operator<S>,
    F: TriFilter<S, A, B, C>,
    Sc: Score + 'static,
{
    /* Adds a filter predicate to the stream. */
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> Tri<
        S,
        A,
        B,
        C,
        EA,
        EB,
        EC,
        P1,
        P2,
        AndTriFilter<
            F,
            FnTriFilter<impl Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync>,
        >,
        Sc,
    >
    where
        P: Fn(&A, &B, &C) -> bool + Send + Sync,
    {
        Tri {
            tree: self.tree,
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
    ) -> Builder<S, A, B, C, EA, EB, EC, P1, P2, F, W, Sc>
    where
        W: Fn(&A, &B, &C) -> Sc + Send + Sync,
    {
        Builder {
            tree: self.tree,
            filter: self.filter,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    /* Extends this triple with a fourth source D.

    The target tuple's condition compiles fresh, so its left closure receives
    the whole borrowed (A, B, C) row and the fourth key domain is independent
    of every earlier relationship.
    */
    pub fn join<D, ED, P3>(
        self,
        target: (ED, P3),
    ) -> crate::stream::cross_quad_stream::Quad<
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
        TriAsQuadFilter<F, A, B, C>,
        Sc,
    >
    where
        D: Clone + Send + Sync + 'static,
        ED: CollectionExtract<S, Item = D> + 'static,
        P3: CompileCondition + 'static,
        P3::Plan: IndexedPlan,
        for<'a> P3::Plan:
            ExecutablePlan<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
        for<'a> BuiltTri<S, EA, EB, EC, P1, P2>:
            Operator<S, View<'a> = Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>>,
        for<'a> JoinNode<
            S,
            BuiltTri<S, EA, EB, EC, P1, P2>,
            CollectionNode<S, ED>,
            <P3 as CompileCondition>::Plan,
        >: Operator<S>,
    {
        let (extractor_d, condition) = target;
        let tree = JoinNode::new(self.tree, CollectionNode::new(extractor_d, 3), condition);
        crate::stream::cross_quad_stream::Quad {
            tree,
            filter: TriAsQuadFilter::new(self.filter),
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> Builder<S, A, B, C, EA, EB, EC, P1, P2, F, impl Fn(&A, &B, &C) -> Sc + Send + Sync, Sc>
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
    ) -> Builder<S, A, B, C, EA, EB, EC, P1, P2, F, impl Fn(&A, &B, &C) -> Sc + Send + Sync, Sc>
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

/* Zero-erasure builder for finalizing a canonical tri constraint. */
pub struct Builder<S, A, B, C, EA, EB, EC, P1, P2, F, W, Sc>
where
    Sc: Score,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
{
    tree: BuiltTri<S, EA, EB, EC, P1, P2>,
    filter: F,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> C, fn() -> Sc)>,
}

impl<S, A, B, C, EA, EB, EC, P1, P2, F, W, Sc> Builder<S, A, B, C, EA, EB, EC, P1, P2, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + std::fmt::Debug + Send + Sync + 'static,
    B: Clone + std::fmt::Debug + Send + Sync + 'static,
    C: Clone + std::fmt::Debug + Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    for<'a> JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, <P1 as CompileCondition>::Plan>:
        Operator<S, View<'a> = Pair<Leaf<'a, A>, Leaf<'a, B>>>,
    for<'a> BuiltTri<S, EA, EB, EC, P1, P2>:
        Operator<S, View<'a> = Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>>,
    for<'a> TriRow<'a, S, EA, EB, EC, P1, P2>: ExplainRow,
    F: TriFilter<S, A, B, C> + 'static,
    W: Fn(&A, &B, &C) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /* Finalizes into the generic operator terminal.

    The authored tri filter runs inside the scored operator so evaluate,
    match_count, initialize, and mutations all honor it exactly once; the
    terminal's weight stays the plain authored per-row weight.
    */
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<
        S,
        TriScored<S, A, B, C, EA, EB, EC, P1, P2, F>,
        impl Fn(&S, &TriRow<'_, S, EA, EB, EC, P1, P2>) -> Sc + Send + Sync,
        Sc,
    > {
        let weight = self.weight;
        let is_hard = self.is_hard;
        let constraint_ref = ConstraintRef::new("", name);
        let scored = TriScored {
            inner: self.tree,
            filter: self.filter,
            accepted: HandleMap::new(),
            marker: PhantomData,
        };
        let weight_fn = move |_: &S, row: &TriRow<'_, S, EA, EB, EC, P1, P2>| {
            let (a, b, c) = TriEntities::entities(row);
            weight(a, b, c)
        };
        OperatorTerminal::new(constraint_ref, self.impact_type, scored, weight_fn, is_hard)
    }
}

/* Borrowed (A, B, C) entity triple off a tri row view. */
pub trait TriEntities<'a, A, B, C> {
    fn entities(&self) -> (&'a A, &'a B, &'a C);
    fn indexes(&self) -> (usize, usize, usize);
}

impl<'a, A, B, C> TriEntities<'a, A, B, C> for Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>> {
    #[inline]
    fn entities(&self) -> (&'a A, &'a B, &'a C) {
        (
            self.left.left.entity,
            self.left.right.entity,
            self.right.entity,
        )
    }
    #[inline]
    fn indexes(&self) -> (usize, usize, usize) {
        (
            self.left.left.index,
            self.left.right.index,
            self.right.index,
        )
    }
}

/* Scored tri operator: the tree plus the authored filter as one row producer.

The terminal's weight closure receives filtered rows only. Filters never
run twice: stateless evaluation applies them here, and retained mutations
publish post-filter deltas through the same predicate.
*/
pub struct TriScored<
    S,
    A,
    B,
    C,
    EA,
    EB,
    EC,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    F,
> where
    BuiltTri<S, EA, EB, EC, P1, P2>: Operator<S>,
    F: TriFilter<S, A, B, C>,
{
    inner: BuiltTri<S, EA, EB, EC, P1, P2>,
    filter: F,
    accepted: HandleMap<()>,
    marker: PhantomData<fn() -> (S, A, B, C)>,
}

impl<S, A, B, C, EA, EB, EC, P1, P2, F> Operator<S> for TriScored<S, A, B, C, EA, EB, EC, P1, P2, F>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, <P1 as CompileCondition>::Plan>:
        Operator<S, View<'a> = Pair<Leaf<'a, A>, Leaf<'a, B>>>,
    for<'a> BuiltTri<S, EA, EB, EC, P1, P2>:
        Operator<S, View<'a> = Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>>,
    F: TriFilter<S, A, B, C> + 'static,
{
    type View<'a> = TriRow<'a, S, EA, EB, EC, P1, P2>;
    type Evaluation = <BuiltTri<S, EA, EB, EC, P1, P2> as Operator<S>>::Evaluation;

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
                let (a, b, c) = row.entities();
                let (ai, bi, ci) = row.indexes();
                if self.filter.test(solution, a, b, c, ai, bi, ci) {
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
            let row = self.inner.resolve(solution, handle).expect("live tri row");
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
            let row = self.inner.resolve(solution, handle).expect("live tri row");
            if self.test(solution, row) {
                self.accepted.insert(handle, ());
                accepted.push(handle);
            }
        }
        changes.inserted = accepted;
        changes
    }
}

impl<S, A, B, C, EA, EB, EC, P1, P2, F> TriScored<S, A, B, C, EA, EB, EC, P1, P2, F>
where
    S: Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    BuiltTri<S, EA, EB, EC, P1, P2>: Operator<S>,
    F: TriFilter<S, A, B, C>,
{
    #[inline]
    fn test<'a>(
        &self,
        solution: &S,
        row: <BuiltTri<S, EA, EB, EC, P1, P2> as Operator<S>>::View<'a>,
    ) -> bool
    where
        A: 'a,
        B: 'a,
        C: 'a,
        <BuiltTri<S, EA, EB, EC, P1, P2> as Operator<S>>::View<'a>: TriEntities<'a, A, B, C>,
    {
        let (a, b, c) = row.entities();
        let (ai, bi, ci) = row.indexes();
        self.filter.test(solution, a, b, c, ai, bi, ci)
    }
}
