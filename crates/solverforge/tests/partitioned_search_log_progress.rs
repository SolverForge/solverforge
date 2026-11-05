//! `log_progress` on a `partitioned_search` phase logs the phase's start and
//! end. Its own test binary: the capturing subscriber is process-global,
//! because the solve runs on a solver thread.

use std::sync::{Arc, Mutex, OnceLock};

use solverforge::{SolverEvent, SolverManager};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

#[path = "partitioned_search/domain/mod.rs"]
mod domain;

use domain::{take_records, Plan};

static MANAGER: SolverManager<Plan> = SolverManager::new();

/// Captured `PartitionedSearch` events: name and `partition_count`.
type Events = Arc<Mutex<Vec<(String, Option<u64>)>>>;

#[derive(Clone, Default)]
struct CaptureSubscriber {
    events: Events,
}

#[derive(Default)]
struct PartitionedEvent {
    event: Option<String>,
    phase: Option<String>,
    partition_count: Option<u64>,
}

impl Visit for PartitionedEvent {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let value = format!("{value:?}").trim_matches('"').to_string();
        match field.name() {
            "event" => self.event = Some(value),
            "phase" => self.phase = Some(value),
            _ => {}
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_debug(field, &value);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "partition_count" {
            self.partition_count = Some(value);
        }
    }
}

impl Subscriber for CaptureSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut captured = PartitionedEvent::default();
        event.record(&mut captured);
        if captured.phase.as_deref() == Some("PartitionedSearch") {
            if let Some(name) = captured.event {
                self.events
                    .lock()
                    .unwrap()
                    .push((name, captured.partition_count));
            }
        }
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

fn captured() -> &'static Events {
    static EVENTS: OnceLock<Events> = OnceLock::new();
    EVENTS.get_or_init(|| {
        let subscriber = CaptureSubscriber::default();
        let events = Arc::clone(&subscriber.events);
        tracing::subscriber::set_global_default(subscriber)
            .expect("this test binary installs the only global subscriber");
        events
    })
}

fn solve(plan: Plan) {
    let (job_id, mut receiver) = MANAGER.solve(plan).expect("solve starts");
    loop {
        match receiver
            .blocking_recv()
            .expect("event stream should reach a terminal event")
        {
            SolverEvent::Completed { .. } => break,
            SolverEvent::Failed { error, .. } => panic!("solve failed: {error}"),
            SolverEvent::Cancelled { .. } => panic!("solve was cancelled"),
            _ => {}
        }
    }
    MANAGER.delete(job_id).expect("the completed job deletes");
}

const LOGGED: &str = r#"
[[phases]]
type = "partitioned_search"
partitioner = "by_task_id"
log_progress = true

[[phases.child_phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases.child_phases]]
type = "custom"
name = "record"
"#;

const QUIET: &str = r#"
[[phases]]
type = "partitioned_search"
partitioner = "by_task_id"

[[phases.child_phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"
"#;

#[test]
fn log_progress_logs_the_partitioned_phase() {
    let events = captured();

    solve(Plan::new(QUIET));
    assert!(
        events.lock().unwrap().is_empty(),
        "log_progress defaults to off"
    );

    solve(Plan::new(LOGGED));
    let logged = std::mem::take(&mut *events.lock().unwrap());
    assert_eq!(
        logged,
        vec![
            ("phase_start".to_string(), Some(4)),
            ("phase_end".to_string(), None),
        ]
    );
    let records: Vec<domain::Record> = take_records(LOGGED);
    assert_eq!(records.len(), 4);
    assert!(records.iter().all(|record| record.all_assigned
        && record.task_ids.len() == 2
        && record.pool_threads >= 1));
    let mut threads: Vec<String> = records
        .iter()
        .map(|record| format!("{:?}", record.thread))
        .collect();
    threads.sort();
    threads.dedup();
    let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get());
    assert!(
        threads.len() <= cores.min(4),
        "`auto` runs one thread per partition up to the cores"
    );
}
