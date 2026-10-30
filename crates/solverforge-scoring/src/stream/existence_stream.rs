//! Semi/anti existence streams over the count-backed membership engine.
//!
//! An existence constraint is a semi/anti join: it retains only left rows whose
//! matching-right key is alive (or dead, for `NotExists`). There is no third
//! retention engine — the relationship is a named pair of key adapters over the
//! two operators' views, the left is a collection over the left stream (whose
//! membership is the stream's own filter), the right is a collection over the
//! right stream (children expanded in place by the key adapter), and scoring is
//! the generic `OperatorTerminal`.

use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::relational::operator::ExistenceNode;
use super::relational::Leaf;
use super::weighting_support::ConstraintWeight;
use crate::constraint::relational::OperatorTerminal;

/// Whether an existence constraint keeps entities with or without a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistenceMode {
    /// Keep left entities with at least one matching right entity.
    Exists,
    /// Keep left entities with no matching right entity.
    NotExists,
}

/// Zero-erasure existence stream: a left operator, a right operator, and the
/// named key adapters between them. `named()` builds the count-backed
/// `ExistenceNode` and its generic terminal.
pub struct ExistsConstraintStream<S, A, K, L, R, LK, RK, Sc>
where
    Sc: Score,
{
    pub(super) mode: ExistenceMode,
    pub(super) left: L,
    pub(super) right: R,
    pub(super) left_key: LK,
    pub(super) right_keys: RK,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> K, fn() -> Sc)>,
}

impl<S, A, K, L, R, LK, RK, Sc> ExistsConstraintStream<S, A, K, L, R, LK, RK, Sc>
where
    Sc: Score,
{
    pub(super) fn new(
        mode: ExistenceMode,
        left: L,
        right: R,
        left_key: LK,
        right_keys: RK,
    ) -> Self {
        Self {
            mode,
            left,
            right,
            left_key,
            right_keys,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, K, L, R, LK, RK, Sc> ExistsConstraintStream<S, A, K, L, R, LK, RK, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    L: super::relational::operator::Operator<S>,
    for<'a> LK: super::relational::view_plan::ViewKey<L::View<'a>, Key = K>,
    R: super::relational::operator::Operator<S>,
    for<'a> RK: crate::stream::relational::operator::RightKeySet<R::View<'a>, Key = K>,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> ExistsConstraintBuilder<S, A, K, L, R, LK, RK, W, Sc>
    where
        W: for<'a> Fn(&S, &Leaf<'a, A>) -> Sc,
    {
        ExistsConstraintBuilder {
            mode: self.mode,
            left: self.left,
            right: self.right,
            left_key: self.left_key,
            right_keys: self.right_keys,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each match by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched entity returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn penalize<W>(
        self,
        weight: W,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        K,
        L,
        R,
        LK,
        RK,
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

    /// Rewards each match by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched entity returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn reward<W>(
        self,
        weight: W,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        K,
        L,
        R,
        LK,
        RK,
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

/// Builder holding a weighted existence constraint until [`named`](Self::named) finalizes it.
pub struct ExistsConstraintBuilder<S, A, K, L, R, LK, RK, W, Sc>
where
    Sc: Score,
{
    mode: ExistenceMode,
    left: L,
    right: R,
    left_key: LK,
    right_keys: RK,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> K, fn() -> Sc)>,
}

impl<S, A, K, L, R, LK, RK, W, Sc> ExistsConstraintBuilder<S, A, K, L, R, LK, RK, W, Sc>
where
    S: Send + Sync + 'static,
    A: 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    L: super::relational::operator::Operator<S>,
    for<'a> LK: super::relational::view_plan::ViewKey<L::View<'a>, Key = K>,
    R: super::relational::operator::Operator<S>,
    for<'a> RK: crate::stream::relational::operator::RightKeySet<R::View<'a>, Key = K>,
    W: for<'a> Fn(&S, &Leaf<'a, A>) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Names the constraint and builds it as an `OperatorTerminal` over the
    /// count-backed `ExistenceNode`.
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<S, ExistenceNode<S, L, R, K, LK, RK>, W, Sc> {
        let exists = matches!(self.mode, ExistenceMode::Exists);
        let node = ExistenceNode::new(
            self.left,
            self.right,
            self.left_key,
            self.right_keys,
            exists,
        );
        OperatorTerminal::new(
            ConstraintRef::new("", name),
            self.impact_type,
            node,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, K, L, R, LK, RK, Sc: Score> std::fmt::Debug
    for ExistsConstraintStream<S, A, K, L, R, LK, RK, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExistsConstraintStream")
            .field("mode", &self.mode)
            .finish()
    }
}

impl<S, A, K, L, R, LK, RK, W, Sc: Score> std::fmt::Debug
    for ExistsConstraintBuilder<S, A, K, L, R, LK, RK, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExistsConstraintBuilder")
            .field("mode", &self.mode)
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
