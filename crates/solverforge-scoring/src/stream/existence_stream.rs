/* Semi/anti existence streams over the composable operator tree.

An existence constraint is a semi/anti join: it retains only left rows whose
matching-right count is nonzero (or zero, for `NotExists`). There is no third
retention engine — the relationship is a compiled `EqualJoiner` over the two
operators' views, the left is a collection over the left stream (whose
membership is the stream's own filter), the right is a collection over the
right stream or a `FlattenNode` over it, and scoring is the generic
`OperatorTerminal`.
*/

use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::relational::operator::{ExistenceNode, Operator};
use super::relational::Leaf;
use super::weighting_support::ConstraintWeight;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistenceMode {
    Exists,
    NotExists,
}

/* Zero-erasure existence stream: a left operator, a right operator, and the
compiled relationship between them. `named()` builds the `ExistenceNode` and
its generic terminal. */
pub struct ExistsConstraintStream<S, A, L, R, C, Sc>
where
    C: CompileCondition,
    Sc: Score,
{
    pub(super) mode: ExistenceMode,
    pub(super) left: L,
    pub(super) right: R,
    pub(super) condition: C,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Sc)>,
}

impl<S, A, L, R, C, Sc> ExistsConstraintStream<S, A, L, R, C, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    L: Operator<S>,
    for<'a> L: Operator<S, View<'a> = Leaf<'a, A>>,
    C: CompileCondition,
    Sc: Score + 'static,
{
    pub(super) fn new(mode: ExistenceMode, left: L, right: R, condition: C) -> Self {
        Self {
            mode,
            left,
            right,
            condition,
            _phantom: PhantomData,
        }
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> ExistsConstraintBuilder<S, A, L, R, C, W, Sc>
    where
        W: for<'a> Fn(&S, &Leaf<'a, A>) -> Sc,
    {
        ExistsConstraintBuilder {
            mode: self.mode,
            left: self.left,
            right: self.right,
            condition: self.condition,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        L,
        R,
        C,
        impl for<'a> Fn(&S, &Leaf<'a, A>) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A,), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |_s: &S, row: &Leaf<'_, A>| weight.score((row.entity,)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        L,
        R,
        C,
        impl for<'a> Fn(&S, &Leaf<'a, A>) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A,), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |_s: &S, row: &Leaf<'_, A>| weight.score((row.entity,)),
            is_hard,
        )
    }
}

pub struct ExistsConstraintBuilder<S, A, L, R, C, W, Sc>
where
    C: CompileCondition,
    Sc: Score,
{
    mode: ExistenceMode,
    left: L,
    right: R,
    condition: C,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Sc)>,
}

impl<S, A, L, R, C, W, Sc> ExistsConstraintBuilder<S, A, L, R, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: 'static,
    L: Operator<S>,
    for<'a> L: Operator<S, View<'a> = Leaf<'a, A>>,
    R: Operator<S>,
    C: CompileCondition + 'static,
    C::Plan: IndexedPlan + 'static,
    for<'a> C::Plan: ExecutablePlan<L::View<'a>, R::View<'a>>,
    W: for<'a> Fn(&S, &Leaf<'a, A>) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<
        S,
        ExistenceNode<S, L, R, C::Plan>,
        impl for<'a> Fn(&S, &L::View<'a>) -> Sc + Send + Sync,
        Sc,
    > {
        let exists = matches!(self.mode, ExistenceMode::Exists);
        let node = ExistenceNode::new(self.left, self.right, self.condition, exists);
        OperatorTerminal::new(
            ConstraintRef::new("", name),
            self.impact_type,
            node,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, L, R, C, Sc: Score> std::fmt::Debug for ExistsConstraintStream<S, A, L, R, C, Sc>
where
    C: CompileCondition,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExistsConstraintStream")
            .field("mode", &self.mode)
            .finish()
    }
}

impl<S, A, L, R, C, W, Sc: Score> std::fmt::Debug for ExistsConstraintBuilder<S, A, L, R, C, W, Sc>
where
    C: CompileCondition,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExistsConstraintBuilder")
            .field("mode", &self.mode)
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
