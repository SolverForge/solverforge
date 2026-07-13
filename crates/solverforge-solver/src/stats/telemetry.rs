/* Solver statistics (zero-erasure).

Stack-allocated statistics for solver and phase performance tracking.
*/

use std::time::Duration;

use super::CandidateTraceTelemetry;

/* Solver-level statistics.

Tracks aggregate metrics across all phases of a solve run.

# Example

```
use solverforge_solver::stats::SolverStats;
use std::time::Duration;

let mut stats = SolverStats::default();
stats.start();
stats.record_step();
stats.record_generated_move(Duration::from_millis(1));
stats.record_evaluated_move(Duration::from_millis(2));
stats.record_move_accepted();
stats.record_generated_move(Duration::from_millis(1));
stats.record_evaluated_move(Duration::from_millis(2));

assert_eq!(stats.step_count, 1);
assert_eq!(stats.moves_evaluated, 2);
assert_eq!(stats.moves_accepted, 1);
```
*/
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SelectorTelemetry {
    pub selector_index: usize,
    pub selector_label: String,
    /// Candidate moves actually yielded by this selector.
    /// This is runtime work, not the size of an unconsumed neighborhood.
    pub moves_generated: u64,
    pub moves_evaluated: u64,
    pub moves_accepted: u64,
    pub moves_applied: u64,
    pub moves_not_doable: u64,
    pub moves_acceptor_rejected: u64,
    pub moves_forager_ignored: u64,
    pub moves_hard_improving: u64,
    pub moves_hard_neutral: u64,
    pub moves_hard_worse: u64,
    pub conflict_repair_provider_generated: u64,
    pub conflict_repair_duplicate_filtered: u64,
    pub conflict_repair_illegal_filtered: u64,
    pub conflict_repair_not_doable_filtered: u64,
    pub conflict_repair_hard_improving: u64,
    pub conflict_repair_exposed: u64,
    pub generation_time: Duration,
    pub evaluation_time: Duration,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MoveTelemetry {
    pub move_label: String,
    /// Candidate moves actually yielded to the engine across all phases.
    /// Exhaust a cursor or query selector sizing separately for logical size.
    pub moves_generated: u64,
    pub moves_evaluated: u64,
    pub moves_accepted: u64,
    pub moves_applied: u64,
    pub moves_not_doable: u64,
    pub moves_acceptor_rejected: u64,
    pub moves_forager_ignored: u64,
    pub moves_score_improving: u64,
    pub moves_applied_improving: u64,
    pub moves_score_equal: u64,
    pub moves_score_worse: u64,
    pub moves_rejected_improving: u64,
    pub applied_score_improvement: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhaseTelemetry {
    pub phase_index: usize,
    pub phase_type: String,
    pub elapsed: Duration,
    pub step_count: u64,
    /// Candidate moves actually yielded to the engine during this phase.
    /// This is runtime work, not the size of an unconsumed neighborhood.
    pub moves_generated: u64,
    pub moves_evaluated: u64,
    pub moves_accepted: u64,
    pub moves_applied: u64,
    pub moves_score_improving: u64,
    pub moves_applied_improving: u64,
    pub score_calculations: u64,
    pub generation_time: Duration,
    pub evaluation_time: Duration,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AppliedMoveTelemetry {
    pub step_index: u64,
    pub move_label: &'static str,
    pub selected_candidate_index: usize,
    pub moves_generated_this_step: u64,
    pub moves_evaluated_this_step: u64,
    pub moves_accepted_this_step: u64,
    pub moves_forager_ignored_this_step: u64,
    pub score_before: f64,
    pub score_after: f64,
    pub score_delta: f64,
    pub hard_feasible_before: bool,
    pub hard_feasible_after: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SolverTelemetry {
    pub elapsed: Duration,
    pub step_count: u64,
    /// Candidate moves actually yielded to the engine across all phases.
    /// Exhaust a cursor or query selector sizing separately for logical size.
    pub moves_generated: u64,
    pub moves_evaluated: u64,
    pub moves_accepted: u64,
    pub moves_applied: u64,
    pub moves_score_improving: u64,
    pub moves_applied_improving: u64,
    pub moves_not_doable: u64,
    pub moves_acceptor_rejected: u64,
    pub moves_forager_ignored: u64,
    pub moves_hard_improving: u64,
    pub moves_hard_neutral: u64,
    pub moves_hard_worse: u64,
    pub conflict_repair_provider_generated: u64,
    pub conflict_repair_duplicate_filtered: u64,
    pub conflict_repair_illegal_filtered: u64,
    pub conflict_repair_not_doable_filtered: u64,
    pub conflict_repair_hard_improving: u64,
    pub conflict_repair_exposed: u64,
    pub score_calculations: u64,
    pub construction_slots_assigned: u64,
    pub construction_slots_kept: u64,
    pub construction_slots_no_doable: u64,
    pub scalar_assignment_required_remaining: u64,
    pub generation_time: Duration,
    pub evaluation_time: Duration,
    pub phase: Option<PhaseTelemetry>,
    pub selector_telemetry: Vec<SelectorTelemetry>,
    pub move_telemetry: Vec<MoveTelemetry>,
    pub applied_move_trace: Vec<AppliedMoveTelemetry>,
    /// Present only when `SolverConfig.candidate_trace` enabled bounded
    /// core-owned candidate-pull diagnostics for this run.
    pub candidate_trace: Option<CandidateTraceTelemetry>,
}

impl SolverTelemetry {
    pub const fn new_const() -> Self {
        Self {
            elapsed: Duration::ZERO,
            step_count: 0,
            moves_generated: 0,
            moves_evaluated: 0,
            moves_accepted: 0,
            moves_applied: 0,
            moves_score_improving: 0,
            moves_applied_improving: 0,
            moves_not_doable: 0,
            moves_acceptor_rejected: 0,
            moves_forager_ignored: 0,
            moves_hard_improving: 0,
            moves_hard_neutral: 0,
            moves_hard_worse: 0,
            conflict_repair_provider_generated: 0,
            conflict_repair_duplicate_filtered: 0,
            conflict_repair_illegal_filtered: 0,
            conflict_repair_not_doable_filtered: 0,
            conflict_repair_hard_improving: 0,
            conflict_repair_exposed: 0,
            score_calculations: 0,
            construction_slots_assigned: 0,
            construction_slots_kept: 0,
            construction_slots_no_doable: 0,
            scalar_assignment_required_remaining: 0,
            generation_time: Duration::ZERO,
            evaluation_time: Duration::ZERO,
            phase: None,
            selector_telemetry: Vec::new(),
            move_telemetry: Vec::new(),
            applied_move_trace: Vec::new(),
            candidate_trace: None,
        }
    }

    /// Removes bounded candidate-pull diagnostic detail before ordinary
    /// lifecycle publication.
    ///
    /// Candidate traces can be intentionally large (up to the configured
    /// diagnostic ceiling). Progress events, retained status, and solution
    /// snapshots are all normal control-plane traffic, so they must never
    /// clone that detail. The retained manager keeps it in its dedicated
    /// detail store and exposes it only through an explicit accessor.
    pub(crate) fn take_candidate_trace(&mut self) -> Option<CandidateTraceTelemetry> {
        self.candidate_trace.take()
    }

    /// Splits bounded diagnostic detail from compact publication telemetry
    /// without cloning either payload.
    pub(crate) fn split_candidate_trace(mut self) -> (Self, Option<CandidateTraceTelemetry>) {
        let candidate_trace = self.take_candidate_trace();
        (self, candidate_trace)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Throughput {
    pub count: u64,
    pub elapsed: Duration,
}

pub(crate) fn whole_units_per_second(count: u64, elapsed: Duration) -> u128 {
    let nanos = elapsed.as_nanos();
    if nanos == 0 {
        0
    } else {
        u128::from(count)
            .saturating_mul(1_000_000_000)
            .checked_div(nanos)
            .unwrap_or(0)
    }
}

pub(crate) fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();

    if secs >= 60 {
        let mins = secs / 60;
        let rem_secs = secs % 60;
        return format!("{mins}m {rem_secs}s");
    }

    if secs > 0 {
        let millis = nanos / 1_000_000;
        if millis == 0 {
            return format!("{secs}s");
        }
        return format!("{secs}s {millis}ms");
    }

    let millis = nanos / 1_000_000;
    if millis > 0 {
        return format!("{millis}ms");
    }

    let micros = nanos / 1_000;
    if micros > 0 {
        return format!("{micros}us");
    }

    format!("{nanos}ns")
}
