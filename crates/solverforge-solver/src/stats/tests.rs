use std::time::Duration;

use super::*;

#[test]
fn solver_snapshot_preserves_exact_counts_and_durations() {
    let mut stats = SolverStats::default();
    stats.start();
    stats.record_step();
    stats.record_generated_batch(3, Duration::from_millis(4));
    stats.record_evaluated_move(Duration::from_millis(5));
    stats.record_move_accepted();
    stats.record_score_calculation();

    let snapshot = stats.snapshot();

    assert_eq!(snapshot.step_count, 1);
    assert_eq!(snapshot.moves_generated, 3);
    assert_eq!(snapshot.moves_evaluated, 1);
    assert_eq!(snapshot.moves_accepted, 1);
    assert_eq!(snapshot.score_calculations, 1);
    assert_eq!(snapshot.generation_time, Duration::from_millis(4));
    assert_eq!(snapshot.evaluation_time, Duration::from_millis(5));
}

#[test]
fn phase_stats_track_generation_and_evaluation_separately() {
    let mut stats = PhaseStats::new(2, "LocalSearch");
    stats.record_step();
    stats.record_generated_batch(7, Duration::from_millis(8));
    stats.record_evaluated_move(Duration::from_millis(9));
    stats.record_move_accepted();
    stats.record_score_calculation();

    assert_eq!(stats.phase_index, 2);
    assert_eq!(stats.phase_type, "LocalSearch");
    assert_eq!(stats.step_count, 1);
    assert_eq!(stats.moves_generated, 7);
    assert_eq!(stats.moves_evaluated, 1);
    assert_eq!(stats.moves_accepted, 1);
    assert_eq!(stats.score_calculations, 1);
    assert_eq!(stats.generation_time(), Duration::from_millis(8));
    assert_eq!(stats.evaluation_time(), Duration::from_millis(9));
}

#[test]
fn solver_snapshot_includes_selector_level_telemetry() {
    let mut stats = SolverStats::default();
    stats.start();
    stats.record_selector_generated(2, 3, Duration::from_millis(4));
    stats.record_selector_evaluated(2, Duration::from_millis(5));
    stats.record_selector_accepted(2);
    stats.record_selector_applied(2);

    let snapshot = stats.snapshot();

    assert_eq!(snapshot.moves_generated, 3);
    assert_eq!(snapshot.moves_evaluated, 1);
    assert_eq!(snapshot.moves_accepted, 1);
    assert_eq!(snapshot.moves_applied, 1);
    assert_eq!(snapshot.selector_telemetry.len(), 1);
    assert_eq!(snapshot.selector_telemetry[0].selector_index, 2);
    assert_eq!(snapshot.selector_telemetry[0].selector_label, "selector-2");
    assert_eq!(snapshot.selector_telemetry[0].moves_generated, 3);
    assert_eq!(snapshot.selector_telemetry[0].moves_evaluated, 1);
    assert_eq!(snapshot.selector_telemetry[0].moves_accepted, 1);
    assert_eq!(snapshot.selector_telemetry[0].moves_applied, 1);
}

#[test]
fn solver_snapshot_prefers_observed_selector_label() {
    let mut stats = SolverStats::default();
    stats.record_selector_generated(2, 1, Duration::from_millis(1));
    stats.record_selector_generated_with_label(2, "conflict_repair", 1, Duration::from_millis(2));

    let snapshot = stats.snapshot();

    assert_eq!(
        snapshot.selector_telemetry[0].selector_label,
        "conflict_repair"
    );
    assert_eq!(snapshot.selector_telemetry[0].moves_generated, 2);
}

#[test]
fn solver_coverage_remaining_aggregates_by_group() {
    let mut stats = SolverStats::default();

    stats.record_coverage_required_remaining("group_a", 3);
    stats.record_coverage_required_remaining("group_b", 0);
    assert_eq!(stats.coverage_required_remaining, 3);
    assert_eq!(stats.snapshot().coverage_required_remaining, 3);

    stats.record_coverage_required_remaining("group_a", 0);
    assert_eq!(stats.coverage_required_remaining, 0);
    assert_eq!(stats.snapshot().coverage_required_remaining, 0);
}

#[test]
fn unattributed_applied_moves_do_not_create_selector_zero_telemetry() {
    let mut stats = SolverStats::default();
    stats.record_generated_move(Duration::from_millis(1));
    stats.record_evaluated_move(Duration::from_millis(2));
    stats.record_move_accepted();
    stats.record_move_applied();

    let snapshot = stats.snapshot();

    assert_eq!(snapshot.moves_generated, 1);
    assert_eq!(snapshot.moves_evaluated, 1);
    assert_eq!(snapshot.moves_accepted, 1);
    assert_eq!(snapshot.moves_applied, 1);
    assert!(snapshot.selector_telemetry.is_empty());
}

#[test]
fn throughput_helpers_use_stage_specific_durations() {
    let mut solver_stats = SolverStats::default();
    solver_stats.start();
    solver_stats.record_generated_batch(5, Duration::from_millis(7));
    solver_stats.record_evaluated_move(Duration::from_millis(11));

    let mut phase_stats = PhaseStats::new(1, "LocalSearch");
    phase_stats.record_generated_batch(3, Duration::from_millis(13));
    phase_stats.record_evaluated_move(Duration::from_millis(17));

    assert_eq!(
        solver_stats.generated_throughput(),
        Throughput {
            count: 5,
            elapsed: Duration::from_millis(7),
        }
    );
    assert_eq!(
        solver_stats.evaluated_throughput(),
        Throughput {
            count: 1,
            elapsed: Duration::from_millis(11),
        }
    );
    assert_eq!(
        phase_stats.generated_throughput(),
        Throughput {
            count: 3,
            elapsed: Duration::from_millis(13),
        }
    );
    assert_eq!(
        phase_stats.evaluated_throughput(),
        Throughput {
            count: 1,
            elapsed: Duration::from_millis(17),
        }
    );
}

#[test]
fn whole_units_per_second_uses_integer_rate_math() {
    assert_eq!(whole_units_per_second(3, Duration::from_millis(2_000)), 1);
    assert_eq!(whole_units_per_second(9, Duration::from_secs(2)), 4);
    assert_eq!(whole_units_per_second(5, Duration::ZERO), 0);
}

#[test]
fn format_duration_uses_exact_integer_units() {
    assert_eq!(format_duration(Duration::from_millis(750)), "750ms");
    assert_eq!(format_duration(Duration::from_millis(2_500)), "2s 500ms");
    assert_eq!(format_duration(Duration::from_secs(125)), "2m 5s");
    assert_eq!(format_duration(Duration::from_micros(42)), "42us");
}
