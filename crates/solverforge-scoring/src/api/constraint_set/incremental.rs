// Typed constraint set for zero-erasure incremental scoring.
//
// This module provides the `ConstraintSet` trait which enables fully
// monomorphized constraint evaluation without virtual dispatch.

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::super::analysis::{ConstraintAnalysis, DetailedConstraintMatch};

// A single constraint with incremental scoring capability.
//
// Unlike the trait-object `Constraint` trait, `IncrementalConstraint` is
// designed for monomorphized code paths where the concrete type is known.
//
// # Incremental Protocol
//
// The incremental methods allow delta-based score updates:
//
// 1. Call `initialize` once to populate internal state
// 2. Before changing an entity's variable: call `on_retract` with old state
// 3. After changing the variable: call `on_insert` with new state
// 4. Score delta = insert_delta - retract_delta
//
// This avoids full re-evaluation on every move.
pub trait IncrementalConstraint<S, Sc: Score>: Send + Sync {
    // Full evaluation of this constraint.
    //
    // Iterates all entities and computes the total score impact.
    // Use this for initial scoring; use `on_insert`/`on_retract` for deltas.
    fn evaluate(&self, solution: &S) -> Sc;

    // Returns the number of matches for this constraint.
    fn match_count(&self, solution: &S) -> usize;

    // Initializes internal state by inserting all entities.
    //
    // Must be called before using incremental methods (`on_insert`/`on_retract`).
    // Returns the total score from initialization.
    fn initialize(&mut self, solution: &S) -> Sc;

    // Called when an entity is inserted or its variable changes.
    //
    // Returns the score delta from this insertion.
    //
    // # Arguments
    // * `solution` - The planning solution
    // * `entity_index` - Index of the entity within its class
    // * `descriptor_index` - Index of the entity class (descriptor) being modified
    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    // Called when an entity is retracted or before its variable changes.
    //
    // Returns the score delta (negative) from this retraction.
    //
    // # Arguments
    // * `solution` - The planning solution
    // * `entity_index` - Index of the entity within its class
    // * `descriptor_index` - Index of the entity class (descriptor) being modified
    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    // Resets internal state for a new solving session.
    fn reset(&mut self);

    // Returns the constraint name.
    fn name(&self) -> &str;

    // Returns true if this is a hard constraint.
    fn is_hard(&self) -> bool {
        false
    }

    // Returns the constraint reference (package + name).
    //
    // Default implementation constructs from `name()`.
    fn constraint_ref(&self) -> ConstraintRef {
        ConstraintRef::new("", self.name())
    }

    // Returns detailed matches with entity justifications.
    //
    // The default implementation returns an empty vector.
    // Constraints should override this to provide detailed match information
    // including the entities involved in each constraint violation.
    //
    // This enables score explanation features without requiring all constraints
    // to implement detailed tracking.
    fn get_matches(&self, _solution: &S) -> Vec<DetailedConstraintMatch<Sc>> {
        Vec::new()
    }

    // Returns the constraint weight (score per match).
    //
    // Used for score explanation. Default returns zero.
    fn weight(&self) -> Sc {
        Sc::zero()
    }
}

// Result of evaluating a single constraint.
#[derive(Debug, Clone)]
pub struct ConstraintResult<Sc> {
    // Constraint name.
    pub name: String,
    // Score contribution from this constraint.
    pub score: Sc,
    // Number of matches for this constraint.
    pub match_count: usize,
    // Whether this is a hard constraint.
    pub is_hard: bool,
}

// A set of constraints that can be evaluated together.
//
// `ConstraintSet` is implemented for tuples of `IncrementalConstraint`,
// enabling fully typed constraint evaluation without virtual dispatch.
pub trait ConstraintSet<S, Sc: Score>: Send + Sync {
    // Evaluates all constraints and returns the total score.
    fn evaluate_all(&self, solution: &S) -> Sc;

    // Returns the number of constraints in this set.
    fn constraint_count(&self) -> usize;

    // Evaluates each constraint individually and returns per-constraint results.
    //
    // Useful for score explanation and debugging.
    fn evaluate_each(&self, solution: &S) -> Vec<ConstraintResult<Sc>>;

    // Evaluates each constraint with detailed match information.
    //
    // Returns per-constraint analysis including all matches with entity
    // justifications. This enables full score explanation features.
    fn evaluate_detailed(&self, solution: &S) -> Vec<ConstraintAnalysis<Sc>>;

    // Initializes all constraints by inserting all entities.
    //
    // Must be called before using incremental methods.
    // Returns the total score from initialization.
    fn initialize_all(&mut self, solution: &S) -> Sc;

    // Called when an entity is inserted.
    //
    // Returns the total score delta from all constraints.
    //
    // # Arguments
    // * `solution` - The planning solution
    // * `entity_index` - Index of the entity within its class
    // * `descriptor_index` - Index of the entity class (descriptor) being modified
    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    // Called when an entity is retracted.
    //
    // Returns the total score delta from all constraints.
    //
    // # Arguments
    // * `solution` - The planning solution
    // * `entity_index` - Index of the entity within its class
    // * `descriptor_index` - Index of the entity class (descriptor) being modified
    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;

    // Resets all constraints for a new solving session.
    fn reset_all(&mut self);
}

// ============================================================================
// Tuple implementations
// ============================================================================

// Implement `ConstraintSet` for an empty tuple (no constraints).
impl<S: Send + Sync, Sc: Score> ConstraintSet<S, Sc> for () {
    #[inline]
    fn evaluate_all(&self, _solution: &S) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn constraint_count(&self) -> usize {
        0
    }

    #[inline]
    fn evaluate_each(&self, _solution: &S) -> Vec<ConstraintResult<Sc>> {
        Vec::new()
    }

    #[inline]
    fn evaluate_detailed(&self, _solution: &S) -> Vec<ConstraintAnalysis<Sc>> {
        Vec::new()
    }

    #[inline]
    fn initialize_all(&mut self, _solution: &S) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn on_insert_all(
        &mut self,
        _solution: &S,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn on_retract_all(
        &mut self,
        _solution: &S,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn reset_all(&mut self) {}
}

// Macro to implement `ConstraintSet` for tuples of various sizes.
macro_rules! impl_constraint_set_for_tuple {
    ($($idx:tt: $T:ident),+) => {
        impl<S, Sc, $($T),+> ConstraintSet<S, Sc> for ($($T,)+)
        where
            S: Send + Sync,
            Sc: Score,
            $($T: IncrementalConstraint<S, Sc>,)+
        {
            #[inline]
            fn evaluate_all(&self, solution: &S) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.evaluate(solution);)+
                total
            }

            #[inline]
            fn constraint_count(&self) -> usize {
                let mut count = 0;
                $(let _ = &self.$idx; count += 1;)+
                count
            }

            fn evaluate_each(&self, solution: &S) -> Vec<ConstraintResult<Sc>> {
                vec![$(ConstraintResult {
                    name: self.$idx.name().to_string(),
                    score: self.$idx.evaluate(solution),
                    match_count: self.$idx.match_count(solution),
                    is_hard: self.$idx.is_hard(),
                }),+]
            }

            fn evaluate_detailed(&self, solution: &S) -> Vec<ConstraintAnalysis<Sc>> {
                vec![$(ConstraintAnalysis::new(
                    self.$idx.constraint_ref(),
                    self.$idx.weight(),
                    self.$idx.evaluate(solution),
                    self.$idx.get_matches(solution),
                    self.$idx.is_hard(),
                )),+]
            }

            #[inline]
            fn initialize_all(&mut self, solution: &S) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.initialize(solution);)+
                total
            }

            #[inline]
            fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.on_insert(solution, entity_index, descriptor_index);)+
                total
            }

            #[inline]
            fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.on_retract(solution, entity_index, descriptor_index);)+
                total
            }

            #[inline]
            fn reset_all(&mut self) {
                $(self.$idx.reset();)+
            }
        }
    };
}

// Implement for tuples of size 1 through 16
impl_constraint_set_for_tuple!(0: C0);
impl_constraint_set_for_tuple!(0: C0, 1: C1);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15);
