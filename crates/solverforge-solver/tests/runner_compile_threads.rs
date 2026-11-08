//! Compiling a runner whose local search uses `move_thread_count = "auto"`
//! starts no threads: the policy resolves against the pool the phase solves
//! in, so the global Rayon pool is not started at compile time.
//!
//! This is its own test binary because it reads the process's thread count,
//! which any other test starting a pool would change. It reads it from
//! procfs, so it runs on Linux only.
#![cfg(target_os = "linux")]

use solverforge_config::{
    ChangeMoveConfig, LocalSearchConfig, MoveSelectorConfig, MoveThreadCount, PhaseConfig,
    SolverConfig,
};
use solverforge_solver::__bench::CompiledRuntimeRunner;

mod support;

fn thread_count() -> usize {
    std::fs::read_dir("/proc/self/task")
        .expect("procfs lists this process's threads")
        .count()
}

#[test]
fn compiling_an_auto_local_search_starts_no_threads() {
    let config = SolverConfig {
        random_seed: Some(7),
        move_thread_count: MoveThreadCount::Auto,
        phases: vec![PhaseConfig::LocalSearch(LocalSearchConfig {
            move_selector: Some(MoveSelectorConfig::ChangeMoveSelector(
                ChangeMoveConfig::default(),
            )),
            ..LocalSearchConfig::default()
        })],
        ..SolverConfig::default()
    };
    let before = thread_count();
    let runner =
        CompiledRuntimeRunner::compile(&config, support::context()).expect("the runner compiles");
    assert_eq!(thread_count(), before);
    drop(runner);
}
