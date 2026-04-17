use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

#[problem_fact]
struct Worker {
    #[planning_id]
    id: usize,
}

#[planning_entity]
struct Task {
    #[planning_id]
    id: usize,

    #[planning_variable(value_range = "workers", allows_unassigned = true)]
    worker: Option<usize>,
}

#[planning_solution(constraints = "constraints")]
struct Plan {
    #[problem_fact_collection]
    workers: Vec<Worker>,

    #[planning_entity_collection]
    tasks: Vec<Task>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    use PlanConstraintStreams;

    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .tasks()
            .penalize_soft()
            .named("penalize_tasks"),
    )
}

fn main() {
    let _ = Plan {
        workers: Vec::new(),
        tasks: Vec::new(),
        score: None,
    };
}
