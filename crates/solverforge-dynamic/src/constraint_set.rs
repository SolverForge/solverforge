//! Constraint set implementation for dynamic constraints.

use solverforge_core::score::HardSoftScore;
use solverforge_scoring::api::analysis::ConstraintAnalysis;
use solverforge_scoring::api::constraint_set::{
    ConstraintResult, ConstraintSet, IncrementalConstraint,
};

use crate::solution::DynamicSolution;

/// A set of dynamic constraints.
///
/// This wraps monomorphized `IncrementalConstraint` implementations from `solverforge-scoring`
/// and delegates all scoring operations to them.
pub struct DynamicConstraintSet {
    constraints: Vec<Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync>>,
}

impl Default for DynamicConstraintSet {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicConstraintSet {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    pub fn from_vec(
        constraints: Vec<
            Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync>,
        >,
    ) -> Self {
        Self { constraints }
    }

    pub fn add(
        &mut self,
        constraint: Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync>,
    ) {
        self.constraints.push(constraint);
    }

    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }
}

impl ConstraintSet<DynamicSolution, HardSoftScore> for DynamicConstraintSet {
    fn evaluate_all(&self, solution: &DynamicSolution) -> HardSoftScore {
        // Delegate to each boxed constraint's evaluate method
        let mut total = HardSoftScore::ZERO;
        for constraint in &self.constraints {
            total = total + constraint.evaluate(solution);
        }
        total
    }

    fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    fn evaluate_each(&self, solution: &DynamicSolution) -> Vec<ConstraintResult<HardSoftScore>> {
        // Delegate to each boxed constraint
        self.constraints
            .iter()
            .map(|c| ConstraintResult {
                name: c.name().to_string(),
                score: c.evaluate(solution),
                match_count: c.match_count(solution),
                is_hard: c.is_hard(),
            })
            .collect()
    }

    fn evaluate_detailed(
        &self,
        solution: &DynamicSolution,
    ) -> Vec<ConstraintAnalysis<HardSoftScore>> {
        // Delegate to each boxed constraint
        self.constraints
            .iter()
            .map(|c| {
                ConstraintAnalysis::new(
                    c.constraint_ref(),
                    c.weight(),
                    c.evaluate(solution),
                    c.get_matches(solution),
                    c.is_hard(),
                )
            })
            .collect()
    }

    fn initialize_all(&mut self, solution: &DynamicSolution) -> HardSoftScore {
        let mut total = HardSoftScore::ZERO;
        for constraint in &mut self.constraints {
            total = total + constraint.initialize(solution);
        }
        total
    }

    fn on_insert_all(
        &mut self,
        solution: &DynamicSolution,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        let mut total = HardSoftScore::ZERO;
        for constraint in &mut self.constraints {
            total = total + constraint.on_insert(solution, entity_index, descriptor_index);
        }
        total
    }

    fn on_retract_all(
        &mut self,
        solution: &DynamicSolution,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        let mut total = HardSoftScore::ZERO;
        for constraint in &mut self.constraints {
            total = total + constraint.on_retract(solution, entity_index, descriptor_index);
        }
        total
    }

    fn reset_all(&mut self) {
        for constraint in &mut self.constraints {
            constraint.reset();
        }
    }
}
