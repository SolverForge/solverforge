use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};

use solverforge_config::{CandidateTraceConfig, SolverConfig};

use super::Solver;
use crate::manager::SolverTerminalReason;
use crate::phase::{Phase, PhaseSequence};
use crate::scope::{ProgressCallback, SolverScope};
use crate::stats::CandidateTracePhasePlan;
use crate::termination::StepCountTermination;
use crate::test_utils::{create_minimal_director, TestDirector, TestSolution};

#[derive(Clone, Debug)]
struct LifecycleRecorder {
    label: &'static str,
    events: Arc<Mutex<Vec<String>>>,
}

impl LifecycleRecorder {
    fn record(&self, event: &'static str) {
        self.events
            .lock()
            .expect("lifecycle recorder must not be poisoned")
            .push(format!("{event}:{}", self.label));
    }
}

#[derive(Clone, Debug)]
struct RecordingPhase(LifecycleRecorder);

impl<ProgressCb> Phase<TestSolution, TestDirector, ProgressCb> for RecordingPhase
where
    ProgressCb: ProgressCallback<TestSolution>,
{
    fn solve(
        &mut self,
        _solver_scope: &mut SolverScope<'_, TestSolution, TestDirector, ProgressCb>,
    ) {
        self.0.record("solve");
    }

    fn phase_type_name(&self) -> &'static str {
        "RecordingPhase"
    }

    fn on_solver_terminal(
        &mut self,
        _solver_scope: &mut SolverScope<'_, TestSolution, TestDirector, ProgressCb>,
    ) {
        self.0.record("terminal");
    }
}

#[derive(Debug)]
struct OrdinaryPhase {
    solve_count: Arc<AtomicUsize>,
}

impl<ProgressCb> Phase<TestSolution, TestDirector, ProgressCb> for OrdinaryPhase
where
    ProgressCb: ProgressCallback<TestSolution>,
{
    fn solve(
        &mut self,
        solver_scope: &mut SolverScope<'_, TestSolution, TestDirector, ProgressCb>,
    ) {
        self.solve_count.fetch_add(1, Ordering::SeqCst);
        solver_scope.increment_step_count();
    }

    fn phase_type_name(&self) -> &'static str {
        "OrdinaryPhase"
    }
}

#[derive(Debug)]
struct TraceFinalizingPhase {
    terminal_count: Arc<AtomicUsize>,
    terminal_plan: CandidateTracePhasePlan,
}

#[derive(Debug)]
struct BlockingTerminalPhase {
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
}

impl<ProgressCb> Phase<TestSolution, TestDirector, ProgressCb> for BlockingTerminalPhase
where
    ProgressCb: ProgressCallback<TestSolution>,
{
    fn solve(
        &mut self,
        _solver_scope: &mut SolverScope<'_, TestSolution, TestDirector, ProgressCb>,
    ) {
    }

    fn phase_type_name(&self) -> &'static str {
        "BlockingTerminalPhase"
    }

    fn on_solver_terminal(
        &mut self,
        _solver_scope: &mut SolverScope<'_, TestSolution, TestDirector, ProgressCb>,
    ) {
        self.entered.wait();
        self.release.wait();
    }
}

impl<ProgressCb> Phase<TestSolution, TestDirector, ProgressCb> for TraceFinalizingPhase
where
    ProgressCb: ProgressCallback<TestSolution>,
{
    fn solve(
        &mut self,
        _solver_scope: &mut SolverScope<'_, TestSolution, TestDirector, ProgressCb>,
    ) {
    }

    fn phase_type_name(&self) -> &'static str {
        "TraceFinalizingPhase"
    }

    fn on_solver_terminal(
        &mut self,
        solver_scope: &mut SolverScope<'_, TestSolution, TestDirector, ProgressCb>,
    ) {
        self.terminal_count.fetch_add(1, Ordering::SeqCst);
        solver_scope.finalize_candidate_trace_resolved_phase_plan(self.terminal_plan.clone());
    }
}

fn recording_phase(label: &'static str, events: &Arc<Mutex<Vec<String>>>) -> RecordingPhase {
    RecordingPhase(LifecycleRecorder {
        label,
        events: Arc::clone(events),
    })
}

fn recorded_events(events: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    events
        .lock()
        .expect("lifecycle recorder must not be poisoned")
        .clone()
}

#[test]
fn phase_default_terminal_hook_preserves_ordinary_solver_semantics() {
    let solve_count = Arc::new(AtomicUsize::new(0));
    let result = Solver::new((OrdinaryPhase {
        solve_count: Arc::clone(&solve_count),
    },))
    .solve(create_minimal_director());

    assert_eq!(solve_count.load(Ordering::SeqCst), 1);
    assert_eq!(result.step_count(), 1);
    assert_eq!(result.terminal_reason(), SolverTerminalReason::Completed);
}

#[test]
fn terminal_hook_propagates_once_through_phase_composites() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sequence = PhaseSequence::new(vec![
        recording_phase("sequence-a", &events),
        recording_phase("sequence-b", &events),
    ]);
    let nested_tuple = (
        ((), recording_phase("tuple-a", &events)),
        recording_phase("tuple-b", &events),
    );
    let result = Solver::new((sequence, nested_tuple)).solve(create_minimal_director());

    assert_eq!(result.terminal_reason(), SolverTerminalReason::Completed);
    assert_eq!(
        recorded_events(&events),
        [
            "solve:sequence-a",
            "solve:sequence-b",
            "solve:tuple-a",
            "solve:tuple-b",
            "terminal:sequence-a",
            "terminal:sequence-b",
            "terminal:tuple-a",
            "terminal:tuple-b",
        ]
    );
}

#[test]
fn terminal_hook_runs_once_when_cancellation_skips_every_phase() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let cancel = AtomicBool::new(true);
    let result = Solver::new((
        recording_phase("first", &events),
        recording_phase("second", &events),
    ))
    .with_terminate(&cancel)
    .solve(create_minimal_director());

    assert_eq!(result.terminal_reason(), SolverTerminalReason::Cancelled);
    assert_eq!(
        recorded_events(&events),
        ["terminal:first", "terminal:second"]
    );
}

#[test]
fn terminal_hook_runs_once_when_config_termination_skips_every_phase() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let result = Solver::new((
        recording_phase("first", &events),
        recording_phase("second", &events),
    ))
    .with_termination(StepCountTermination::new(0))
    .solve(create_minimal_director());

    assert_eq!(
        result.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
    assert_eq!(
        recorded_events(&events),
        ["terminal:first", "terminal:second"]
    );
}

#[test]
fn terminal_hook_can_finalize_candidate_trace_before_final_stats_are_taken() {
    let terminal_plan = CandidateTracePhasePlan::known(
        "test.terminal_phase_plan",
        [("status", "completed")],
        Vec::new(),
    );
    let terminal_count = Arc::new(AtomicUsize::new(0));
    let config = SolverConfig {
        candidate_trace: Some(CandidateTraceConfig::new(
            std::num::NonZeroUsize::new(1).expect("one is non-zero"),
        )),
        ..SolverConfig::default()
    };

    let result = Solver::new((TraceFinalizingPhase {
        terminal_count: Arc::clone(&terminal_count),
        terminal_plan: terminal_plan.clone(),
    },))
    .with_config(config)
    .solve(create_minimal_director());

    assert_eq!(terminal_count.load(Ordering::SeqCst), 1);
    let trace = result
        .stats()
        .snapshot()
        .candidate_trace
        .expect("enabled candidate trace must be retained in final stats");
    assert_eq!(trace.header.resolved_phase_plan, terminal_plan);
    assert!(trace.header.resolved_phase_plan_complete);
}

#[test]
fn cancellation_accepted_during_terminal_hook_is_settled_before_result_extraction() {
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = Arc::clone(&cancel);
    let worker_entered = Arc::clone(&entered);
    let worker_release = Arc::clone(&release);
    let (sender, receiver) = std::sync::mpsc::channel();

    rayon::spawn(move || {
        let result = Solver::new((BlockingTerminalPhase {
            entered: worker_entered,
            release: worker_release,
        },))
        .with_terminate(worker_cancel.as_ref())
        .solve(create_minimal_director());
        sender
            .send(result.terminal_reason())
            .expect("test receiver remains alive");
    });

    entered.wait();
    cancel.store(true, Ordering::SeqCst);
    release.wait();

    assert_eq!(
        receiver
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("solver settles cancellation after the hook"),
        SolverTerminalReason::Cancelled
    );
}
