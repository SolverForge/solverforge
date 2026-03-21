use crate::domain::Plan;
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: Prefer assignments whose affinity group matches the task preference.
pub struct AffinityMatchConstraint {
    penalties: Vec<i64>,
}

impl AffinityMatchConstraint {
    pub fn new() -> Self {
        Self {
            penalties: Vec::new(),
        }
    }

    fn ensure_shape(&mut self, solution: &Plan) {
        if self.penalties.len() != solution.tasks.len() {
            self.penalties = vec![0; solution.tasks.len()];
        }
    }

    fn penalty(solution: &Plan, entity_index: usize) -> i64 {
        let Some(task) = solution.tasks.get(entity_index) else {
            return 0;
        };
        let Some(resource_idx) = task.resource_idx else {
            return 0;
        };
        let Some(resource) = solution.resources.get(resource_idx) else {
            return 0;
        };

        if task.preferred_group == resource.affinity_group {
            0
        } else {
            task.demand
        }
    }

    fn total_penalty(solution: &Plan) -> i64 {
        solution
            .tasks
            .iter()
            .enumerate()
            .map(|(entity_index, _)| Self::penalty(solution, entity_index))
            .sum()
    }

    fn update_penalty(&mut self, solution: &Plan, entity_index: usize) -> HardSoftScore {
        self.ensure_shape(solution);
        if entity_index >= self.penalties.len() {
            return HardSoftScore::ZERO;
        }

        let before = self.penalties[entity_index];
        let after = Self::penalty(solution, entity_index);
        self.penalties[entity_index] = after;

        HardSoftScore::of(0, -(after - before))
    }
}

impl IncrementalConstraint<Plan, HardSoftScore> for AffinityMatchConstraint {
    fn evaluate(&self, solution: &Plan) -> HardSoftScore {
        HardSoftScore::of(0, -Self::total_penalty(solution))
    }

    fn match_count(&self, solution: &Plan) -> usize {
        solution
            .tasks
            .iter()
            .enumerate()
            .filter(|(entity_index, _)| Self::penalty(solution, *entity_index) > 0)
            .count()
    }

    fn initialize(&mut self, solution: &Plan) -> HardSoftScore {
        self.ensure_shape(solution);
        for (entity_index, penalty) in self.penalties.iter_mut().enumerate() {
            *penalty = Self::penalty(solution, entity_index);
        }
        HardSoftScore::of(0, -self.penalties.iter().sum::<i64>())
    }

    fn on_insert(
        &mut self,
        solution: &Plan,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        if descriptor_index != 0 {
            return HardSoftScore::ZERO;
        }
        self.update_penalty(solution, entity_index)
    }

    fn on_retract(
        &mut self,
        solution: &Plan,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        if descriptor_index != 0 {
            return HardSoftScore::ZERO;
        }
        self.update_penalty(solution, entity_index)
    }

    fn reset(&mut self) {
        self.penalties.clear();
    }

    fn name(&self) -> &str {
        "Affinity match"
    }

    fn is_hard(&self) -> bool {
        false
    }

    fn weight(&self) -> HardSoftScore {
        HardSoftScore::ONE_SOFT
    }
}

pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    AffinityMatchConstraint::new()
}
