//! Zero-erasure if_exists/if_not_exists constraint stream.
//!
//! A `IfExistsStream` is created from `UniConstraintStream::if_exists()` or
//! `if_not_exists()` and provides filtering, weighting, and finalization.

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::if_exists::{ExistenceMode, IfExistsUniConstraint};

use super::filter::UniFilter;

/// Zero-erasure stream for building if_exists/if_not_exists constraints.
///
/// Created by `UniConstraintStream::if_exists()` or `if_not_exists()`.
/// Filters A entities based on whether a matching B exists.
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Primary entity type (scored)
/// - `B` - Secondary entity type (checked for existence)
/// - `K` - Join key type
/// - `EA` - Extractor for A entities
/// - `EB` - Extractor for B entities (returns Vec for filtering)
/// - `KA` - Key extractor for A
/// - `KB` - Key extractor for B
/// - `FA` - Filter on A entities
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::ConstraintFactory;
/// use solverforge_scoring::stream::joiner::equal_bi;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Shift { id: usize, employee_idx: Option<usize> }
///
/// #[derive(Clone)]
/// struct Employee { id: usize, on_vacation: bool }
///
/// #[derive(Clone)]
/// struct Schedule { shifts: Vec<Shift>, employees: Vec<Employee> }
///
/// // Penalize shifts assigned to employees who are on vacation
/// let constraint = ConstraintFactory::<Schedule, SimpleScore>::new()
///     .for_each(|s: &Schedule| s.shifts.as_slice())
///     .filter(|shift: &Shift| shift.employee_idx.is_some())
///     .if_exists_filtered(
///         |s: &Schedule| s.employees.iter().filter(|e| e.on_vacation).cloned().collect(),
///         equal_bi(
///             |shift: &Shift| shift.employee_idx,
///             |emp: &Employee| Some(emp.id),
///         ),
///     )
///     .penalize(SimpleScore::of(1))
///     .as_constraint("Vacation conflict");
///
/// let schedule = Schedule {
///     shifts: vec![
///         Shift { id: 0, employee_idx: Some(0) },
///         Shift { id: 1, employee_idx: Some(1) },
///         Shift { id: 2, employee_idx: None },
///     ],
///     employees: vec![
///         Employee { id: 0, on_vacation: true },
///         Employee { id: 1, on_vacation: false },
///     ],
/// };
///
/// // Shift 0 is assigned to employee 0 who is on vacation
/// assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
/// ```
pub struct IfExistsStream<S, A, B, K, EA, EB, KA, KB, FA, Sc>
where
    Sc: Score,
{
    mode: ExistenceMode,
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter_a: FA,
    _phantom: PhantomData<(S, A, B, K, Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, FA, Sc> IfExistsStream<S, A, B, K, EA, EB, KA, KB, FA, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> Vec<B> + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    FA: UniFilter<A>,
    Sc: Score + 'static,
{
    /// Creates a new if_exists stream.
    pub(crate) fn new(
        mode: ExistenceMode,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter_a: FA,
    ) -> Self {
        Self {
            mode,
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter_a,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each matching entity with a fixed weight.
    pub fn penalize(
        self,
        weight: Sc,
    ) -> IfExistsBuilder<S, A, B, K, EA, EB, KA, KB, FA, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Clone,
    {
        let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
        IfExistsBuilder {
            mode: self.mode,
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter_a: self.filter_a,
            impact_type: ImpactType::Penalty,
            weight: move |_: &A| weight.clone(),
            is_hard,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each matching entity with a dynamic weight.
    pub fn penalize_with<W>(self, weight_fn: W) -> IfExistsBuilder<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        IfExistsBuilder {
            mode: self.mode,
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter_a: self.filter_a,
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each matching entity with a dynamic weight, explicitly marked as hard.
    pub fn penalize_hard_with<W>(self, weight_fn: W) -> IfExistsBuilder<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        IfExistsBuilder {
            mode: self.mode,
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter_a: self.filter_a,
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    /// Rewards each matching entity with a fixed weight.
    pub fn reward(
        self,
        weight: Sc,
    ) -> IfExistsBuilder<S, A, B, K, EA, EB, KA, KB, FA, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Clone,
    {
        let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
        IfExistsBuilder {
            mode: self.mode,
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter_a: self.filter_a,
            impact_type: ImpactType::Reward,
            weight: move |_: &A| weight.clone(),
            is_hard,
            _phantom: PhantomData,
        }
    }

    /// Rewards each matching entity with a dynamic weight.
    pub fn reward_with<W>(self, weight_fn: W) -> IfExistsBuilder<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        IfExistsBuilder {
            mode: self.mode,
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter_a: self.filter_a,
            impact_type: ImpactType::Reward,
            weight: weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    /// Rewards each matching entity with a dynamic weight, explicitly marked as hard.
    pub fn reward_hard_with<W>(self, weight_fn: W) -> IfExistsBuilder<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        IfExistsBuilder {
            mode: self.mode,
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter_a: self.filter_a,
            impact_type: ImpactType::Reward,
            weight: weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, K, EA, EB, KA, KB, FA, Sc: Score> std::fmt::Debug
    for IfExistsStream<S, A, B, K, EA, EB, KA, KB, FA, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IfExistsStream")
            .field("mode", &self.mode)
            .finish()
    }
}

/// Zero-erasure builder for finalizing an if_exists constraint.
pub struct IfExistsBuilder<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
where
    Sc: Score,
{
    mode: ExistenceMode,
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter_a: FA,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(S, A, B, K, Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, FA, W, Sc> IfExistsBuilder<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> Vec<B> + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    FA: UniFilter<A>,
    W: Fn(&A) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Finalizes the builder into a zero-erasure `IfExistsUniConstraint`.
    pub fn as_constraint(
        self,
        name: &str,
    ) -> IfExistsUniConstraint<S, A, B, K, EA, EB, KA, KB, impl Fn(&A) -> bool + Send + Sync, W, Sc>
    {
        let filter = self.filter_a;
        let combined_filter = move |a: &A| filter.test(a);

        IfExistsUniConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.mode,
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            combined_filter,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, B, K, EA, EB, KA, KB, FA, W, Sc: Score> std::fmt::Debug
    for IfExistsBuilder<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IfExistsBuilder")
            .field("mode", &self.mode)
            .field("impact_type", &self.impact_type)
            .finish()
    }
}

