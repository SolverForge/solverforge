/* Canonical five-source fluent join: one operator tree, no per-arity engine.

A `Penta` owns `JoinNode<JoinNode<JoinNode<JoinNode<A,B>,C>,D>,E>` over compiled
condition plans. Each successive relationship compiles fresh from its joiner,
so every left closure receives the whole borrowed row of what came before it
and every key domain stays independent. Terminal scoring is the generic
`OperatorTerminal`; explanations borrow the full row.
*/

use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::filter::{AndPentaFilter, FnPentaFilter, PentaFilter};
use super::joiner::plan::{CompileCondition, IndexedPlan};
use super::relational::operator::{
    CollectionNode, ExplainRow, JoinNode, Operator, Pair, RowChanges,
};
use super::relational::{HandleMap, Leaf, RowHandle};
use super::weighting_support::ConstraintWeight;
use crate::constraint::relational::OperatorTerminal;

/* The full penta tree: ((((A ⋈ B) ⋈ C) ⋈ D) ⋈ E), each step its own plan. */
pub type BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4> = JoinNode<
    S,
    JoinNode<
        S,
        JoinNode<
            S,
            JoinNode<
                S,
                CollectionNode<S, EA>,
                CollectionNode<S, EB>,
                <P1 as CompileCondition>::Plan,
            >,
            CollectionNode<S, EC>,
            <P2 as CompileCondition>::Plan,
        >,
        CollectionNode<S, ED>,
        <P3 as CompileCondition>::Plan,
    >,
    CollectionNode<S, EE>,
    <P4 as CompileCondition>::Plan,
>;

/* The penta tree's borrowed row: `Pair<Pair<Pair<Pair<Leaf<A>, Leaf<B>>, Leaf<C>>, Leaf<D>>, Leaf<E>>`. */
pub type PentaRow<'a, S, EA, EB, EC, ED, EE, P1, P2, P3, P4> =
    <BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4> as Operator<S>>::View<'a>;

/* Borrowed (A, B, C, D, E) entity quintuple off a penta row view. */
pub trait PentaEntities<'a, A, B, C, D, E> {
    fn entities(&self) -> (&'a A, &'a B, &'a C, &'a D, &'a E);
    fn indexes(&self) -> (usize, usize, usize, usize, usize);
}

impl<'a, A, B, C, D, E> PentaEntities<'a, A, B, C, D, E>
    for Pair<Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>, Leaf<'a, E>>
{
    #[inline]
    fn entities(&self) -> (&'a A, &'a B, &'a C, &'a D, &'a E) {
        (
            self.left.left.left.left.entity,
            self.left.left.left.right.entity,
            self.left.left.right.entity,
            self.left.right.entity,
            self.right.entity,
        )
    }
    #[inline]
    fn indexes(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.left.left.left.left.index,
            self.left.left.left.right.index,
            self.left.left.right.index,
            self.left.right.index,
            self.right.index,
        )
    }
}

pub struct Penta<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F, Sc>
where
    Sc: Score,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    P4: CompileCondition<Plan: IndexedPlan>,
{
    pub(crate) tree: BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>,
    pub(crate) filter: F,
    pub(crate) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> C,
        fn() -> D,
        fn() -> E,
        fn() -> Sc,
    )>,
}

impl<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F, Sc>
    Penta<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    D: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    EA: 'static,
    EB: 'static,
    EC: 'static,
    ED: 'static,
    EE: 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    P3: CompileCondition<Plan: IndexedPlan> + 'static,
    P4: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<S>,
    F: PentaFilter<S, A, B, C, D, E>,
    Sc: Score + 'static,
{
    /* Adds a filter predicate to the stream. */
    #[allow(clippy::type_complexity)]
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> Penta<
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
        AndPentaFilter<
            F,
            FnPentaFilter<
                impl Fn(&S, &A, &B, &C, &D, &E, usize, usize, usize, usize, usize) -> bool + Send + Sync,
            >,
        >,
        Sc,
    >
    where
        P: Fn(&A, &B, &C, &D, &E) -> bool + Send + Sync,
    {
        Penta {
            tree: self.tree,
            filter: AndPentaFilter::new(
                self.filter,
                FnPentaFilter::new(
                    move |_s: &S,
                          a: &A,
                          b: &B,
                          c: &C,
                          d: &D,
                          e: &E,
                          _ai: usize,
                          _bi: usize,
                          _ci: usize,
                          _di: usize,
                          _ei: usize| { predicate(a, b, c, d, e) },
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
    ) -> PentaBuilder<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F, W, Sc>
    where
        W: Fn(&A, &B, &C, &D, &E) -> Sc + Send + Sync,
    {
        PentaBuilder {
            tree: self.tree,
            filter: self.filter,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn penalize<W>(
        self,
        weight: W,
    ) -> PentaBuilder<
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
        F,
        impl Fn(&A, &B, &C, &D, &E) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w B, &'w C, &'w D, &'w E), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |a: &A, b: &B, c: &C, d: &D, e: &E| weight.score((a, b, c, d, e)),
            is_hard,
        )
    }

    #[allow(clippy::type_complexity)]
    pub fn reward<W>(
        self,
        weight: W,
    ) -> PentaBuilder<
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
        F,
        impl Fn(&A, &B, &C, &D, &E) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w B, &'w C, &'w D, &'w E), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |a: &A, b: &B, c: &C, d: &D, e: &E| weight.score((a, b, c, d, e)),
            is_hard,
        )
    }
}

/* Zero-erasure builder for finalizing a canonical penta constraint. */
pub struct PentaBuilder<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F, W, Sc>
where
    Sc: Score,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    P4: CompileCondition<Plan: IndexedPlan>,
{
    tree: BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>,
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
        fn() -> E,
        fn() -> Sc,
    )>,
}

impl<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F, W, Sc>
    PentaBuilder<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + std::fmt::Debug + Send + Sync + 'static,
    B: Clone + std::fmt::Debug + Send + Sync + 'static,
    C: Clone + std::fmt::Debug + Send + Sync + 'static,
    D: Clone + std::fmt::Debug + Send + Sync + 'static,
    E: Clone + std::fmt::Debug + Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    P4: CompileCondition<Plan: IndexedPlan>,
    for<'a> BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<
        S,
        View<'a> = Pair<
            Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
            Leaf<'a, E>,
        >,
    >,
    for<'a> PentaRow<'a, S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: ExplainRow,
    F: PentaFilter<S, A, B, C, D, E> + 'static,
    W: Fn(&A, &B, &C, &D, &E) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<
        S,
        PentaScored<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F>,
        impl Fn(&S, &PentaRow<'_, S, EA, EB, EC, ED, EE, P1, P2, P3, P4>) -> Sc + Send + Sync,
        Sc,
    > {
        let weight = self.weight;
        let is_hard = self.is_hard;
        let constraint_ref = ConstraintRef::new("", name);
        let scored = PentaScored {
            inner: self.tree,
            filter: self.filter,
            accepted: HandleMap::new(),
            marker: PhantomData,
        };
        let weight_fn = move |_: &S, row: &PentaRow<'_, S, EA, EB, EC, ED, EE, P1, P2, P3, P4>| {
            let (a, b, c, d, e) = PentaEntities::entities(row);
            weight(a, b, c, d, e)
        };
        OperatorTerminal::new(constraint_ref, self.impact_type, scored, weight_fn, is_hard)
    }
}

/* Scored penta operator: the tree plus the authored filter as one row producer. */
pub struct PentaScored<
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
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    P4: CompileCondition<Plan: IndexedPlan>,
    F,
> where
    BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<S>,
    F: PentaFilter<S, A, B, C, D, E>,
{
    inner: BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>,
    filter: F,
    accepted: HandleMap<()>,
    marker: PhantomData<fn() -> (S, A, B, C, D, E)>,
}

impl<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F> Operator<S>
    for PentaScored<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    D: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    P3: CompileCondition<Plan: IndexedPlan> + 'static,
    P4: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<
        S,
        View<'a> = Pair<
            Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
            Leaf<'a, E>,
        >,
    >,
    F: PentaFilter<S, A, B, C, D, E> + 'static,
{
    type View<'a> = PentaRow<'a, S, EA, EB, EC, ED, EE, P1, P2, P3, P4>;
    type Evaluation =
        <BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4> as Operator<S>>::Evaluation;

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
                let (a, b, c, d, e) = row.entities();
                let (ai, bi, ci, di, ei) = row.indexes();
                if self
                    .filter
                    .test(solution, a, b, c, d, e, ai, bi, ci, di, ei)
                {
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
            let row = self
                .inner
                .resolve(solution, handle)
                .expect("live penta row");
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
            let row = self
                .inner
                .resolve(solution, handle)
                .expect("live penta row");
            if self.test(solution, row) {
                self.accepted.insert(handle, ());
                accepted.push(handle);
            }
        }
        changes.inserted = accepted;
        changes
    }
}

impl<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F>
    PentaScored<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F>
where
    S: Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    P4: CompileCondition<Plan: IndexedPlan>,
    BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<S>,
    F: PentaFilter<S, A, B, C, D, E>,
{
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn test<'a>(
        &self,
        solution: &S,
        row: <BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4> as Operator<S>>::View<'a>,
    ) -> bool
    where
        A: 'a,
        B: 'a,
        C: 'a,
        D: 'a,
        E: 'a,
        <BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4> as Operator<S>>::View<'a>:
            PentaEntities<'a, A, B, C, D, E>,
    {
        let (a, b, c, d, e) = row.entities();
        let (ai, bi, ci, di, ei) = row.indexes();
        self.filter
            .test(solution, a, b, c, d, e, ai, bi, ci, di, ei)
    }
}
