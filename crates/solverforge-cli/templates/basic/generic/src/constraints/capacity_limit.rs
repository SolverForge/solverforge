use crate::domain::Plan;
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: Assigned demand must not exceed resource capacity.
///
/// This is implemented directly as an incremental constraint so the default
/// scaffold can express exact per-resource capacity limits without requiring
/// domain-specific shadow variables.
pub struct CapacityLimitConstraint {
    loads: Vec<i64>,
}

impl CapacityLimitConstraint {
    pub fn new() -> Self {
        Self { loads: Vec::new() }
    }

    fn ensure_shape(&mut self, solution: &Plan) {
        if self.loads.len() != solution.resources.len() {
            self.loads = vec![0; solution.resources.len()];
        }
    }

    fn overload(load: i64, capacity: i64) -> i64 {
        (load - capacity).max(0)
    }

    fn total_penalty(solution: &Plan, loads: &[i64]) -> i64 {
        solution
            .resources
            .iter()
            .zip(loads.iter().copied())
            .map(|(resource, load)| Self::overload(load, resource.capacity))
            .sum()
    }

    fn update_load(
        &mut self,
        solution: &Plan,
        entity_index: usize,
        delta: i64,
    ) -> HardSoftScore {
        self.ensure_shape(solution);
        let Some(task) = solution.tasks.get(entity_index) else {
            return HardSoftScore::ZERO;
        };
        let Some(resource_idx) = task.resource_idx else {
            return HardSoftScore::ZERO;
        };
        let Some(resource) = solution.resources.get(resource_idx) else {
            return HardSoftScore::ZERO;
        };

        let before = Self::overload(self.loads[resource_idx], resource.capacity);
        self.loads[resource_idx] += delta;
        let after = Self::overload(self.loads[resource_idx], resource.capacity);

        HardSoftScore::of(-(after - before), 0)
    }
}

impl IncrementalConstraint<Plan, HardSoftScore> for CapacityLimitConstraint {
    fn evaluate(&self, solution: &Plan) -> HardSoftScore {
        let mut loads = vec![0; solution.resources.len()];
        for task in &solution.tasks {
            if let Some(resource_idx) = task.resource_idx {
                if resource_idx < loads.len() {
                    loads[resource_idx] += task.demand;
                }
            }
        }
        HardSoftScore::of(-Self::total_penalty(solution, &loads), 0)
    }

    fn match_count(&self, solution: &Plan) -> usize {
        let mut loads = vec![0; solution.resources.len()];
        for task in &solution.tasks {
            if let Some(resource_idx) = task.resource_idx {
                if resource_idx < loads.len() {
                    loads[resource_idx] += task.demand;
                }
            }
        }
        solution
            .resources
            .iter()
            .zip(loads.iter().copied())
            .filter(|(resource, load)| *load > resource.capacity)
            .count()
    }

    fn initialize(&mut self, solution: &Plan) -> HardSoftScore {
        self.ensure_shape(solution);
        for load in &mut self.loads {
            *load = 0;
        }
        for task in &solution.tasks {
            if let Some(resource_idx) = task.resource_idx {
                if resource_idx < self.loads.len() {
                    self.loads[resource_idx] += task.demand;
                }
            }
        }
        HardSoftScore::of(-Self::total_penalty(solution, &self.loads), 0)
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
        self.update_load(solution, entity_index, solution.tasks[entity_index].demand)
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
        self.update_load(solution, entity_index, -solution.tasks[entity_index].demand)
    }

    fn reset(&mut self) {
        self.loads.clear();
    }

    fn name(&self) -> &str {
        "Capacity limit"
    }

    fn is_hard(&self) -> bool {
        true
    }

    fn weight(&self) -> HardSoftScore {
        HardSoftScore::ONE_HARD
    }
}

pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    CapacityLimitConstraint::new()
}
