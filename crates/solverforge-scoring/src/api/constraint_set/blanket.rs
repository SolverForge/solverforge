/* Every `IncrementalConstraint` is a one-constraint `ConstraintSet`. */

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::super::analysis::ConstraintAnalysis;
use super::incremental::{
    ConstraintMetadata, ConstraintResult, ConstraintSet, IncrementalConstraint,
};

impl<S, Sc, C> ConstraintSet<S, Sc> for C
where
    S: Send + Sync,
    Sc: Score,
    C: IncrementalConstraint<S, Sc>,
{
    #[inline]
    fn evaluate_all(&self, solution: &S) -> Sc {
        self.evaluate(solution)
    }

    #[inline]
    fn constraint_count(&self) -> usize {
        1
    }

    fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>> {
        vec![ConstraintMetadata::new(
            self.constraint_ref(),
            self.is_hard(),
        )]
    }

    fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        vec![ConstraintResult {
            name: self.name(),
            score: self.evaluate(solution),
            match_count: self.match_count(solution),
            is_hard: self.is_hard(),
            weight_multiplier: 1,
        }]
    }

    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
        vec![ConstraintAnalysis::new(
            self.constraint_ref(),
            1,
            self.evaluate(solution),
            self.get_matches(solution),
            self.is_hard(),
        )]
    }

    #[inline]
    fn initialize_all(&mut self, solution: &S) -> Sc {
        self.initialize(solution)
    }

    #[inline]
    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.on_insert(solution, entity_index, descriptor_index)
    }

    #[inline]
    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.on_retract(solution, entity_index, descriptor_index)
    }

    #[inline]
    fn on_insert_all_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
    ) -> (Sc, bool) {
        self.on_insert_overflowing(solution, entity_index, descriptor_index)
    }

    #[inline]
    fn on_retract_all_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
    ) -> (Sc, bool) {
        self.on_retract_overflowing(solution, entity_index, descriptor_index)
    }

    #[inline]
    fn reset_all(&mut self) {
        self.reset();
    }

    fn constraint_weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>) {
        refs.push(self.constraint_ref());
    }

    #[inline]
    fn evaluate_all_weighted(&self, solution: &S, multipliers: &[u32]) -> Sc {
        self.evaluate_weighted(solution, multipliers[0])
    }

    fn evaluate_each_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>> {
        vec![ConstraintResult {
            name: self.name(),
            score: self.evaluate_weighted(solution, multipliers[0]),
            match_count: self.match_count(solution),
            is_hard: self.is_hard(),
            weight_multiplier: multipliers[0],
        }]
    }

    fn evaluate_detailed_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        vec![ConstraintAnalysis::new(
            self.constraint_ref(),
            multipliers[0],
            self.evaluate_weighted(solution, multipliers[0]),
            self.get_matches_weighted(solution, multipliers[0]),
            self.is_hard(),
        )]
    }

    #[inline]
    fn initialize_all_weighted(&mut self, solution: &S, multipliers: &[u32]) -> Sc {
        self.initialize_weighted(solution, multipliers[0])
    }

    #[inline]
    fn on_insert_all_weighted(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> Sc {
        self.on_insert_weighted(solution, entity_index, descriptor_index, multipliers[0])
    }

    #[inline]
    fn on_retract_all_weighted(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> Sc {
        self.on_retract_weighted(solution, entity_index, descriptor_index, multipliers[0])
    }

    #[inline]
    fn on_insert_all_weighted_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> (Sc, bool) {
        self.on_insert_weighted_overflowing(
            solution,
            entity_index,
            descriptor_index,
            multipliers[0],
        )
    }

    #[inline]
    fn on_retract_all_weighted_overflowing(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        multipliers: &[u32],
    ) -> (Sc, bool) {
        self.on_retract_weighted_overflowing(
            solution,
            entity_index,
            descriptor_index,
            multipliers[0],
        )
    }
}
