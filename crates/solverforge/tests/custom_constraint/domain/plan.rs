use solverforge::prelude::*;

use super::{Resource, Task};

#[planning_solution]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

/// Penalizes each resource by the square of its assigned task size, so one
/// task's contribution depends on every other task on the same resource.
///
/// It retains the per-resource load and the resource each task is counted
/// on, so `on_retract` removes exactly what `on_insert` added.
pub struct ResourceLoad {
    constraint_ref: ConstraintRef,
    loads: Vec<i64>,
    placed: Vec<Option<usize>>,
}

impl ResourceLoad {
    pub fn new() -> Self {
        Self {
            constraint_ref: ConstraintRef::new("", "resource load"),
            loads: Vec::new(),
            placed: Vec::new(),
        }
    }

    fn penalty(load: i64) -> HardSoftScore {
        HardSoftScore::of_soft(-load * load)
    }

    // The score change of moving one resource's load by `by`.
    fn shift(&mut self, resource: usize, by: i64) -> HardSoftScore {
        let before = Self::penalty(self.loads[resource]);
        self.loads[resource] += by;
        Self::penalty(self.loads[resource]) - before
    }
}

impl SingleConstraint for ResourceLoad {}

impl IncrementalConstraint<Plan, HardSoftScore> for ResourceLoad {
    fn evaluate(&self, solution: &Plan) -> HardSoftScore {
        let mut loads = vec![0; solution.resources.len()];
        for task in &solution.tasks {
            if let Some(resource) = task.resource_idx {
                loads[resource] += task.size;
            }
        }
        loads
            .into_iter()
            .fold(HardSoftScore::ZERO, |sum, load| sum + Self::penalty(load))
    }

    fn match_count(&self, solution: &Plan) -> usize {
        solution.resources.len()
    }

    fn initialize(&mut self, solution: &Plan) -> HardSoftScore {
        self.loads = vec![0; solution.resources.len()];
        self.placed = vec![None; solution.tasks.len()];
        let mut score = HardSoftScore::ZERO;
        for index in 0..solution.tasks.len() {
            score = score + self.on_insert(solution, index, 0);
        }
        score
    }

    fn on_insert(&mut self, solution: &Plan, entity: usize, descriptor: usize) -> HardSoftScore {
        if descriptor != 0 || self.placed[entity].is_some() {
            return HardSoftScore::ZERO;
        }
        let task = &solution.tasks[entity];
        match task.resource_idx {
            Some(resource) => {
                self.placed[entity] = Some(resource);
                self.shift(resource, task.size)
            }
            None => HardSoftScore::ZERO,
        }
    }

    fn on_retract(&mut self, solution: &Plan, entity: usize, descriptor: usize) -> HardSoftScore {
        if descriptor != 0 {
            return HardSoftScore::ZERO;
        }
        match self.placed[entity].take() {
            Some(resource) => self.shift(resource, -solution.tasks[entity].size),
            None => HardSoftScore::ZERO,
        }
    }

    fn reset(&mut self) {
        self.loads.clear();
        self.placed.clear();
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }
}
