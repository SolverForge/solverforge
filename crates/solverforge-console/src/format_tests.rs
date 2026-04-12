use super::*;
use std::sync::{Arc, Mutex};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::{Layer, Registry};

#[derive(Clone)]
struct CaptureLayer {
    outputs: Arc<Mutex<Vec<String>>>,
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let output = format_event(&visitor, *event.metadata().level());
        if !output.is_empty() {
            self.outputs.lock().unwrap().push(output);
        }
    }
}

fn capture_events(f: impl FnOnce()) -> Vec<String> {
    let outputs = Arc::new(Mutex::new(Vec::new()));
    let subscriber = Registry::default().with(CaptureLayer {
        outputs: outputs.clone(),
    });

    tracing::subscriber::with_default(subscriber, f);
    let captured = outputs.lock().unwrap().clone();
    captured
}

#[test]
fn format_duration_covers_milliseconds_seconds_and_minutes() {
    assert_eq!(format_duration_ms(750), "750ms");
    assert_eq!(format_duration_ms(2_500), "2.50s");
    assert_eq!(format_duration_ms(125_000), "2m 5s");
}

#[test]
fn calculate_problem_scale_handles_zero_and_nonzero_inputs() {
    assert_eq!(calculate_problem_scale(0, 10), "0");
    assert_eq!(calculate_problem_scale(10, 100), "1.000 x 10^20");
}

#[test]
fn format_score_handles_hard_soft_and_simple_scores() {
    let hard_soft = format_score("-2hard/5soft");
    assert!(hard_soft.contains("-2hard"));
    assert!(hard_soft.contains("5soft"));

    let simple = format_score("-7");
    assert!(simple.contains("-7"));

    let fallback = format_score("N/A");
    assert!(fallback.contains("N/A"));
}

#[test]
fn format_event_renders_progress_and_trace_steps() {
    let progress = EventVisitor {
        event: Some("progress".to_string()),
        steps: Some(12_345),
        speed: Some(678),
        current_score: Some("0hard/9soft".to_string()),
        ..EventVisitor::default()
    };
    let progress_output = format_event(&progress, Level::INFO);
    assert!(progress_output.contains("steps"));
    assert!(progress_output.contains("678"));
    assert!(progress_output.contains("0hard"));

    let outputs = capture_events(|| {
        tracing::trace!(
            target: "solverforge_solver::test",
            event = "step",
            step = 42u64,
            move_index = 3u64,
            score = "-1hard/0soft",
            accepted = true,
        );
    });

    let step_output = outputs
        .iter()
        .find(|output| output.contains("Step"))
        .cloned()
        .expect("expected trace step output");
    assert!(step_output.contains("Step"));
    assert!(step_output.contains("Entity"));
    assert!(step_output.contains("3"));
    assert!(step_output.contains("-1hard"));
}

#[test]
fn format_event_renders_solve_start_and_end_summaries() {
    let outputs = capture_events(|| {
        tracing::info!(
            target: "solverforge_solver::test",
            event = "solve_start",
            entity_count = 120u64,
            solve_shape = "list",
            value_count = 25u64,
            constraint_count = 7u64,
            time_limit_secs = 30u64,
        );
    });

    let start_output = outputs
        .iter()
        .find(|output| output.contains("Solving"))
        .cloned()
        .expect("expected solve_start output");
    assert!(start_output.contains("Solving"));
    assert!(start_output.contains("elements"));
    assert!(start_output.contains("120"));
    assert!(start_output.contains("25"));
    assert!(start_output.contains("constraints"));

    let end = EventVisitor {
        event: Some("solve_end".to_string()),
        score: Some("0hard/-15soft".to_string()),
        ..EventVisitor::default()
    };
    let end_output = format_event(&end, Level::INFO);
    assert!(end_output.contains("Solving complete"));
    assert!(end_output.contains("FEASIBLE"));
    assert!(end_output.contains("Final Score:"));
}
