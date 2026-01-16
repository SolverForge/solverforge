//! Zero-erasure grouped constraint stream for penta-arity group-by patterns.

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::collector::PentaCollector;
use crate::constraint::grouped_penta::GroupedPentaConstraint;

/// Zero-erasure constraint stream over grouped entity quintuples.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::ConstraintFactory;
/// use solverforge_scoring::stream::joiner::equal;
/// use solverforge_scoring::stream::collector::penta_count;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// struct Task { team: u32, priority: u32 }
///
/// #[derive(Clone)]
/// struct Solution { tasks: Vec<Task> }
///
/// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
///     .for_each(|s: &Solution| s.tasks.as_slice())
///     .join_self(equal(|t: &Task| t.team))
///     .join_self(equal(|t: &Task| t.team))
///     .join_self(equal(|t: &Task| t.team))
///     .join_self(equal(|t: &Task| t.team))
///     .group_by(
///         |_a: &Task, _b: &Task, _c: &Task, _d: &Task, e: &Task| e.priority,
///         penta_count(),
///     )
///     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
///     .as_constraint("Penta priority clustering");
///
/// let solution = Solution {
///     tasks: vec![
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///     ],
/// };
///
/// // 1 quintuple -> 1 penalty
/// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
/// ```
pub struct GroupedPentaConstraintStream<S, A, GK, JK, E, JKE, Flt, KF, C, Sc>
where
    Sc: Score,
{
    extractor: E,
    join_key_extractor: JKE,
    filter: Flt,
    key_fn: KF,
    collector: C,
    _phantom: PhantomData<(S, A, GK, JK, Sc)>,
}

impl<S, A, GK, JK, E, JKE, Flt, KF, C, Sc>
    GroupedPentaConstraintStream<S, A, GK, JK, E, JKE, Flt, KF, C, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    Flt: Fn(&S, &A, &A, &A, &A, &A) -> bool + Send + Sync,
    KF: Fn(&A, &A, &A, &A, &A) -> GK + Send + Sync,
    C: PentaCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    Sc: Score + 'static,
{
    pub(crate) fn new(
        extractor: E,
        join_key_extractor: JKE,
        filter: Flt,
        key_fn: KF,
        collector: C,
    ) -> Self {
        Self {
            extractor,
            join_key_extractor,
            filter,
            key_fn,
            collector,
            _phantom: PhantomData,
        }
    }

    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedPentaConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedPentaConstraintBuilder {
            extractor: self.extractor,
            join_key_extractor: self.join_key_extractor,
            filter: self.filter,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type: ImpactType::Penalty,
            weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedPentaConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedPentaConstraintBuilder {
            extractor: self.extractor,
            join_key_extractor: self.join_key_extractor,
            filter: self.filter,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type: ImpactType::Penalty,
            weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    pub fn reward_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedPentaConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedPentaConstraintBuilder {
            extractor: self.extractor,
            join_key_extractor: self.join_key_extractor,
            filter: self.filter,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type: ImpactType::Reward,
            weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    pub fn reward_hard_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedPentaConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedPentaConstraintBuilder {
            extractor: self.extractor,
            join_key_extractor: self.join_key_extractor,
            filter: self.filter,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type: ImpactType::Reward,
            weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, GK, JK, E, JKE, Flt, KF, C, Sc: Score> std::fmt::Debug
    for GroupedPentaConstraintStream<S, A, GK, JK, E, JKE, Flt, KF, C, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedPentaConstraintStream").finish()
    }
}

pub struct GroupedPentaConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
where
    Sc: Score,
{
    extractor: E,
    join_key_extractor: JKE,
    filter: Flt,
    key_fn: KF,
    collector: C,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    _phantom: PhantomData<(S, A, GK, JK, Sc)>,
}

impl<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
    GroupedPentaConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    Flt: Fn(&S, &A, &A, &A, &A, &A) -> bool + Send + Sync,
    KF: Fn(&A, &A, &A, &A, &A) -> GK + Send + Sync,
    C: PentaCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn as_constraint(
        self,
        name: &str,
    ) -> GroupedPentaConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc> {
        GroupedPentaConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor,
            self.join_key_extractor,
            self.key_fn,
            self.filter,
            self.collector,
            self.weight_fn,
            self.is_hard,
        )
    }
}

impl<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc: Score> std::fmt::Debug
    for GroupedPentaConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedPentaConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
