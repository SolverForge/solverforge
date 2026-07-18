use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::info;

use crate::scope::{PhaseScope, ProgressCallback, SolverScope};
use crate::stats::{format_duration, whole_units_per_second};

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

    use super::run_construction_phase;
    use crate::scope::SolverScope;
    use crate::test_utils::create_minimal_director;

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
}
