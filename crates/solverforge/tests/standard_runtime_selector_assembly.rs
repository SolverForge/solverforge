use solverforge::prelude::*;
use solverforge::{SolverEvent, SolverLifecycleState, SolverManager};

#[problem_fact]
struct Resource {
    #[planning_id]
    id: String,
}

#[planning_entity]
struct Task {
    #[planning_id]
    id: String,

    #[planning_variable(value_range = "resources", allows_unassigned = true)]
    resource_idx: Option<usize>,
}

#[planning_solution(
    constraints = "define_constraints",
    solver_toml = "fixtures/standard_runtime_selector_assembly_solver.toml"
)]
struct Plan {
    #[problem_fact_collection]
    resources: Vec<Resource>,

    #[planning_entity_collection]
    tasks: Vec<Task>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn define_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .tasks()
        .penalize_with(|_| HardSoftScore::of(0, 0))
        .named("noop"),)
}

fn seeded_plan() -> Plan {
    let resources = (0..4)
        .map(|idx| Resource {
            id: format!("resource-{idx}"),
        })
        .collect();
    let tasks = (0..12)
        .map(|idx| Task {
            id: format!("task-{idx}"),
            resource_idx: None,
        })
        .collect();

    Plan {
        resources,
        tasks,
        score: None,
    }
}

#[test]
fn standard_only_solution_builds_default_selector_without_failure() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER.solve(seeded_plan()).expect("job should start");
    let mut completed = false;

    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::BestSolution { .. } => {}
            SolverEvent::Completed { metadata, .. } => {
                assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
                completed = true;
                break;
            }
            SolverEvent::Failed { error, .. } => {
                panic!("standard runtime selector assembly failed: {error}");
            }
            _ => {}
        }
    }

    assert!(completed, "expected selector assembly solve to complete");
    MANAGER.delete(job_id).expect("delete completed job");
}
