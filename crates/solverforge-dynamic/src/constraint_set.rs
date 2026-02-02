//! Constraint set implementation for dynamic constraints.

use solverforge_core::score::HardSoftScore;
use solverforge_core::ConstraintRef;
use solverforge_scoring::api::analysis::ConstraintAnalysis;
use solverforge_scoring::api::constraint_set::{
    ConstraintResult, ConstraintSet, IncrementalConstraint,
};

use crate::constraint::DynamicConstraint;
use crate::solution::DynamicSolution;

/// A set of dynamic constraints.
#[derive(Debug, Clone, Default)]
pub struct DynamicConstraintSet {
    constraints: Vec<DynamicConstraint>,
}

impl DynamicConstraintSet {
    /// Creates a new empty constraint set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a constraint set from a vector of constraints.
    pub fn from_vec(constraints: Vec<DynamicConstraint>) -> Self {
        Self { constraints }
    }

    /// Adds a constraint to the set.
    pub fn add(&mut self, constraint: DynamicConstraint) {
        self.constraints.push(constraint);
    }

    /// Returns an iterator over the constraints.
    pub fn iter(&self) -> impl Iterator<Item = &DynamicConstraint> {
        self.constraints.iter()
    }
}

impl ConstraintSet<DynamicSolution, HardSoftScore> for DynamicConstraintSet {
    fn evaluate_all(&self, _solution: &DynamicSolution) -> HardSoftScore {
        // Use cached scores from incremental state - O(c) where c = constraint count
        let mut total = HardSoftScore::ZERO;
        for constraint in &self.constraints {
            total = total + constraint.cached_score();
        }
        total
    }

    fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    fn evaluate_each(&self, _solution: &DynamicSolution) -> Vec<ConstraintResult<HardSoftScore>> {
        // Use cached scores from incremental state
        self.constraints
            .iter()
            .map(|c| ConstraintResult {
                name: c.name.to_string(),
                score: c.cached_score(),
                match_count: c.match_count(),
                is_hard: c.is_hard,
            })
            .collect()
    }

    fn evaluate_detailed(
        &self,
        _solution: &DynamicSolution,
    ) -> Vec<ConstraintAnalysis<HardSoftScore>> {
        // Use cached scores from incremental state
        self.constraints
            .iter()
            .map(|c| {
                ConstraintAnalysis::new(
                    ConstraintRef::new("", &*c.name),
                    c.weight,
                    c.cached_score(),
                    Vec::new(), // No detailed matches for now
                    c.is_hard,
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
