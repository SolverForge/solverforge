//! Zero-erasure complemented constraint stream.
//!
//! A `ComplementedConstraintStream` adds entities from a complement source
//! that are not present in grouped results, with default values.

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::collector::UniCollector;
use crate::constraint::complemented::ComplementedGroupConstraint;

/// Zero-erasure constraint stream with complemented groups.
///
/// `ComplementedConstraintStream` results from calling `complement` on a
/// `GroupedConstraintStream`. It ensures all keys from a complement source
/// are represented, using default values for missing keys.
///
/// The key function for A entities returns `Option<K>` to allow skipping
/// entities without valid keys (e.g., unassigned shifts).
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Original entity type (e.g., Shift)
/// - `B` - Complement entity type (e.g., Employee)
/// - `K` - Group key type
/// - `EA` - Extractor for A entities
/// - `EB` - Extractor for B entities (complement source)
/// - `KA` - Key function for A (returns `Option<K>` to allow filtering)
/// - `KB` - Key function for B
/// - `C` - Collector type
/// - `D` - Default value function
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::ConstraintFactory;
/// use solverforge_scoring::stream::collector::count;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Hash, PartialEq, Eq)]
/// struct Employee { id: usize }
///
/// #[derive(Clone, Hash, PartialEq, Eq)]
/// struct Shift { employee_id: usize }
///
/// #[derive(Clone)]
/// struct Schedule {
///     employees: Vec<Employee>,
///     shifts: Vec<Shift>,
/// }
///
/// // Count shifts per employee, including employees with 0 shifts
/// let constraint = ConstraintFactory::<Schedule, SimpleScore>::new()
///     .for_each(|s: &Schedule| &s.shifts)
///     .group_by(|shift: &Shift| shift.employee_id, count())
///     .complement(
///         |s: &Schedule| s.employees.as_slice(),
///         |emp: &Employee| emp.id,
///         |_emp: &Employee| 0usize,
///     )
///     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
///     .as_constraint("Shift count");
///
/// let schedule = Schedule {
///     employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
///     shifts: vec![
///         Shift { employee_id: 0 },
///         Shift { employee_id: 0 },
///         // Employee 1 has 0 shifts, Employee 2 has 0 shifts
///     ],
/// };
///
/// // Employee 0: 2, Employee 1: 0, Employee 2: 0 → Total: -2
/// assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-2));
/// ```
pub struct ComplementedConstraintStream<S, A, B, K, EA, EB, KA, KB, C, D, Sc>
where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    collector: C,
    default_fn: D,
    _phantom: PhantomData<(S, A, B, K, Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, C, D, Sc>
    ComplementedConstraintStream<S, A, B, K, EA, EB, KA, KB, C, D, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    D: Fn(&B) -> C::Result + Send + Sync,
    Sc: Score + 'static,
{
    /// Creates a new complemented constraint stream.
    pub(crate) fn new(
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        collector: C,
        default_fn: D,
    ) -> Self {
        Self {
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            collector,
            default_fn,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each complemented group with a weight based on the result.
    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> ComplementedConstraintBuilder<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        ComplementedConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            collector: self.collector,
            default_fn: self.default_fn,
            impact_type: ImpactType::Penalty,
            weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each complemented group, explicitly marked as hard constraint.
    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> ComplementedConstraintBuilder<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        ComplementedConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            collector: self.collector,
            default_fn: self.default_fn,
            impact_type: ImpactType::Penalty,
            weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    /// Rewards each complemented group with a weight based on the result.
    pub fn reward_with<W>(
        self,
        weight_fn: W,
    ) -> ComplementedConstraintBuilder<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        ComplementedConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            collector: self.collector,
            default_fn: self.default_fn,
            impact_type: ImpactType::Reward,
            weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    /// Rewards each complemented group, explicitly marked as hard constraint.
    pub fn reward_hard_with<W>(
        self,
        weight_fn: W,
    ) -> ComplementedConstraintBuilder<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        ComplementedConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            collector: self.collector,
            default_fn: self.default_fn,
            impact_type: ImpactType::Reward,
            weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, K, EA, EB, KA, KB, C, D, Sc: Score> std::fmt::Debug
    for ComplementedConstraintStream<S, A, B, K, EA, EB, KA, KB, C, D, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComplementedConstraintStream").finish()
    }
}

/// Zero-erasure builder for finalizing a complemented constraint.
pub struct ComplementedConstraintBuilder<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    collector: C,
    default_fn: D,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    _phantom: PhantomData<(S, A, B, K, Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
    ComplementedConstraintBuilder<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    D: Fn(&B) -> C::Result + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Finalizes the builder into a `ComplementedGroupConstraint`.
    pub fn as_constraint(
        self,
        name: &str,
    ) -> ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc> {
        ComplementedGroupConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            self.collector,
            self.default_fn,
            self.weight_fn,
            self.is_hard,
        )
    }
}

impl<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc: Score> std::fmt::Debug
    for ComplementedConstraintBuilder<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComplementedConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
