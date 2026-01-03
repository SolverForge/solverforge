//! Benchmark for typed solver using full Solver infrastructure.
//!
//! Run with: cargo run --release -p employee-scheduling --bin typed_bench

use employee_scheduling::{demo_data, typed_solver};
use std::time::Duration;

fn main() {
    let schedule = demo_data::generate(demo_data::DemoData::Large);
    let n_shifts = schedule.shifts.len();
    let n_employees = schedule.employees.len();

    println!("Typed Solver Benchmark (Full Infrastructure)");
    println!("  Shifts: {}", n_shifts);
    println!("  Employees: {}", n_employees);
    println!();

    // Solve with 10 second time limit
    let config = typed_solver::TypedSolverConfig {
        time_limit: Duration::from_secs(10),
        late_acceptance_size: 400,
    };

    println!("Solving with {:?} time limit...", config.time_limit);
    let start = std::time::Instant::now();
    let result = typed_solver::solve(schedule, config);
    let elapsed = start.elapsed();

    println!();
    println!("Results:");
    println!("  Time: {:.2?}", elapsed);
    println!("  Final score: {:?}", result.score);

    // Count assignments
    let assigned = result.shifts.iter().filter(|s| s.employee_idx.is_some()).count();
    println!("  Assigned shifts: {}/{}", assigned, n_shifts);
}
