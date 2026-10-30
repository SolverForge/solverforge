//! Solver statistics (zero-erasure).
//!
//! Stack-allocated statistics for solver and phase performance tracking.

use std::time::Duration;

use super::CandidateTraceTelemetry;

/// Counters for one move selector of a phase, keyed by its index.
///
/// Entries appear in [`SolverTelemetry::selector_telemetry`] in first-seen
/// selector order, summed across phases.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SelectorTelemetry {
    /// Position of the selector in the phase's selector list; the key of this entry.
    pub selector_index: usize,
    /// Display label of the selector. It reads `selector-<index>` until the engine
    /// records a descriptive label for the same index.
    pub selector_label: String,
    /// Candidate moves actually yielded by this selector.
    /// This is runtime work, not the size of an unconsumed neighborhood.
    pub moves_generated: u64,
    /// Candidate moves from this selector that reached evaluation.
    pub moves_evaluated: u64,
    /// Evaluated moves from this selector that the acceptor accepted.
    pub moves_accepted: u64,
    /// Moves from this selector that were committed to the working solution.
    pub moves_applied: u64,
    /// Moves from this selector rejected as not doable (or touching pinned
    /// entities) before scoring.
    pub moves_not_doable: u64,
    /// Scored moves from this selector that the acceptor rejected.
    pub moves_acceptor_rejected: u64,
    /// Accepted moves from this selector that lost the forager's pick.
    pub moves_forager_ignored: u64,
    /// Scored moves from this selector whose hard score beat the step's
    /// reference score.
    pub moves_hard_improving: u64,
    /// Scored moves from this selector whose hard score equalled the reference.
    pub moves_hard_neutral: u64,
    /// Scored moves from this selector whose hard score fell below the reference.
    pub moves_hard_worse: u64,
    /// Conflict-repair candidates the provider generated. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_provider_generated: u64,
    /// Conflict-repair candidates dropped as duplicates. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_duplicate_filtered: u64,
    /// Conflict-repair candidates dropped as illegal. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_illegal_filtered: u64,
    /// Conflict-repair candidates dropped as not doable. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_not_doable_filtered: u64,
    /// Conflict-repair candidates that improved the hard score. No shipped engine
    /// path records this counter yet, so it stays 0.
    pub conflict_repair_hard_improving: u64,
    /// Conflict-repair candidates exposed to the search. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_exposed: u64,
    /// Time this selector spent generating its moves.
    pub generation_time: Duration,
    /// Time spent evaluating this selector's moves.
    pub evaluation_time: Duration,
}

/// Counters for one move kind, keyed by the move's telemetry label.
///
/// Entries are kept sorted by label and summed across phases in
/// [`SolverTelemetry::move_telemetry`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MoveTelemetry {
    /// Telemetry label of the move kind (for example `change` or `swap`).
    pub move_label: String,
    /// Candidate moves actually yielded to the engine across all phases.
    /// Exhaust a cursor or query selector sizing separately for logical size.
    pub moves_generated: u64,
    /// Moves of this kind that reached evaluation, scored or not.
    pub moves_evaluated: u64,
    /// Evaluated moves of this kind that the acceptor accepted.
    pub moves_accepted: u64,
    /// Moves of this kind committed to the working solution.
    pub moves_applied: u64,
    /// Moves of this kind rejected as not doable before scoring.
    pub moves_not_doable: u64,
    /// Scored moves of this kind that the acceptor rejected.
    pub moves_acceptor_rejected: u64,
    /// Accepted moves of this kind that lost the forager's pick.
    pub moves_forager_ignored: u64,
    /// Scored moves of this kind whose score beat the step's reference score.
    pub moves_score_improving: u64,
    /// Applied moves of this kind that improved a feasible score.
    pub moves_applied_improving: u64,
    /// Scored moves of this kind whose score equalled the reference score.
    pub moves_score_equal: u64,
    /// Scored moves of this kind whose score fell below the reference score.
    pub moves_score_worse: u64,
    /// Moves of this kind the acceptor rejected although their score improved.
    pub moves_rejected_improving: u64,
    /// Sum of the scalar score gains (`Score::to_scalar` difference) of the
    /// applied improving moves of this kind; moves applied from an infeasible
    /// score add nothing.
    pub applied_score_improvement: f64,
}

/// Counters for one phase of a solve, as published in
/// [`SolverTelemetry::phases`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhaseTelemetry {
    /// Zero-based position of this phase in the solve's execution order.
    pub phase_index: usize,
    /// Phase type name, for example `Construction Heuristic` or `Local Search`.
    pub phase_type: String,
    /// Solver-active time the phase ran, excluding pauses; for the running phase,
    /// the time so far.
    pub elapsed: Duration,
    /// Steps the phase completed.
    pub step_count: u64,
    /// Candidate moves actually yielded to the engine during this phase.
    /// This is runtime work, not the size of an unconsumed neighborhood.
    pub moves_generated: u64,
    /// Candidate moves that reached evaluation during this phase.
    pub moves_evaluated: u64,
    /// Candidates this phase's parallel evaluation evaluated ahead of the
    /// sequential decision point and then discarded.
    pub moves_speculative: u64,
    /// Evaluated moves the phase accepted.
    pub moves_accepted: u64,
    /// Moves the phase committed to the working solution.
    pub moves_applied: u64,
    /// Scored moves whose score beat the step's reference score.
    pub moves_score_improving: u64,
    /// Applied moves that improved a feasible score.
    pub moves_applied_improving: u64,
    /// Score calculations the phase performed.
    pub score_calculations: u64,
    /// Time the phase spent generating candidate moves.
    pub generation_time: Duration,
    /// Time the phase spent evaluating candidate moves.
    pub evaluation_time: Duration,
}

/// One committed local-search step, kept in a bounded trace.
///
/// The trace keeps the first eight applied moves of a solve, in step order.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AppliedMoveTelemetry {
    /// Phase-local index of the step that applied the move.
    pub step_index: u64,
    /// Telemetry label of the applied move kind.
    pub move_label: &'static str,
    /// Index of the selected candidate in the step's candidate stream.
    pub selected_candidate_index: usize,
    /// Candidates the step generated before it chose.
    pub moves_generated_this_step: u64,
    /// Candidates the step evaluated before it chose.
    pub moves_evaluated_this_step: u64,
    /// Candidates the step accepted, the selected one included.
    pub moves_accepted_this_step: u64,
    /// Accepted candidates the forager did not pick (accepted minus one).
    pub moves_forager_ignored_this_step: u64,
    /// Step score before the move, as `Score::to_scalar`.
    pub score_before: f64,
    /// Step score after the move, as `Score::to_scalar`.
    pub score_after: f64,
    /// `score_after - score_before`.
    pub score_delta: f64,
    /// Whether the score before the move was feasible.
    pub hard_feasible_before: bool,
    /// Whether the score after the move is feasible.
    pub hard_feasible_after: bool,
}

/// Solve-wide counters published with progress events, snapshots and the
/// final result.
///
/// Run totals are the sum of every phase plus work done outside phases.
/// Progress events and snapshots carry no `candidate_trace`; the solver
/// manager keeps that detail separately.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SolverTelemetry {
    /// Solver-active time since the solve started, excluding pauses.
    pub elapsed: Duration,
    /// Steps completed across all phases.
    pub step_count: u64,
    /// Candidate moves actually yielded to the engine across all phases.
    /// Exhaust a cursor or query selector sizing separately for logical size.
    pub moves_generated: u64,
    /// Candidate moves that reached evaluation across all phases.
    pub moves_evaluated: u64,
    /// Candidates evaluated ahead of the sequential decision point by parallel
    /// evaluation and then discarded. Zero for sequential evaluation.
    pub moves_speculative: u64,
    /// Evaluated moves accepted across all phases.
    pub moves_accepted: u64,
    /// Moves committed to the working solution across all phases.
    pub moves_applied: u64,
    /// Scored moves whose score beat the step's reference score.
    pub moves_score_improving: u64,
    /// Applied moves that improved a feasible score.
    pub moves_applied_improving: u64,
    /// Moves rejected as not doable (or touching pinned entities) before scoring.
    pub moves_not_doable: u64,
    /// Scored moves the acceptor rejected.
    pub moves_acceptor_rejected: u64,
    /// Accepted moves that lost the forager's pick.
    pub moves_forager_ignored: u64,
    /// Scored local-search moves whose hard score beat the reference score.
    pub moves_hard_improving: u64,
    /// Scored local-search moves whose hard score equalled the reference score.
    pub moves_hard_neutral: u64,
    /// Scored local-search moves whose hard score fell below the reference score.
    pub moves_hard_worse: u64,
    /// Conflict-repair candidates the provider generated. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_provider_generated: u64,
    /// Conflict-repair candidates dropped as duplicates. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_duplicate_filtered: u64,
    /// Conflict-repair candidates dropped as illegal. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_illegal_filtered: u64,
    /// Conflict-repair candidates dropped as not doable. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_not_doable_filtered: u64,
    /// Conflict-repair candidates that improved the hard score. No shipped engine
    /// path records this counter yet, so it stays 0.
    pub conflict_repair_hard_improving: u64,
    /// Conflict-repair candidates exposed to the search. No shipped engine path
    /// records this counter yet, so it stays 0.
    pub conflict_repair_exposed: u64,
    /// Score calculations across the solve, the initial one included.
    pub score_calculations: u64,
    /// Construction slots filled by an applied move.
    pub construction_slots_assigned: u64,
    /// Construction slots whose current value was legally kept.
    pub construction_slots_kept: u64,
    /// Construction slots completed without a doable candidate to assign.
    pub construction_slots_no_doable: u64,
    /// Required scalar-assignment rows still unassigned, summed over the latest
    /// count of each assignment group.
    pub scalar_assignment_required_remaining: u64,
    /// Real-time problem changes applied so far. A change acknowledged as
    /// `ProblemChangeId(n)` has been applied once this reaches `n`.
    pub problem_changes_applied: u64,
    /// Time spent generating candidate moves across all phases.
    pub generation_time: Duration,
    /// Time spent evaluating candidate moves across all phases.
    pub evaluation_time: Duration,
    /// Every phase started so far, in execution order; the last entry of a
    /// progress snapshot is the phase still running. Run totals above equal
    /// the sum of these phases plus work done outside any phase, such as the
    /// initial score calculation.
    pub phases: Vec<PhaseTelemetry>,
    /// Per-selector counters summed over phases, in first-seen selector order.
    pub selector_telemetry: Vec<SelectorTelemetry>,
    /// Per-move-kind counters summed over phases, sorted by move label.
    pub move_telemetry: Vec<MoveTelemetry>,
    /// First applied local-search moves of the solve (at most eight); empty in
    /// progress snapshots taken without the trace.
    pub applied_move_trace: Vec<AppliedMoveTelemetry>,
    /// Present only when `SolverConfig.candidate_trace` enabled bounded
    /// core-owned candidate-pull diagnostics for this run.
    pub candidate_trace: Option<CandidateTraceTelemetry>,
}

impl SolverTelemetry {
    /// Returns telemetry with every counter at zero and empty lists, usable in
    /// `const` contexts.
    pub const fn new_const() -> Self {
        Self {
            elapsed: Duration::ZERO,
            step_count: 0,
            moves_generated: 0,
            moves_evaluated: 0,
            moves_speculative: 0,
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
            problem_changes_applied: 0,
            generation_time: Duration::ZERO,
            evaluation_time: Duration::ZERO,
            phases: Vec::new(),
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

/// A count of work items and the time spent on them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Throughput {
    /// Number of items (moves generated or evaluated).
    pub count: u64,
    /// Time spent on those items.
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
