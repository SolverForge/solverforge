use super::nurses::{Schedule as MySchedule, Shift};

pub fn manage(problem: MySchedule, new_shift: Shift) {
// readme:begin
use solverforge::{SolverLifecycleState, SolverManager, SolverStatus};

static MANAGER: SolverManager<MySchedule> = SolverManager::new();

let (job_id, mut events) = MANAGER.solve(problem).unwrap();

let status: SolverStatus<_> = MANAGER.get_status(job_id).unwrap();
assert_eq!(status.lifecycle_state, SolverLifecycleState::Solving);

MANAGER.pause(job_id).unwrap();
MANAGER.resume(job_id).unwrap();
MANAGER.cancel(job_id).unwrap();

let snapshot = MANAGER.get_snapshot(job_id, None).unwrap();
let analysis = MANAGER.analyze_snapshot(job_id, Some(snapshot.snapshot_revision)).unwrap();
MANAGER.delete(job_id).unwrap();
// readme:end
// readme:begin
use solverforge::ClosureProblemChange;

let id = MANAGER
    .add_problem_change(
        job_id,
        ClosureProblemChange::<MySchedule, _>::new("add shift", move |director| {
            director.working_solution_mut().shifts.push(new_shift);
        }),
    )
    .unwrap();
// Applied once telemetry.problem_changes_applied >= id.get(); the next
// BestSolution carries the changed problem and may score lower than before.
// readme:end
}

pub fn report(solution: MySchedule) {
// readme:begin
use solverforge::analyze;

let analysis = analyze(&solution);

println!("Score: {}", analysis.score);
for constraint in &analysis.constraints {
    println!("  {}: {}", constraint.name, constraint.score);
}
// readme:end
}
