use std::time::Duration;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::info;

use crate::scope::{PhaseScope, ProgressCallback, SolverScope};
use crate::stats::{
    format_duration, whole_units_per_second, CandidateTraceDisposition, CandidateTracePullToken,
};

/// Telemetry for accepted construction moves whose solution changes are still
/// buffered outside the score director.
///
/// Accepted moves remain visible while a route is assembled, but applied
/// counters and trace dispositions are published only after the caller commits
/// that route to the working solution.
#[derive(Default)]
pub(crate) struct PendingConstructionMoveTelemetry {
    accepted_count: u64,
    trace_tokens: Vec<CandidateTracePullToken>,
}

impl PendingConstructionMoveTelemetry {
    pub(crate) fn record_accepted<S, D, ProgressCb>(
        &mut self,
        phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
        trace_token: Option<CandidateTracePullToken>,
    ) where
        S: PlanningSolution,
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        phase_scope.record_move_accepted();
        self.accepted_count += 1;
        if let Some(trace_token) = trace_token {
            self.trace_tokens.push(trace_token);
        }
    }

    pub(crate) fn record_committed<S, D, ProgressCb>(
        self,
        phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    ) where
        S: PlanningSolution,
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        for trace_token in self.trace_tokens {
            phase_scope.record_candidate_trace_disposition(
                trace_token,
                CandidateTraceDisposition::Selected,
            );
            phase_scope.record_candidate_trace_disposition(
                trace_token,
                CandidateTraceDisposition::Applied,
            );
        }
        for _ in 0..self.accepted_count {
            phase_scope.record_move_applied();
        }
    }

    pub(crate) fn record_discarded<S, D, ProgressCb>(
        self,
        phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    ) where
        S: PlanningSolution,
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        for trace_token in self.trace_tokens {
            phase_scope.record_candidate_trace_disposition(
                trace_token,
                CandidateTraceDisposition::ForagerIgnored,
            );
        }
    }
}

pub(crate) fn record_construction_candidate<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    generation_duration: Duration,
    evaluation_duration: Duration,
) where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    phase_scope.record_generated_move(generation_duration);
    phase_scope.record_evaluated_move(evaluation_duration);
    report_construction_progress_if_due(phase_scope);
}

pub(crate) fn report_construction_progress_if_due<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    phase_scope.report_progress_if_due();
}

/// Runs one construction kernel inside the shared lifecycle telemetry boundary.
///
/// Construction implementations own their candidate and commit loops, while
/// this boundary owns the structured phase events consumed by the console.
pub(crate) fn run_construction_phase<S, D, ProgressCb, R>(
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    phase_index: usize,
    phase_type: &'static str,
    run: impl FnOnce(&mut PhaseScope<'_, '_, S, D, ProgressCb>) -> R,
) -> R
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut phase_scope = PhaseScope::with_phase_type(solver_scope, phase_index, phase_type);
    info!(
        event = "phase_start",
        phase = phase_type,
        phase_index = phase_index,
    );

    let result = run(&mut phase_scope);

    let duration = phase_scope.elapsed();
    let steps = phase_scope.step_count();
    let stats = phase_scope.stats();
    let moves_speed = whole_units_per_second(stats.moves_evaluated, duration);
    let calc_speed = whole_units_per_second(stats.score_calculations, duration);
    let acceptance_rate = stats.acceptance_rate() * 100.0;
    let score = phase_scope
        .solver_scope()
        .best_score()
        .or_else(|| phase_scope.solver_scope().current_score())
        .map(ToString::to_string)
        .unwrap_or_else(|| "none".to_string());

    info!(
        event = "phase_end",
        phase = phase_type,
        phase_index = phase_index,
        duration = %format_duration(duration),
        steps = steps,
        moves_generated = stats.moves_generated,
        moves_evaluated = stats.moves_evaluated,
        moves_accepted = stats.moves_accepted,
        score_calculations = stats.score_calculations,
        generation_time = %format_duration(stats.generation_time()),
        evaluation_time = %format_duration(stats.evaluation_time()),
        moves_speed = moves_speed,
        calc_speed = calc_speed,
        acceptance_rate = format!("{acceptance_rate:.1}%"),
        score = score,
    );

    result
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Metadata, Subscriber};

    use super::{
        record_construction_candidate, run_construction_phase, PendingConstructionMoveTelemetry,
    };
    use crate::scope::{SolverProgressKind, SolverProgressRef, SolverScope};
    use crate::test_utils::{create_minimal_director, TestSolution};

    #[derive(Clone, Default)]
    struct CaptureSubscriber {
        events: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct CapturedEvent {
        event: Option<String>,
        phase: Option<String>,
        phase_index: Option<u64>,
    }

    impl Visit for CapturedEvent {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            let value = format!("{value:?}").trim_matches('"').to_string();
            match field.name() {
                "event" => self.event = Some(value),
                "phase" => self.phase = Some(value),
                _ => {}
            }
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
            if field.name() == "phase_index" {
                self.phase_index = Some(value);
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
            let mut captured = CapturedEvent::default();
            event.record(&mut captured);
            self.events.lock().unwrap().push(captured);
        }

        fn enter(&self, _span: &Id) {}

        fn exit(&self, _span: &Id) {}
    }

    #[test]
    fn construction_lifecycle_wraps_the_kernel_with_structured_events() {
        let subscriber = CaptureSubscriber::default();
        let captured = Arc::clone(&subscriber.events);
        let mut solver_scope = SolverScope::new(create_minimal_director());
        solver_scope.start_solving();

        tracing::subscriber::with_default(subscriber, || {
            run_construction_phase(&mut solver_scope, 3, "Test Construction", |phase_scope| {
                phase_scope.calculate_score();
                phase_scope.update_best_solution();
            });
        });

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            CapturedEvent {
                event: Some("phase_start".to_string()),
                phase: Some("Test Construction".to_string()),
                phase_index: Some(3),
            }
        );
        assert_eq!(
            events[1],
            CapturedEvent {
                event: Some("phase_end".to_string()),
                phase: Some("Test Construction".to_string()),
                phase_index: Some(3),
            }
        );
    }

    #[test]
    fn every_construction_candidate_delegates_progress_to_the_phase_pulse() {
        let published_moves = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&published_moves);
        let mut solver_scope = SolverScope::new_with_callback(
            create_minimal_director(),
            move |progress: SolverProgressRef<'_, TestSolution>| {
                if progress.kind == SolverProgressKind::Progress {
                    captured
                        .lock()
                        .expect("progress recorder should lock")
                        .push(
                            progress
                                .telemetry
                                .phase
                                .expect("construction progress should include phase telemetry")
                                .moves_evaluated,
                        );
                }
            },
            None,
            None,
        );
        solver_scope.start_solving();

        let mut phase_scope =
            crate::scope::PhaseScope::with_phase_type(&mut solver_scope, 0, "Test Construction");
        record_construction_candidate(
            &mut phase_scope,
            std::time::Duration::ZERO,
            std::time::Duration::ZERO,
        );
        phase_scope
            .solver_scope_mut()
            .begin_phase_progress(0, "Test Construction", 0, 1);
        record_construction_candidate(
            &mut phase_scope,
            std::time::Duration::ZERO,
            std::time::Duration::ZERO,
        );

        assert_eq!(
            *published_moves
                .lock()
                .expect("progress recorder should lock"),
            vec![1, 2]
        );
    }

    #[test]
    fn pending_construction_moves_publish_applied_counts_only_on_commit() {
        let mut solver_scope = SolverScope::new(create_minimal_director());
        let mut phase_scope =
            crate::scope::PhaseScope::with_phase_type(&mut solver_scope, 0, "Test Construction");
        let mut pending_move_telemetry = PendingConstructionMoveTelemetry::default();

        pending_move_telemetry.record_accepted(&mut phase_scope, None);
        pending_move_telemetry.record_accepted(&mut phase_scope, None);
        assert_eq!(phase_scope.stats().moves_accepted, 2);
        assert_eq!(phase_scope.stats().moves_applied, 0);

        pending_move_telemetry.record_committed(&mut phase_scope);
        assert_eq!(phase_scope.stats().moves_applied, 2);
        drop(phase_scope);
        assert_eq!(solver_scope.stats().moves_accepted, 2);
        assert_eq!(solver_scope.stats().moves_applied, 2);
    }
}
