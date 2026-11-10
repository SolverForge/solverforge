//! A one-variable scalar model the solver's integration tests compile:
//! tasks assigned a worker from `0..4`. Each test binary uses part of it.

use std::any::TypeId;

use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
    VariableDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_solver::{
    CrossEntityDistanceMeter, RuntimeModel, ScalarVariableSlot, SearchContext, ValueSource,
    VariableSlot,
};

#[derive(Clone, Debug)]
pub(crate) struct Task {
    pub(crate) worker: Option<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct Plan {
    pub(crate) tasks: Vec<Task>,
    pub(crate) score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct NoMeter;

impl CrossEntityDistanceMeter<Plan> for NoMeter {
    fn distance(&self, _: &Plan, _: usize, _: usize, _: usize, _: usize) -> f64 {
        0.0
    }
}

fn tasks(plan: &Plan) -> &Vec<Task> {
    &plan.tasks
}

fn tasks_mut(plan: &mut Plan) -> &mut Vec<Task> {
    &mut plan.tasks
}

fn task_count(plan: &Plan) -> usize {
    plan.tasks.len()
}

fn worker_get(plan: &Plan, entity: usize, _variable_index: usize) -> Option<usize> {
    plan.tasks[entity].worker
}

fn worker_set(plan: &mut Plan, entity: usize, _variable_index: usize, value: Option<usize>) {
    plan.tasks[entity].worker = value;
}

fn descriptor() -> SolutionDescriptor {
    let entity = EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
        .with_extractor(Box::new(EntityCollectionExtractor::new(
            "Task", "tasks", tasks, tasks_mut,
        )))
        .with_variable(VariableDescriptor::genuine("worker"));
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(entity)
}

/// The search context binding the model's one scalar slot.
pub(crate) fn context() -> SearchContext<Plan, usize, NoMeter, NoMeter> {
    let slot = VariableSlot::Scalar(ScalarVariableSlot::new(
        0,
        0,
        "Task",
        task_count,
        "worker",
        worker_get,
        worker_set,
        ValueSource::CountableRange { from: 0, to: 4 },
        false,
    ));
    SearchContext::new(descriptor(), RuntimeModel::new(vec![slot]), Some(7))
        .expect("the model binds to its descriptor")
}

/// `count` tasks, none assigned.
pub(crate) fn unassigned(count: usize) -> Plan {
    Plan {
        tasks: (0..count).map(|_| Task { worker: None }).collect(),
        score: None,
    }
}

/// A director over `plan` scoring the task count.
pub(crate) fn director(plan: Plan) -> solverforge_scoring::ScoreDirector<Plan, ()> {
    solverforge_scoring::ScoreDirector::simple(plan, descriptor(), |plan, _| plan.tasks.len())
}
