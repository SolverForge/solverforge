//! `partitioned_search` is run by the framework: the registered partitioner
//! splits the solution, `child_phases` solve each partition on
//! `thread_count` threads, and the partitions are merged back.

use solverforge::{SolverEvent, SolverManager};

#[path = "partitioned_search/domain/mod.rs"]
mod domain;

use domain::{take_records, Plan};

static MANAGER: SolverManager<Plan> = SolverManager::new();

fn solve(plan: Plan) -> Plan {
    let (job_id, mut receiver) = MANAGER.solve(plan).expect("solve starts");
    let solution = loop {
        match receiver
            .blocking_recv()
            .expect("event stream should reach a terminal event")
        {
            SolverEvent::Completed { solution, .. } => break solution,
            SolverEvent::Failed { error, .. } => panic!("solve failed: {error}"),
            SolverEvent::Cancelled { .. } => panic!("solve was cancelled"),
            _ => {}
        }
    };
    MANAGER.delete(job_id).expect("the completed job deletes");
    solution
}

fn distinct_threads(records: &[domain::Record]) -> usize {
    let mut threads: Vec<_> = records.iter().map(|record| record.thread).collect();
    threads.sort_by_key(|thread| format!("{thread:?}"));
    threads.dedup();
    threads.len()
}

const TWO_THREADS: &str = r#"
[[phases]]
type = "partitioned_search"
partitioner = "by_task_id"
thread_count = { count = 2 }

[[phases.child_phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases.child_phases]]
type = "custom"
name = "record"
"#;

#[test]
fn child_phases_solve_every_partition_on_the_configured_threads() {
    let solution = solve(Plan::new(TWO_THREADS));

    assert!(solution
        .tasks
        .iter()
        .all(|task| task.resource_idx.is_some()));
    assert_eq!(solution.score.map(|score| score.hard()), Some(0));

    let mut records = take_records(TWO_THREADS);
    records.sort_by_key(|record| record.task_ids[0]);
    let partitions: Vec<Vec<usize>> = records.iter().map(|r| r.task_ids.clone()).collect();
    assert_eq!(
        partitions,
        vec![vec![0, 4], vec![1, 5], vec![2, 6], vec![3, 7]]
    );
    for record in &records {
        assert!(record.all_assigned, "construction ran before `record`");
        assert_eq!(
            record.pool_threads, 2,
            "partitions run on a two-thread pool"
        );
    }
    assert!(distinct_threads(&records) <= 2);
}

const ONE_THREAD: &str = r#"
[[phases]]
type = "partitioned_search"
partitioner = "by_task_id"
thread_count = "none"

[[phases.child_phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases.child_phases]]
type = "custom"
name = "record"
"#;

#[test]
fn thread_count_none_solves_the_partitions_on_the_solver_thread() {
    let solution = solve(Plan::new(ONE_THREAD));

    assert!(solution
        .tasks
        .iter()
        .all(|task| task.resource_idx.is_some()));
    let records = take_records(ONE_THREAD);
    assert_eq!(records.len(), 4);
    assert_eq!(distinct_threads(&records), 1);
}

const DEFAULT_CHILDREN: &str = r#"
[[phases]]
type = "partitioned_search"
partitioner = "by_task_id"
"#;

#[test]
fn empty_child_phases_run_the_default_phases_in_each_partition() {
    let solution = solve(Plan::new(DEFAULT_CHILDREN));

    assert!(solution
        .tasks
        .iter()
        .all(|task| task.resource_idx.is_some()));
    assert_eq!(solution.score.map(|score| score.hard()), Some(0));
}

const BOUNDED: &str = r#"
[termination]
seconds_spent_limit = 60

[[phases]]
type = "partitioned_search"
partitioner = "by_task_id"
thread_count = "none"

[phases.termination]
step_count_limit = 12

[[phases.child_phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases.child_phases]]
type = "local_search"

[phases.child_phases.acceptor]
type = "late_acceptance"
late_acceptance_size = 4

[phases.child_phases.move_selector]
type = "change_move_selector"

[[phases]]
type = "custom"
name = "record"
"#;

#[test]
fn the_phase_termination_ends_the_partitions_and_keeps_their_work() {
    let started = std::time::Instant::now();
    let (job_id, mut receiver) = MANAGER.solve(Plan::new(BOUNDED)).expect("solve starts");
    let (solution, reason) = loop {
        match receiver
            .blocking_recv()
            .expect("event stream should reach a terminal event")
        {
            SolverEvent::Completed { metadata, solution } => {
                break (solution, metadata.terminal_reason)
            }
            SolverEvent::Failed { error, .. } => panic!("solve failed: {error}"),
            SolverEvent::Cancelled { .. } => panic!("solve was cancelled"),
            _ => {}
        }
    };
    MANAGER.delete(job_id).expect("the completed job deletes");

    // The child local search has no limit of its own: only the phase's 12
    // steps per partition end it, well before the solve's 60 s.
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    assert_eq!(reason, Some(solverforge::SolverTerminalReason::Completed));
    assert!(solution
        .tasks
        .iter()
        .all(|task| task.resource_idx.is_some()));

    // The phase end merged the partitions and the solve went on to the next
    // phase, which saw the whole, merged plan.
    let records = take_records(BOUNDED);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].task_ids, (0..8).collect::<Vec<_>>());
    assert!(records[0].all_assigned);
}

const RECOMMENDED: &str = r#"
[[phases]]
type = "partitioned_search"
partitioner = "by_task_id_two_at_once"

[[phases.child_phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases.child_phases]]
type = "custom"
name = "record"
"#;

#[test]
fn auto_threads_follow_the_recommended_partition_count() {
    let solution = solve(Plan::new(RECOMMENDED));
    assert!(solution
        .tasks
        .iter()
        .all(|task| task.resource_idx.is_some()));

    let records = take_records(RECOMMENDED);
    assert_eq!(records.len(), 4);
    let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get());
    let expected = cores.min(2);
    if expected > 1 {
        assert!(records.iter().all(|record| record.pool_threads == expected));
    }
    assert!(distinct_threads(&records) <= expected);
}
