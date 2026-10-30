//! Monomorphized constraint set for zero-erasure incremental scoring.
//!
//! This module provides the `ConstraintSet` trait which enables fully
//! monomorphized constraint evaluation without virtual dispatch.

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::super::analysis::{ConstraintAnalysis, DetailedConstraintMatch};
use super::weighting::{scale_contribution, scale_contribution_parts};
use crate::constraint::score_sum::checked;

/// Marks a type as one constraint.
///
/// Every [`IncrementalConstraint`] is also a one-constraint [`ConstraintSet`]
/// through a blanket impl. Tuples of constraint sets and constraint-set
/// chains are constraint sets through their own impls, and without this
/// marker the compiler could not tell those impls apart from the blanket one.
/// A hand-written constraint adds the empty
/// `impl SingleConstraint for MyConstraint {}` next to its
/// `IncrementalConstraint` impl. Never implement it for a tuple, a chain or
/// any other type that holds several constraints: that type is already a
/// `ConstraintSet`, and the marker would claim it is one constraint.
pub trait SingleConstraint {}

/// One named constraint whose score is kept up to date incrementally.
///
/// The fluent constraint stream API builds these; implement it directly for
/// a constraint the streams cannot express. A score director drives it as
/// follows, and an implementation must uphold every point:
///
/// - `initialize` is called on a fresh or [`reset`](Self::reset) constraint
///   before any notification, and returns the same score as
///   [`evaluate`](Self::evaluate) on that solution.
/// - When a planning variable of entity `entity_index` in the entity class
///   `descriptor_index` changes, `on_retract` is called once with the
///   solution still in its old state, and `on_insert` once after the change.
///   `on_retract` returns the negation of the score the entity's matches
///   currently contribute and `on_insert` the score they contribute after the
///   change. `initialize` plus every delta returned since must always equal
///   `evaluate` on the current solution.
/// - Entity indexes are local to their entity class. A notification for an
///   entity class the constraint does not read returns zero.
/// - `reset` drops all retained state; `initialize` is called again before
///   the next notification.
/// - `constraint_ref` returns the same reference for the constraint's whole
///   life, and `is_hard` agrees with every other constraint sharing that
///   full reference; conflicting hardness panics when the set's metadata is
///   built.
///
/// See `docs/extend-scoring.md` for a worked example.
pub trait IncrementalConstraint<S, Sc: Score>: SingleConstraint + Send + Sync {
    /// Computes this constraint's whole score from scratch, without reading
    /// or changing retained state.
    fn evaluate(&self, solution: &S) -> Sc;

    /// Returns the number of matches for this constraint in `solution`.
    fn match_count(&self, solution: &S) -> usize;

    /// Builds retained state from `solution` and returns the constraint's
    /// whole score, equal to [`evaluate`](Self::evaluate).
    fn initialize(&mut self, solution: &S) -> Sc;

    /// Called after entity `entity_index` of entity class `descriptor_index`
    /// changed; returns the score its matches now add.
    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    /// Called before entity `entity_index` of entity class
    /// `descriptor_index` changes; returns the negation of the score its
    /// matches currently contribute.
    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    /// `on_insert` returning the delta unchecked with a flag that is set when a
    /// level overflowed, so an enclosing set or director can sum several deltas
    /// and check once. The default delegates to `on_insert`, which checks itself.
    #[doc(hidden)]
    #[inline]
    fn on_insert_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
    ) -> (Sc, bool) {
        (
            self.on_insert(solution, entity_index, descriptor_index),
            false,
        )
    }

    /// `on_retract` returning the delta unchecked with a flag that is set when a
    /// level overflowed, so an enclosing set or director can sum several deltas
    /// and check once. The default delegates to `on_retract`, which checks itself.
    #[doc(hidden)]
    #[inline]
    fn on_retract_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
    ) -> (Sc, bool) {
        (
            self.on_retract(solution, entity_index, descriptor_index),
            false,
        )
    }

    /// Drops all retained state. `initialize` runs again before the next
    /// notification.
    fn reset(&mut self);

    /// Returns the constraint's reference (package and name), the same for
    /// its whole life.
    fn constraint_ref(&self) -> &ConstraintRef;

    /// Returns the constraint's name.
    fn name(&self) -> &str {
        &self.constraint_ref().name
    }

    /// Returns whether this is a hard constraint. Every constraint sharing
    /// this constraint's full reference must agree.
    fn is_hard(&self) -> bool {
        false
    }

    /// Returns each match with the entities that justify it, for score
    /// explanation. The default returns no matches.
    fn get_matches<'a>(&'a self, _solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>> {
        Vec::new()
    }

    /// Weighted counterparts of the scoring methods.
    ///
    /// `WeightedMode` score directors call these instead of the unweighted
    /// methods. Each receives this constraint's bound multiplier and the
    /// defaults scale the whole contribution by it, which is exact because
    /// integer scaling is linear: the scaled sum of match scores equals the sum of
    /// scaled match scores. Retained state therefore stays unweighted, and the
    /// unweighted methods never see a multiplier.
    #[inline]
    fn evaluate_weighted(&self, solution: &S, multiplier: u32) -> Sc {
        scale_contribution(self.evaluate(solution), multiplier)
    }

    /// Weighted [`initialize`](Self::initialize): the initial score scaled by `multiplier`.
    #[inline]
    fn initialize_weighted(&mut self, solution: &S, multiplier: u32) -> Sc {
        scale_contribution(self.initialize(solution), multiplier)
    }

    /// Weighted [`on_insert`](Self::on_insert): the insert delta scaled by `multiplier`.
    #[inline]
    fn on_insert_weighted(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multiplier: u32,
    ) -> Sc {
        checked(self.on_insert_weighted_overflowing(
            solution,
            entity_index,
            descriptor_index,
            multiplier,
        ))
    }

    /// `on_insert_weighted` returning the contribution unchecked with an overflow flag;
    /// see `on_insert_overflowing`.
    #[doc(hidden)]
    #[inline]
    fn on_insert_weighted_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multiplier: u32,
    ) -> (Sc, bool) {
        scale_contribution_parts(
            self.on_insert_overflowing(solution, entity_index, descriptor_index),
            multiplier,
        )
    }

    /// Weighted [`on_retract`](Self::on_retract): the retract delta scaled by `multiplier`.
    #[inline]
    fn on_retract_weighted(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multiplier: u32,
    ) -> Sc {
        checked(self.on_retract_weighted_overflowing(
            solution,
            entity_index,
            descriptor_index,
            multiplier,
        ))
    }

    /// `on_retract_weighted` returning the contribution unchecked with an overflow flag;
    /// see `on_insert_overflowing`.
    #[doc(hidden)]
    #[inline]
    fn on_retract_weighted_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multiplier: u32,
    ) -> (Sc, bool) {
        scale_contribution_parts(
            self.on_retract_overflowing(solution, entity_index, descriptor_index),
            multiplier,
        )
    }

    /// Weighted [`get_matches`](Self::get_matches): every match score scaled by
    /// `multiplier`.
    fn get_matches_weighted<'a>(
        &'a self,
        solution: &S,
        multiplier: u32,
    ) -> Vec<DetailedConstraintMatch<'a, Sc>> {
        let mut matches = self.get_matches(solution);
        for detailed in &mut matches {
            detailed.score = scale_contribution(detailed.score, multiplier);
        }
        matches
    }
}

/// Result of evaluating a single constraint.
#[derive(Debug, Clone)]
pub struct ConstraintResult<'a, Sc> {
    /// Constraint name.
    pub name: &'a str,
    /// Score contribution from this constraint.
    pub score: Sc,
    /// Number of matches for this constraint.
    pub match_count: usize,
    /// Whether this is a hard constraint.
    pub is_hard: bool,
    /// Bound runtime weight multiplier; `0` means the constraint is disabled.
    pub weight_multiplier: u32,
}

/// Immutable public metadata for a scoring constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstraintMetadata<'a> {
    /// The constraint's package-qualified identity, borrowed from the constraint.
    pub constraint_ref: &'a ConstraintRef,
    /// Whether the constraint is hard.
    pub is_hard: bool,
}

impl<'a> ConstraintMetadata<'a> {
    /// Creates metadata borrowing `constraint_ref`.
    pub fn new(constraint_ref: &'a ConstraintRef, is_hard: bool) -> Self {
        Self {
            constraint_ref,
            is_hard,
        }
    }

    /// Returns the short constraint name.
    pub fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    /// Returns the package-qualified name used for exact matching.
    pub fn full_name(&self) -> String {
        self.constraint_ref.full_name()
    }
}

/// A set of constraints that can be evaluated together.
///
/// `ConstraintSet` is implemented for each `IncrementalConstraint` as a singleton
/// set and for tuples of nested `ConstraintSet` values. This keeps composition
/// fully monomorphized without virtual dispatch.
pub trait ConstraintSet<S, Sc: Score>: Send + Sync {
    /// Evaluates all constraints and returns the total score.
    fn evaluate_all(&self, solution: &S) -> Sc;

    /// Returns the number of constraints in this set.
    fn constraint_count(&self) -> usize;

    /// Returns deduplicated immutable metadata for constraints in this set.
    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        deduplicate_constraint_metadata(self.constraint_metadata_entries())
    }

    /// Returns one metadata entry per constraint in authored order.
    #[doc(hidden)]
    fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>>;

    /// Evaluates each constraint individually and returns per-constraint results.
    ///
    /// Useful for score explanation and debugging.
    fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>>;

    /// Evaluates each constraint with detailed match information.
    ///
    /// Returns per-constraint analysis including all matches with entity
    /// justifications. This enables full score explanation features.
    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>>;

    /// Initializes all constraints by inserting all entities.
    ///
    /// Must be called before using incremental methods.
    /// Returns the total score from initialization.
    fn initialize_all(&mut self, solution: &S) -> Sc;

    /// Called when an entity is inserted.
    ///
    /// Returns the total score delta from all constraints.
    ///
    /// # Arguments
    /// * `solution` - The planning solution
    /// * `entity_index` - Index of the entity within its class
    /// * `descriptor_index` - Index of the entity class (descriptor) being modified
    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    /// Called when an entity is retracted.
    ///
    /// Returns the total score delta from all constraints.
    ///
    /// # Arguments
    /// * `solution` - The planning solution
    /// * `entity_index` - Index of the entity within its class
    /// * `descriptor_index` - Index of the entity class (descriptor) being modified
    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    /// `on_insert_all` returning the delta unchecked with a flag that is set when a
    /// level overflowed, so an enclosing set or director can sum several deltas
    /// and check once. The default delegates to `on_insert_all`, which checks itself.
    #[doc(hidden)]
    #[inline]
    fn on_insert_all_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
    ) -> (Sc, bool) {
        (
            self.on_insert_all(solution, entity_index, descriptor_index),
            false,
        )
    }

    /// `on_retract_all` returning the delta unchecked with a flag that is set when a
    /// level overflowed, so an enclosing set or director can sum several deltas
    /// and check once. The default delegates to `on_retract_all`, which checks itself.
    #[doc(hidden)]
    #[inline]
    fn on_retract_all_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
    ) -> (Sc, bool) {
        (
            self.on_retract_all(solution, entity_index, descriptor_index),
            false,
        )
    }

    /// Resets all constraints for a new solving session.
    fn reset_all(&mut self);

    /// Runtime constraint weights.
    ///
    /// A set exposes `constraint_count()` weight slots: one per independently
    /// scored named constraint, in structural order (tuple and chain children
    /// left to right, shared-terminal scorers in authored order).
    /// `constraint_weight_refs` pushes each slot's constraint reference; every
    /// `*_weighted` method receives one multiplier per slot in that order and
    /// scales each slot's contribution by its multiplier. Only `WeightedMode`
    /// score directors call these; `UnweightedMode` directors never name them,
    /// so the unweighted methods stay the whole scoring path of every other
    /// director.
    fn constraint_weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>);

    /// Weighted [`evaluate_all`](Self::evaluate_all): `multipliers` holds one
    /// multiplier per weight slot.
    fn evaluate_all_weighted(&self, solution: &S, multipliers: &[u32]) -> Sc;

    /// Weighted [`evaluate_each`](Self::evaluate_each): weighted scores, with each
    /// result's `weight_multiplier` set from its slot.
    fn evaluate_each_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>>;

    /// Weighted [`evaluate_detailed`](Self::evaluate_detailed): weighted totals and
    /// match scores, with each analysis's `weight_multiplier` set from its slot.
    fn evaluate_detailed_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>>;

    /// Weighted [`initialize_all`](Self::initialize_all).
    fn initialize_all_weighted(&mut self, solution: &S, multipliers: &[u32]) -> Sc;

    /// Weighted [`on_insert_all`](Self::on_insert_all).
    fn on_insert_all_weighted(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> Sc;

    /// Weighted [`on_retract_all`](Self::on_retract_all).
    fn on_retract_all_weighted(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> Sc;
    /// Weighted `on_insert_all_overflowing`.
    #[doc(hidden)]
    #[inline]
    fn on_insert_all_weighted_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> (Sc, bool) {
        (
            self.on_insert_all_weighted(solution, entity_index, descriptor_index, multipliers),
            false,
        )
    }

    /// Weighted `on_retract_all_overflowing`.
    #[doc(hidden)]
    #[inline]
    fn on_retract_all_weighted_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> (Sc, bool) {
        (
            self.on_retract_all_weighted(solution, entity_index, descriptor_index, multipliers),
            false,
        )
    }
}

pub(super) fn push_constraint_metadata<'a>(
    metadata: &mut Vec<ConstraintMetadata<'a>>,
    candidate: ConstraintMetadata<'a>,
) {
    if let Some(existing) = metadata
        .iter()
        .find(|item| item.constraint_ref == candidate.constraint_ref)
    {
        assert_eq!(
            existing.is_hard,
            candidate.is_hard,
            "constraint `{}` has conflicting hard/non-hard metadata",
            candidate.full_name()
        );
        return;
    }
    metadata.push(candidate);
}

pub(super) fn deduplicate_constraint_metadata<'a>(
    candidates: Vec<ConstraintMetadata<'a>>,
) -> Vec<ConstraintMetadata<'a>> {
    let mut metadata = Vec::new();
    for candidate in candidates {
        push_constraint_metadata(&mut metadata, candidate);
    }
    metadata
}
