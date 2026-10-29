use solverforge::SolverConfig;

use super::nurses::Schedule;

pub fn load_and_solve(plan: Schedule) -> Result<Schedule, Box<dyn std::error::Error>> {
// readme:begin
let config = SolverConfig::load("solver.toml")?;
// readme:end
// readme:begin
let solved = plan.solve_with_config(config)?;
// readme:end
    Ok(solved)
}
