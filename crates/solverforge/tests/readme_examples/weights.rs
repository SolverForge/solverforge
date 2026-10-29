use solverforge::prelude::*;
use solverforge::{ClosureProblemChange, SolverManager};

use super::weighted::Schedule;

static MANAGER: SolverManager<Schedule> = SolverManager::new();

pub fn reweight(problem: Schedule) -> Result<(), Box<dyn std::error::Error>> {
// readme:begin
let mut weights = ConstraintWeights::new();
weights.set("Travel time", 10).set("Overtime", 0);
Schedule::validate_constraint_weights(&weights)?;
// readme:end
    let (job_id, _events) = MANAGER.solve(problem)?;
// readme:begin
MANAGER.add_problem_change(
    job_id,
    ClosureProblemChange::<Schedule, _>::new("reweight", move |director| {
        director.working_solution_mut().weights = weights.clone();
    }),
)?;
// readme:end
    Ok(())
}
