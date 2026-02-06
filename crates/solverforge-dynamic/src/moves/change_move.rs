//! Change move implementation for dynamic solutions.

use std::fmt;
use std::sync::Arc;

use solverforge_scoring::ScoreDirector;
use solverforge_solver::heuristic::r#move::Move;

use crate::solution::{DynamicSolution, DynamicValue};

#[derive(Clone)]
pub struct DynamicChangeMove {
    pub class_idx: usize,
    pub entity_idx: usize,
    pub field_idx: usize,
    pub variable_name: Arc<str>,
    pub new_value: DynamicValue,
    entity_indices: [usize; 1],
}

impl DynamicChangeMove {
    pub fn new(
        class_idx: usize,
        entity_idx: usize,
        field_idx: usize,
        variable_name: impl Into<Arc<str>>,
        new_value: DynamicValue,
    ) -> Self {
        Self {
            class_idx,
            entity_idx,
            field_idx,
            variable_name: variable_name.into(),
            new_value,
            entity_indices: [entity_idx],
        }
    }
}

impl fmt::Debug for DynamicChangeMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ChangeMove(class={}, entity={}, {}={:?})",
            self.class_idx, self.entity_idx, self.variable_name, self.new_value
        )
    }
}

impl Move<DynamicSolution> for DynamicChangeMove {
    fn is_doable<D: ScoreDirector<DynamicSolution>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        if let Some(current) = solution.get_field(self.class_idx, self.entity_idx, self.field_idx) {
            return current != &self.new_value;
        }
        false
    }

    fn do_move<D: ScoreDirector<DynamicSolution>>(&self, score_director: &mut D) {
        let old_value = score_director
            .working_solution()
            .get_field(self.class_idx, self.entity_idx, self.field_idx)
            .cloned();

        score_director.before_variable_changed(
            self.class_idx,
            self.entity_idx,
            &self.variable_name,
        );

        score_director.working_solution_mut().update_field(
            self.class_idx,
            self.entity_idx,
            self.field_idx,
            self.new_value.clone(),
        );

        score_director.after_variable_changed(self.class_idx, self.entity_idx, &self.variable_name);

        if let Some(old_value) = old_value {
            let class_idx = self.class_idx;
            let entity_idx = self.entity_idx;
            let field_idx = self.field_idx;
            score_director.register_undo(Box::new(move |solution: &mut DynamicSolution| {
                solution.update_field(class_idx, entity_idx, field_idx, old_value);
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
