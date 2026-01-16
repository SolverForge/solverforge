//! Zero-erasure grouped constraint stream for bi-arity group-by patterns.
//!
//! A `GroupedBiConstraintStream` operates on groups of entity pairs and supports
//! filtering, weighting, and constraint finalization.

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::collector::BiCollector;
use crate::constraint::grouped_bi::GroupedBiConstraint;

/// Zero-erasure constraint stream over grouped entity pairs.
///
/// `GroupedBiConstraintStream` is created by `BiConstraintStream::group_by()`
/// and operates on (key, collector_result) tuples.
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Entity type
/// - `GK` - Group key type
/// - `JK` - Join key type (for self-join matching)
/// - `E` - Extractor function for entities
/// - `JKE` - Join key extractor
/// - `Flt` - Filter predicate
/// - `KF` - Group key function
/// - `C` - Collector type
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::ConstraintFactory;
/// use solverforge_scoring::stream::joiner::equal;
/// use solverforge_scoring::stream::collector::bi_count;
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
///     .group_by(
///         |_a: &Task, b: &Task| b.priority,
///         bi_count(),
///     )
///     .penalize_with(|count: &usize| SimpleScore::of((*count * *count) as i64))
///     .as_constraint("Priority clustering");
///
/// let solution = Solution {
///     tasks: vec![
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///     ],
/// };
///
/// // 3 tasks on team 1, all priority 1: 3 pairs -> 9 penalty
/// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-9));
/// ```
pub struct GroupedBiConstraintStream<S, A, GK, JK, E, JKE, Flt, KF, C, Sc>
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
    GroupedBiConstraintStream<S, A, GK, JK, E, JKE, Flt, KF, C, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    Flt: Fn(&S, &A, &A) -> bool + Send + Sync,
    KF: Fn(&A, &A) -> GK + Send + Sync,
    C: BiCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    Sc: Score + 'static,
{
    /// Creates a new zero-erasure grouped bi-constraint stream.
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

    /// Penalizes each group with a weight based on the collector result.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_scoring::stream::ConstraintFactory;
    /// use solverforge_scoring::stream::joiner::equal;
    /// use solverforge_scoring::stream::collector::bi_count;
    /// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Item { group: u32 }
    ///
    /// #[derive(Clone)]
    /// struct Solution { items: Vec<Item> }
    ///
    /// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    ///     .for_each(|s: &Solution| s.items.as_slice())
    ///     .join_self(equal(|i: &Item| i.group))
    ///     .group_by(|_a: &Item, b: &Item| b.group, bi_count())
    ///     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
    ///     .as_constraint("Group pairs");
    ///
    /// let solution = Solution {
    ///     items: vec![
    ///         Item { group: 1 },
    ///         Item { group: 1 },
    ///     ],
    /// };
    ///
    /// // 1 pair -> -1 penalty
    /// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
    /// ```
    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedBiConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedBiConstraintBuilder {
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

    /// Penalizes each group with a weight, explicitly marked as hard constraint.
    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedBiConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedBiConstraintBuilder {
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

    /// Rewards each group with a weight based on the collector result.
    pub fn reward_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedBiConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedBiConstraintBuilder {
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

    /// Rewards each group with a weight, explicitly marked as hard constraint.
    pub fn reward_hard_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedBiConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedBiConstraintBuilder {
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
    for GroupedBiConstraintStream<S, A, GK, JK, E, JKE, Flt, KF, C, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedBiConstraintStream").finish()
    }
}

/// Zero-erasure builder for finalizing a grouped bi-constraint.
pub struct GroupedBiConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
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
    GroupedBiConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    Flt: Fn(&S, &A, &A) -> bool + Send + Sync,
    KF: Fn(&A, &A) -> GK + Send + Sync,
    C: BiCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Finalizes the builder into a zero-erasure `GroupedBiConstraint`.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_scoring::stream::ConstraintFactory;
    /// use solverforge_scoring::stream::joiner::equal;
    /// use solverforge_scoring::stream::collector::bi_count;
    /// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Task { team: u32 }
    ///
    /// #[derive(Clone)]
    /// struct Solution { tasks: Vec<Task> }
    ///
    /// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    ///     .for_each(|s: &Solution| s.tasks.as_slice())
    ///     .join_self(equal(|t: &Task| t.team))
    ///     .group_by(|_a: &Task, b: &Task| b.team, bi_count())
    ///     .penalize_with(|n: &usize| SimpleScore::of(*n as i64))
    ///     .as_constraint("Team penalty");
    ///
    /// assert_eq!(constraint.name(), "Team penalty");
    /// ```
    pub fn as_constraint(
        self,
        name: &str,
    ) -> GroupedBiConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc> {
        GroupedBiConstraint::new(
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
    for GroupedBiConstraintBuilder<S, A, GK, JK, E, JKE, Flt, KF, C, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedBiConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
