//! Each scalar construction heuristic assigns in its documented entity
//! order, value order and forager. Tasks have difficulties 1, 3, 2; workers
//! have strengths 2, 0, 1 plus `load_weight` per task already on them and
//! costs 3, 2, 1; a task only goes to a free worker.

use solverforge::{SolverEvent, SolverManager};

#[path = "construction_heuristics/domain/mod.rs"]
mod domain;

use domain::Plan;

static MANAGER: SolverManager<Plan> = SolverManager::new();

fn construct(heuristic: &'static str, load_weight: i64) -> Vec<Option<usize>> {
    construct_plan(Plan::new(heuristic, load_weight))
}

fn construct_plan(plan: Plan) -> Vec<Option<usize>> {
    let heuristic = plan.heuristic;
    let (job_id, mut receiver) = MANAGER.solve(plan).expect("solve starts");
    let solution = loop {
        match receiver
            .blocking_recv()
            .expect("event stream should reach a terminal event")
        {
            SolverEvent::Completed { solution, .. } => break solution,
            SolverEvent::Failed { error, .. } => panic!("{heuristic} failed: {error}"),
            SolverEvent::Cancelled { .. } => panic!("{heuristic} was cancelled"),
            _ => {}
        }
    };
    MANAGER.delete(job_id).expect("the completed job deletes");
    solution.workers_by_task()
}

#[test]
fn first_fit_takes_tasks_in_order_and_the_first_free_worker() {
    assert_eq!(construct("first_fit", 0), [Some(0), Some(1), Some(2)]);
}

#[test]
fn first_fit_decreasing_takes_the_hardest_task_first() {
    // Tasks 1, 2, 0 in that order, each on the first free worker.
    assert_eq!(
        construct("first_fit_decreasing", 0),
        [Some(2), Some(0), Some(1)]
    );
}

#[test]
fn allocate_entity_from_queue_takes_the_easiest_task_first() {
    // Tasks 0, 2, 1.
    assert_eq!(
        construct("allocate_entity_from_queue", 0),
        [Some(0), Some(2), Some(1)]
    );
}

#[test]
fn allocate_to_value_from_queue_offers_the_weakest_worker_first() {
    // Workers in strength order 1, 2, 0.
    assert_eq!(
        construct("allocate_to_value_from_queue", 0),
        [Some(1), Some(2), Some(0)]
    );
}

#[test]
fn weakest_fit_picks_the_weakest_free_worker() {
    // Loaded workers get stronger, so the weakest is always a free one:
    // task 0 → 1, task 1 → 2, task 2 → 0.
    assert_eq!(construct("weakest_fit", 10), [Some(1), Some(2), Some(0)]);
}

#[test]
fn weakest_fit_decreasing_picks_the_weakest_worker_for_the_hardest_task() {
    // Task 1 → 1, task 2 → 2, task 0 → 0.
    assert_eq!(
        construct("weakest_fit_decreasing", 10),
        [Some(0), Some(1), Some(2)]
    );
}

#[test]
fn strongest_fit_picks_the_strongest_free_worker() {
    // Loaded workers get weaker: task 0 → 0, task 1 → 2, task 2 → 1.
    assert_eq!(construct("strongest_fit", -10), [Some(0), Some(2), Some(1)]);
}

#[test]
fn strongest_fit_decreasing_picks_the_strongest_worker_for_the_hardest_task() {
    // Task 1 → 0, task 2 → 2, task 0 → 1.
    assert_eq!(
        construct("strongest_fit_decreasing", -10),
        [Some(1), Some(0), Some(2)]
    );
}

#[test]
fn cheapest_insertion_picks_the_best_scoring_worker() {
    // The cheapest free worker: task 0 → 2, task 1 → 1, task 2 → 0.
    assert_eq!(
        construct("cheapest_insertion", 0),
        [Some(2), Some(1), Some(0)]
    );
}

#[test]
fn allocate_to_value_from_queue_reorders_the_workers_after_each_assignment() {
    // Worker 0's key falls by 10 per assigned task: after task 0 takes
    // worker 1, worker 0 comes before worker 2 for task 1.
    let plan = Plan {
        drift: -10,
        ..Plan::new("allocate_to_value_from_queue", 0)
    };
    assert_eq!(construct_plan(plan), [Some(1), Some(0), Some(2)]);
}

#[test]
fn first_fit_decreasing_reorders_the_tasks_after_each_assignment() {
    // Task 0's difficulty rises by 10 per assigned task: after task 1 takes
    // worker 0, task 0 comes before task 2.
    let plan = Plan {
        difficulty_drift: 10,
        ..Plan::new("first_fit_decreasing", 0)
    };
    assert_eq!(construct_plan(plan), [Some(1), Some(0), Some(2)]);
}
