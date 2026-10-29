/* Monomorphized constraint set for zero-erasure incremental scoring.

This module provides the `ConstraintSet` trait which enables fully
monomorphized constraint evaluation without virtual dispatch.
*/

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::super::analysis::{ConstraintAnalysis, DetailedConstraintMatch};
use super::weighting::{scale_contribution, scale_contribution_parts};
use crate::constraint::score_sum::checked;

#[doc(hidden)]
pub trait IncrementalConstraintSealed {}

/* A single constraint with incremental scoring capability.

Unlike the trait-object `Constraint` trait, `IncrementalConstraint` is
designed for monomorphized code paths where the concrete type is known.

# Incremental Protocol

The incremental methods allow delta-based score updates:

1. Call `initialize` once to populate internal state
2. Before changing an entity's variable: call `on_retract` with old state
3. After changing the variable: call `on_insert` with new state
4. Score delta = insert_delta - retract_delta

This avoids full re-evaluation on every move.
*/
pub trait IncrementalConstraint<S, Sc: Score>: IncrementalConstraintSealed + Send + Sync {
    /* Full evaluation of this constraint.

    Iterates all entities and computes the total score impact.
    Use this for initial scoring; use `on_insert`/`on_retract` for deltas.
    */
    fn evaluate(&self, solution: &S) -> Sc;

    // Returns the number of matches for this constraint.
    fn match_count(&self, solution: &S) -> usize;

    /* Initializes internal state by inserting all entities.

    Must be called before using incremental methods (`on_insert`/`on_retract`).
    Returns the total score from initialization.
    */
    fn initialize(&mut self, solution: &S) -> Sc;

    /* Called when an entity is inserted or its variable changes.

    Returns the score delta from this insertion.

    # Arguments
    * `solution` - The planning solution
    * `entity_index` - Index of the entity within its class
    * `descriptor_index` - Index of the entity class (descriptor) being modified
    */
    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    /* Called when an entity is retracted or before its variable changes.

    Returns the score delta (negative) from this retraction.

    # Arguments
    * `solution` - The planning solution
    * `entity_index` - Index of the entity within its class
    * `descriptor_index` - Index of the entity class (descriptor) being modified
    */
    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    /* `on_insert` returning the delta unchecked with a flag that is set when a
    level overflowed, so an enclosing set or director can sum several deltas
    and check once. The default delegates to `on_insert`, which checks itself. */
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

    /* `on_retract` returning the delta unchecked with a flag that is set when a
    level overflowed, so an enclosing set or director can sum several deltas
    and check once. The default delegates to `on_retract`, which checks itself. */
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

    // Resets internal state for a new solving session.
    fn reset(&mut self);

    // Returns the constraint reference (package + name).
    fn constraint_ref(&self) -> &ConstraintRef;

    // Returns the constraint name.
    fn name(&self) -> &str {
        &self.constraint_ref().name
    }

    // Returns true if this is a hard constraint.
    fn is_hard(&self) -> bool {
        false
    }

    /* Returns detailed matches with entity justifications.

    The default implementation returns an empty vector.
    Constraints should override this to provide detailed match information
    including the entities involved in each constraint violation.

    This enables score explanation features without requiring all constraints
    to implement detailed tracking.
    */
    fn get_matches<'a>(&'a self, _solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>> {
        Vec::new()
    }

    /* Weighted counterparts of the scoring methods.

    `WeightedMode` score directors call these instead of the unweighted
    methods. Each receives this constraint's bound multiplier and the
    defaults scale the whole contribution by it, which is exact because
    integer scaling is linear: the scaled sum of match scores equals the sum of
    scaled match scores. Retained state therefore stays unweighted, and the
    unweighted methods never see a multiplier.
    */
    #[inline]
    fn evaluate_weighted(&self, solution: &S, multiplier: u32) -> Sc {
        scale_contribution(self.evaluate(solution), multiplier)
    }

    #[inline]
    fn initialize_weighted(&mut self, solution: &S, multiplier: u32) -> Sc {
        scale_contribution(self.initialize(solution), multiplier)
    }

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

    /* `on_insert_weighted` returning the contribution unchecked with an overflow flag;
    see `on_insert_overflowing`. */
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

    /* `on_retract_weighted` returning the contribution unchecked with an overflow flag;
    see `on_insert_overflowing`. */
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

// Result of evaluating a single constraint.
#[derive(Debug, Clone)]
pub struct ConstraintResult<'a, Sc> {
    // Constraint name.
    pub name: &'a str,
    // Score contribution from this constraint.
    pub score: Sc,
    // Number of matches for this constraint.
    pub match_count: usize,
    // Whether this is a hard constraint.
    pub is_hard: bool,
    // Bound runtime weight multiplier; `0` means the constraint is disabled.
    pub weight_multiplier: u32,
}

// Immutable public metadata for a scoring constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstraintMetadata<'a> {
    pub constraint_ref: &'a ConstraintRef,
    pub is_hard: bool,
}

impl<'a> ConstraintMetadata<'a> {
    pub fn new(constraint_ref: &'a ConstraintRef, is_hard: bool) -> Self {
        Self {
            constraint_ref,
            is_hard,
        }
    }

    pub fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    pub fn full_name(&self) -> String {
        self.constraint_ref.full_name()
    }
}

/* A set of constraints that can be evaluated together.

`ConstraintSet` is implemented for each `IncrementalConstraint` as a singleton
set and for tuples of nested `ConstraintSet` values. This keeps composition
fully monomorphized without virtual dispatch.
*/
pub trait ConstraintSet<S, Sc: Score>: Send + Sync {
    // Evaluates all constraints and returns the total score.
    fn evaluate_all(&self, solution: &S) -> Sc;

    // Returns the number of constraints in this set.
    fn constraint_count(&self) -> usize;

    // Returns deduplicated immutable metadata for constraints in this set.
    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        deduplicate_constraint_metadata(self.constraint_metadata_entries())
    }

    // Returns one metadata entry per constraint in authored order.
    #[doc(hidden)]
    fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>>;

    /* Evaluates each constraint individually and returns per-constraint results.

    Useful for score explanation and debugging.
    */
    fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>>;

    /* Evaluates each constraint with detailed match information.

    Returns per-constraint analysis including all matches with entity
    justifications. This enables full score explanation features.
    */
    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>>;

    /* Initializes all constraints by inserting all entities.

    Must be called before using incremental methods.
    Returns the total score from initialization.
    */
    fn initialize_all(&mut self, solution: &S) -> Sc;

    /* Called when an entity is inserted.

    Returns the total score delta from all constraints.

    # Arguments
    * `solution` - The planning solution
    * `entity_index` - Index of the entity within its class
    * `descriptor_index` - Index of the entity class (descriptor) being modified
    */
    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    /* Called when an entity is retracted.

    Returns the total score delta from all constraints.

    # Arguments
    * `solution` - The planning solution
    * `entity_index` - Index of the entity within its class
    * `descriptor_index` - Index of the entity class (descriptor) being modified
    */
    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    /* `on_insert_all` returning the delta unchecked with a flag that is set when a
    level overflowed, so an enclosing set or director can sum several deltas
    and check once. The default delegates to `on_insert_all`, which checks itself. */
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

    /* `on_retract_all` returning the delta unchecked with a flag that is set when a
    level overflowed, so an enclosing set or director can sum several deltas
    and check once. The default delegates to `on_retract_all`, which checks itself. */
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

    // Resets all constraints for a new solving session.
    fn reset_all(&mut self);

    /* Runtime constraint weights.

    A set exposes `constraint_count()` weight slots: one per independently
    scored named constraint, in structural order (tuple and chain children
    left to right, shared-terminal scorers in authored order).
    `constraint_weight_refs` pushes each slot's constraint reference; every
    `*_weighted` method receives one multiplier per slot in that order and
    scales each slot's contribution by its multiplier. Only `WeightedMode`
    score directors call these; `UnweightedMode` directors never name them,
    so the unweighted methods stay the whole scoring path of every other
    director.
    */
    fn constraint_weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>);

    fn evaluate_all_weighted(&self, solution: &S, multipliers: &[u32]) -> Sc;

    fn evaluate_each_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>>;

    fn evaluate_detailed_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>>;

    fn initialize_all_weighted(&mut self, solution: &S, multipliers: &[u32]) -> Sc;

    fn on_insert_all_weighted(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> Sc;

    fn on_retract_all_weighted(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> Sc;
    /* Weighted `on_insert_all_overflowing`. */
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

    /* Weighted `on_retract_all_overflowing`. */
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
