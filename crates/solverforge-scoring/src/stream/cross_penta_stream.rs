mod scored;
pub use scored::PentaScored;

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
use super::relational::operator::{CollectionNode, ExplainRow, JoinNode, Operator, Pair};
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
    F: PentaFilter<S, A, B, C, D, E> + 'static,
    Sc: Score + 'static,
    for<'a> BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<
        S,
        View<'a> = Pair<
            Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
            Leaf<'a, E>,
        >,
    >,
    BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Send + Sync,
{
    /// Continues the chain past the named cross-stream arities.
    ///
    /// Returns a `Chain` wrapping this penta's tree; subsequent `.join(...)`
    /// calls nest further `JoinNode`s with no fixed ceiling. The continuation
    /// drops the penta's authored filter slot, so apply `.filter(...)` on the
    /// returned chain instead.
    ///
    /// # Key shapes
    ///
    /// This join also takes VIEW keys, not entity keys: the left key closure
    /// receives the whole borrowed (A, B, C, D, E) row and the right key
    /// receives `&Leaf<'_, G>` (read fields via `.entity`). `equal_bi` with
    /// entity closures does NOT typecheck here (E0631) — use `equal_on`.
    pub fn join<G, EG, P5>(
        self,
        target: (EG, P5),
    ) -> super::chain_stream::Chain<
        S,
        JoinNode<
            S,
            BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>,
            CollectionNode<S, EG>,
            <P5 as CompileCondition>::Plan,
        >,
        Sc,
    >
    where
        G: Clone + Send + Sync + 'static,
        EG: super::collection_extract::CollectionExtract<S, Item = G> + 'static,
        P5: CompileCondition + 'static,
        P5::Plan: IndexedPlan,
        for<'a> P5::Plan: super::joiner::plan::ExecutablePlan<
            PentaRow<'a, S, EA, EB, EC, ED, EE, P1, P2, P3, P4>,
            Leaf<'a, G>,
        >,
        for<'a> JoinNode<
            S,
            BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>,
            CollectionNode<S, EG>,
            <P5 as CompileCondition>::Plan,
        >: Operator<S>,
    {
        let chain = super::chain_stream::Chain::new(self.tree, 5);
        chain.join(target)
    }
}

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
            child_removed: Vec::new(),
            child_inserted: Vec::new(),
            marker: PhantomData,
        };
        let weight_fn = move |_: &S, row: &PentaRow<'_, S, EA, EB, EC, ED, EE, P1, P2, P3, P4>| {
            let (a, b, c, d, e) = PentaEntities::entities(row);
            weight(a, b, c, d, e)
        };
        OperatorTerminal::new(constraint_ref, self.impact_type, scored, weight_fn, is_hard)
    }
}
