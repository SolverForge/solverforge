//! Zero-erasure grouped cross-bi-constraint stream for aggregated cross-entity pair scoring.
//!
//! A `GroupedCrossBiConstraintStream` operates on aggregated groups of cross-entity pairs (A, B)
//! and supports filtering, weighting, and constraint finalization.
//!
//! # Example
//!
//! ```
//! use solverforge_scoring::stream::ConstraintFactory;
//! use solverforge_scoring::stream::joiner::equal_bi;
//! use solverforge_scoring::stream::collector::cross_bi_count;
//! use solverforge_scoring::api::constraint_set::IncrementalConstraint;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug, Hash, PartialEq, Eq)]
//! struct Shift { day: u32 }
//!
//! #[derive(Clone, Debug, Hash, PartialEq, Eq)]
//! struct Employee { skill: u32 }
//!
//! #[derive(Clone)]
//! struct Solution { shifts: Vec<Shift>, employees: Vec<Employee> }
//!
//! // Count shift-employee pairs per day, penalize imbalanced days
//! let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
//!     .for_each(|s: &Solution| s.shifts.as_slice())
//!     .join(|s: &Solution| s.employees.as_slice(), equal_bi(|sh: &Shift| sh.day, |_: &Employee| 1u32))
//!     .group_by(
//!         |sh: &Shift, _emp: &Employee| sh.day,
//!         cross_bi_count(),
//!     )
//!     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
//!     .as_constraint("Assignments per day");
//!
//! let solution = Solution {
//!     shifts: vec![Shift { day: 1 }, Shift { day: 1 }],
//!     employees: vec![Employee { skill: 1 }],
//! };
//!
//! // 2 shifts × 1 employee on day 1 = 2 pairs -> -2 penalty
//! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-2));
//! ```

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::grouped_cross_bi::GroupedCrossBiConstraint;
use crate::stream::collector::CrossBiCollector;

/// A constraint stream operating on grouped cross-entity pairs.
pub struct GroupedCrossBiConstraintStream<S, A, B, GK, JK, EA, EB, KA, KB, F, KF, C, Sc> {
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    group_key_fn: KF,
    collector: C,
    _phantom: PhantomData<fn(&S, &A, &B) -> (GK, JK, Sc)>,
}

impl<S, A, B, GK, JK, EA, EB, KA, KB, F, KF, C, Sc>
    GroupedCrossBiConstraintStream<S, A, B, GK, JK, EA, EB, KA, KB, F, KF, C, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    B: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: Fn(&S, &A, &B) -> bool + Send + Sync,
    KF: Fn(&A, &B) -> GK + Send + Sync,
    C: CrossBiCollector<A, B> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    Sc: Score + 'static,
{
    /// Creates a new grouped cross-bi-constraint stream.
    pub fn new(
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
        group_key_fn: KF,
        collector: C,
    ) -> Self {
        Self {
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter,
            group_key_fn,
            collector,
            _phantom: PhantomData,
        }
    }

    /// Applies a penalty based on the aggregated group result.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_scoring::stream::ConstraintFactory;
    /// use solverforge_scoring::stream::joiner::equal_bi;
    /// use solverforge_scoring::stream::collector::cross_bi_count;
    /// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Task { project: u32 }
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Worker { dept: u32 }
    ///
    /// #[derive(Clone)]
    /// struct Solution { tasks: Vec<Task>, workers: Vec<Worker> }
    ///
    /// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    ///     .for_each(|s: &Solution| s.tasks.as_slice())
    ///     .join(|s: &Solution| s.workers.as_slice(), equal_bi(|_: &Task| 1u32, |_: &Worker| 1u32))
    ///     .group_by(
    ///         |t: &Task, _w: &Worker| t.project,
    ///         cross_bi_count(),
    ///     )
    ///     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
    ///     .as_constraint("Tasks per project");
    ///
    /// let solution = Solution {
    ///     tasks: vec![Task { project: 1 }, Task { project: 1 }],
    ///     workers: vec![Worker { dept: 1 }],
    /// };
    ///
    /// // 2 tasks × 1 worker = 2 pairs, grouped by project 1 -> -2
    /// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-2));
    /// ```
    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedCrossBiConstraintBuilder<S, A, B, GK, JK, EA, EB, KA, KB, F, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedCrossBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            group_key_fn: self.group_key_fn,
            collector: self.collector,
            weight_fn,
            is_penalty: true,
            _phantom: PhantomData,
        }
    }

    /// Applies a reward based on the aggregated group result.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_scoring::stream::ConstraintFactory;
    /// use solverforge_scoring::stream::joiner::equal_bi;
    /// use solverforge_scoring::stream::collector::cross_bi_count;
    /// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Resource { category: u32 }
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Consumer { need: u32 }
    ///
    /// #[derive(Clone)]
    /// struct Solution { resources: Vec<Resource>, consumers: Vec<Consumer> }
    ///
    /// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    ///     .for_each(|s: &Solution| s.resources.as_slice())
    ///     .join(|s: &Solution| s.consumers.as_slice(), equal_bi(|_: &Resource| 1u32, |_: &Consumer| 1u32))
    ///     .group_by(
    ///         |r: &Resource, _c: &Consumer| r.category,
    ///         cross_bi_count(),
    ///     )
    ///     .reward_with(|count: &usize| SimpleScore::of(*count as i64))
    ///     .as_constraint("Resource utilization");
    ///
    /// let solution = Solution {
    ///     resources: vec![Resource { category: 1 }],
    ///     consumers: vec![Consumer { need: 1 }, Consumer { need: 1 }],
    /// };
    ///
    /// // 1 resource × 2 consumers = 2 pairs -> +2
    /// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(2));
    /// ```
    pub fn reward_with<W>(
        self,
        weight_fn: W,
    ) -> GroupedCrossBiConstraintBuilder<S, A, B, GK, JK, EA, EB, KA, KB, F, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        GroupedCrossBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            group_key_fn: self.group_key_fn,
            collector: self.collector,
            weight_fn,
            is_penalty: false,
            _phantom: PhantomData,
        }
    }
}

/// Builder for grouped cross-bi constraints.
pub struct GroupedCrossBiConstraintBuilder<S, A, B, GK, JK, EA, EB, KA, KB, F, KF, C, W, Sc> {
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    group_key_fn: KF,
    collector: C,
    weight_fn: W,
    is_penalty: bool,
    _phantom: PhantomData<fn(&S, &A, &B) -> (GK, JK, Sc)>,
}

impl<S, A, B, GK, JK, EA, EB, KA, KB, F, KF, C, W, Sc>
    GroupedCrossBiConstraintBuilder<S, A, B, GK, JK, EA, EB, KA, KB, F, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    B: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: Fn(&S) -> &[A] + Send + Sync + 'static,
    EB: Fn(&S) -> &[B] + Send + Sync + 'static,
    KA: Fn(&A) -> JK + Send + Sync + 'static,
    KB: Fn(&B) -> JK + Send + Sync + 'static,
    F: Fn(&S, &A, &B) -> bool + Send + Sync + 'static,
    KF: Fn(&A, &B) -> GK + Send + Sync + 'static,
    C: CrossBiCollector<A, B> + Send + Sync + 'static,
    C::Value: Clone + Send + Sync,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync + 'static,
    Sc: Score + 'static,
{
    /// Finalizes the constraint with a name.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_scoring::stream::ConstraintFactory;
    /// use solverforge_scoring::stream::joiner::equal_bi;
    /// use solverforge_scoring::stream::collector::cross_bi_count;
    /// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Item { group: u32 }
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Slot { capacity: u32 }
    ///
    /// #[derive(Clone)]
    /// struct Solution { items: Vec<Item>, slots: Vec<Slot> }
    ///
    /// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    ///     .for_each(|s: &Solution| s.items.as_slice())
    ///     .join(|s: &Solution| s.slots.as_slice(), equal_bi(|_: &Item| 1u32, |_: &Slot| 1u32))
    ///     .group_by(
    ///         |i: &Item, _s: &Slot| i.group,
    ///         cross_bi_count(),
    ///     )
    ///     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
    ///     .as_constraint("Item-slot pairs");
    ///
    /// assert_eq!(constraint.name(), "Item-slot pairs");
    /// ```
    pub fn as_constraint(
        self,
        name: &str,
    ) -> GroupedCrossBiConstraint<S, A, B, GK, JK, EA, EB, KA, KB, KF, F, C, W, Sc>
    {
        let impact_type = if self.is_penalty {
            ImpactType::Penalty
        } else {
            ImpactType::Reward
        };
        GroupedCrossBiConstraint::new(
            ConstraintRef::new("", name),
            impact_type,
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            self.group_key_fn,
            self.filter,
            self.collector,
            self.weight_fn,
            false, // is_hard
        )
    }
}
