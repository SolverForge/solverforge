//! Benchmark for incremental scoring performance.
//!
//! Run with: cargo run --release -p employee-scheduling --bin bench

use employee_scheduling::{constraints, demo_data};
use solverforge::TypedScoreDirector;
use std::time::Instant;

fn main() {
    let schedule = demo_data::generate(demo_data::DemoData::Large);
    let n_shifts = schedule.shifts.len();
    let n_employees = schedule.employees.len();

    println!("Benchmark: Incremental Scoring (Fluent API)");
    println!("  Shifts: {}", n_shifts);
    println!("  Employees: {}", n_employees);
    println!();

    let constraint_set = constraints::create_fluent_constraints();
    let mut director = TypedScoreDirector::new(schedule, constraint_set);

    // Initialize
    let init_start = Instant::now();
    let initial_score = director.calculate_score();
    println!("Initial score: {} ({:?})", initial_score, init_start.elapsed());
    println!();

    // Benchmark: deterministic do/undo cycle for each shift×employee combination
    // This measures pure incremental scoring throughput
    let bench_start = Instant::now();
    let mut moves: u64 = 0;

    for shift_idx in 0..n_shifts {
        let old_idx = director.working_solution().shifts[shift_idx].employee_idx;

        for emp_idx in 0..n_employees {
            // Do move
            director.before_variable_changed(shift_idx);
            director.working_solution_mut().shifts[shift_idx].employee_idx = Some(emp_idx);
            director.after_variable_changed(shift_idx);
            let _ = director.get_score();
            moves += 1;

            // Undo move
            director.before_variable_changed(shift_idx);
            director.working_solution_mut().shifts[shift_idx].employee_idx = old_idx;
            director.after_variable_changed(shift_idx);
            let _ = director.get_score();
            moves += 1;
        }
    }

    let elapsed = bench_start.elapsed();
    let moves_per_sec = moves as f64 / elapsed.as_secs_f64();

    println!("Results:");
    println!("  Moves: {}", moves);
    println!("  Time: {:.2?}", elapsed);
    println!("  Moves/sec: {:.0}", moves_per_sec);

    // Verify score unchanged after all do/undo cycles
    let final_score = director.get_score();
    assert_eq!(initial_score, final_score, "Score corrupted!");
    println!("  Final score: {} (verified)", final_score);
}
