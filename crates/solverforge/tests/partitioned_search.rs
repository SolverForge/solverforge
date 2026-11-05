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
