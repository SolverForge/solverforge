//! Swap move implementation for dynamic solutions.

use std::fmt;
use std::sync::Arc;

use solverforge_scoring::ScoreDirector;
use solverforge_solver::heuristic::r#move::Move;

use crate::solution::DynamicSolution;

/// Swaps values of a planning variable between two entities of the same class.
#[derive(Clone)]
pub struct DynamicSwapMove {
    pub class_idx: usize,
    pub left_entity_idx: usize,
    pub right_entity_idx: usize,
    pub field_idx: usize,
    pub variable_name: Arc<str>,
    entity_indices: [usize; 2],
}

impl DynamicSwapMove {
    pub fn new(
        class_idx: usize,
        left_entity_idx: usize,
        right_entity_idx: usize,
        field_idx: usize,
        variable_name: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            class_idx,
            left_entity_idx,
            right_entity_idx,
            field_idx,
            variable_name: variable_name.into(),
            entity_indices: [left_entity_idx, right_entity_idx],
        }
    }
}

impl fmt::Debug for DynamicSwapMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SwapMove(class={}, left={}, right={}, {})",
            self.class_idx, self.left_entity_idx, self.right_entity_idx, self.variable_name
        )
    }
}

impl Move<DynamicSolution> for DynamicSwapMove {
    fn is_doable<D: ScoreDirector<DynamicSolution>>(&self, score_director: &D) -> bool {
        if self.left_entity_idx == self.right_entity_idx {
            return false;
        }
        let solution = score_director.working_solution();
        let left = solution.get_field(self.class_idx, self.left_entity_idx, self.field_idx);
        let right = solution.get_field(self.class_idx, self.right_entity_idx, self.field_idx);
        match (left, right) {
            (Some(l), Some(r)) => l != r,
            _ => false,
        }
    }

    fn do_move<D: ScoreDirector<DynamicSolution>>(&self, score_director: &mut D) {
        let left_value = score_director
            .working_solution()
            .get_field(self.class_idx, self.left_entity_idx, self.field_idx)
            .cloned();
        let right_value = score_director
            .working_solution()
            .get_field(self.class_idx, self.right_entity_idx, self.field_idx)
            .cloned();

        // Notify before changes
        score_director.before_variable_changed(
            self.class_idx,
            self.left_entity_idx,
            &self.variable_name,
        );
        score_director.before_variable_changed(
            self.class_idx,
            self.right_entity_idx,
            &self.variable_name,
        );

        // Swap: left gets right's value, right gets left's value
        if let Some(ref rv) = right_value {
            score_director.working_solution_mut().update_field(
                self.class_idx,
                self.left_entity_idx,
                self.field_idx,
                rv.clone(),
            );
        }
        if let Some(ref lv) = left_value {
            score_director.working_solution_mut().update_field(
                self.class_idx,
                self.right_entity_idx,
                self.field_idx,
                lv.clone(),
            );
        }

        // Notify after changes
        score_director.after_variable_changed(
            self.class_idx,
            self.left_entity_idx,
            &self.variable_name,
        );
        score_director.after_variable_changed(
            self.class_idx,
            self.right_entity_idx,
            &self.variable_name,
        );

        // Register undo — swap back
        if let (Some(lv), Some(rv)) = (left_value, right_value) {
            let class_idx = self.class_idx;
            let left_idx = self.left_entity_idx;
            let right_idx = self.right_entity_idx;
            let field_idx = self.field_idx;
            score_director.register_undo(Box::new(move |solution: &mut DynamicSolution| {
                solution.update_field(class_idx, left_idx, field_idx, lv);
                solution.update_field(class_idx, right_idx, field_idx, rv);
            }));
        }
    }

    fn descriptor_index(&self) -> usize {
        self.class_idx
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        &self.variable_name
    }
}
