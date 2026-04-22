use solverforge::prelude::*;
use solverforge::{SolverEvent, SolverManager, SolverTerminalReason};

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
    solver_toml = "fixtures/scalar_runtime_publication_solver.toml"
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
        .filter(|task: &Task| task.resource_idx.is_none())
        .penalize(HardSoftScore::of(0, 1))
        .named("unassigned task"),)
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
fn scalar_only_solution_runs_construction_in_retained_runtime() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER.solve(seeded_plan()).expect("job should start");
    let mut completed_solution = None;

    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::BestSolution { .. } => {}
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(
                    metadata.terminal_reason,
                    Some(SolverTerminalReason::Completed)
                );
                completed_solution = Some(solution);
                break;
            }
            other => {
                eprintln!("event={other:?}");
            }
        }
    }

    let solution = completed_solution.expect("expected a completed solution");
    assert!(
        solution
            .tasks
            .iter()
            .all(|task| task.resource_idx.is_some()),
        "scalar construction should assign all tasks instead of taking the trivial path"
    );

    MANAGER.delete(job_id).expect("delete completed job");
}
