//! Zero-erasure incremental uni-constraint.
//!
//! All closure types are concrete generics - no Arc, no dyn, fully monomorphized.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};
use crate::api::constraint_set::IncrementalConstraint;

/// Zero-erasure incremental uni-constraint.
///
/// All closure types are concrete generics - no Arc, no dyn, fully monomorphized.
pub struct IncrementalUniConstraint<S, A, E, F, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor: E,
    filter: F,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(S, A, Sc)>,
}

impl<S, A, E, F, W, Sc> IncrementalUniConstraint<S, A, E, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    F: Fn(&A) -> bool + Send + Sync,
    W: Fn(&A) -> Sc + Send + Sync,
    Sc: Score,
{
    /// Creates a new zero-erasure incremental uni-constraint.
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor: E,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            extractor,
            filter,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn matches(&self, entity: &A) -> bool {
        (self.filter)(entity)
    }

    #[inline]
    fn compute_delta(&self, entity: &A) -> Sc {
        let base = (self.weight)(entity);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    #[inline]
    fn reverse_delta(&self, entity: &A) -> Sc {
        let base = (self.weight)(entity);
        match self.impact_type {
            ImpactType::Penalty => base,
            ImpactType::Reward => -base,
        }
    }
}

impl<S, A, E, F, W, Sc> IncrementalConstraint<S, Sc> for IncrementalUniConstraint<S, A, E, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Debug + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    F: Fn(&A) -> bool + Send + Sync,
    W: Fn(&A) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities = (self.extractor)(solution);
        let mut total = Sc::zero();
        for entity in entities {
            if self.matches(entity) {
                total = total + self.compute_delta(entity);
            }
        }
        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities = (self.extractor)(solution);
        entities.iter().filter(|e| self.matches(e)).count()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.evaluate(solution)
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }
        let entity = &entities[entity_index];
        if self.matches(entity) {
            self.compute_delta(entity)
        } else {
            Sc::zero()
        }
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }
        let entity = &entities[entity_index];
        if self.matches(entity) {
            self.reverse_delta(entity)
        } else {
            Sc::zero()
        }
    }

    fn reset(&mut self) {
        // Stateless
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        self.constraint_ref.clone()
    }

    fn get_matches(&self, solution: &S) -> Vec<DetailedConstraintMatch<Sc>> {
        let entities = (self.extractor)(solution);
        let cref = self.constraint_ref.clone();
        entities
            .iter()
            .filter(|e| self.matches(e))
            .map(|entity| {
                let entity_ref = EntityRef::new(entity);
                let justification = ConstraintJustification::new(vec![entity_ref]);
                DetailedConstraintMatch::new(
                    cref.clone(),
                    self.compute_delta(entity),
                    justification,
                )
            })
            .collect()
    }

    fn weight(&self) -> Sc {
        // For uni-constraints, we use a unit entity to compute the base weight.
        // This works for constant weights; for dynamic weights, returns zero.
        Sc::zero()
    }
}

impl<S, A, E, F, W, Sc: Score> std::fmt::Debug for IncrementalUniConstraint<S, A, E, F, W, Sc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementalUniConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
