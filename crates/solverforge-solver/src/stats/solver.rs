use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::{
    AppliedMoveTelemetry, CandidatePullTelemetry, CandidateTraceDisposition,
    CandidateTracePhasePlan, CandidateTracePullToken, CandidateTraceRecordDecision,
    CandidateTraceTelemetry, PhaseStats, PhaseTelemetry, SolverTelemetry, Throughput,
};

/* Solve-wide statistics, kept per phase.

Every counter is recorded exactly once, into the phase that is running, or
into the solve-wide accumulator when no phase is (the initial score
calculation, for example). A finished phase folds into that accumulator and
keeps a frozen snapshot, so run totals are always the accumulator plus the
running phase, and the per-phase records always sum to them.
*/
#[derive(Debug)]
pub struct SolverStats {
    start_time: Option<Instant>,
    pause_started_at: Option<Instant>,
    // Work outside any phase plus every finished phase, folded together.
    folded: PhaseStats,
    finished_phases: Vec<PhaseTelemetry>,
    current_phase: Option<RunningPhase>,
    scalar_assignment_required_remaining_by_group: BTreeMap<&'static str, u64>,
    candidate_trace: Option<CandidateTraceTelemetry>,
}

#[derive(Debug)]
struct RunningPhase {
    stats: PhaseStats,
    // Solver-active elapsed at phase entry, when the solve clock had started.
    started_at: Option<Duration>,
}

impl Default for SolverStats {
    fn default() -> Self {
        Self {
            start_time: None,
            pause_started_at: None,
            folded: PhaseStats::new(0, "Solve"),
            finished_phases: Vec::new(),
            current_phase: None,
            scalar_assignment_required_remaining_by_group: BTreeMap::new(),
            candidate_trace: None,
        }
    }
}

macro_rules! run_totals {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$doc])*
            pub fn $name(&self) -> u64 {
                self.folded.$name
                    + self
                        .current_phase
                        .as_ref()
                        .map_or(0, |phase| phase.stats.$name)
            }
        )*
    };
}

macro_rules! record_into_active {
    ($($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*)),* $(,)?) => {
        $(
            $(#[$doc])*
            pub fn $name(&mut self, $($arg: $ty),*) {
                self.active().$name($($arg),*);
            }
        )*
    };
}

impl SolverStats {
    /// Marks the start of solving.
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.pause_started_at = None;
    }

    /// Solver-active time since `start`, excluding pauses; zero before it.
    pub fn elapsed(&self) -> Duration {
        self.active_elapsed().unwrap_or_default()
    }

    /// Solver-active time since `start`, or `None` before the solve started.
    pub fn active_elapsed(&self) -> Option<Duration> {
        match (self.start_time, self.pause_started_at) {
            (Some(start), Some(paused_at)) => Some(paused_at.duration_since(start)),
            (Some(start), None) => Some(start.elapsed()),
            _ => None,
        }
    }

    /// The start instant, shifted forward by every finished pause.
    pub fn started_at(&self) -> Option<Instant> {
        self.start_time
    }

    pub fn is_paused(&self) -> bool {
        self.pause_started_at.is_some()
    }

    pub fn pause(&mut self) {
        if self.start_time.is_some() && self.pause_started_at.is_none() {
            self.pause_started_at = Some(Instant::now());
        }
    }

    pub fn resume(&mut self) {
        if let (Some(start), Some(paused_at)) = (self.start_time, self.pause_started_at.take()) {
            self.start_time = Some(start + paused_at.elapsed());
        }
    }

    /// Opens the next phase; counters recorded from now on belong to it. A
    /// phase still running is finished first.
    pub fn begin_phase(&mut self, phase_type: &'static str) {
        self.end_phase();
        let phase_index = self.finished_phases.len();
        self.current_phase = Some(RunningPhase {
            stats: PhaseStats::new(phase_index, phase_type),
            started_at: self.active_elapsed(),
        });
    }

    /// Freezes the running phase's record and folds it into the run totals.
    pub fn end_phase(&mut self) {
        if let Some(phase) = self.current_phase.take() {
            self.finished_phases.push(self.phase_snapshot(&phase));
            self.folded.absorb(&phase.stats);
        }
    }

    /// Solver-active time the running phase has spent, if a phase is running.
    pub fn phase_elapsed(&self) -> Option<Duration> {
        self.current_phase
            .as_ref()
            .map(|phase| self.running_elapsed(phase))
    }

    /// Statistics of the phase that is running, if any.
    pub fn phase(&self) -> Option<&PhaseStats> {
        self.current_phase.as_ref().map(|phase| &phase.stats)
    }

    pub fn phase_mut(&mut self) -> Option<&mut PhaseStats> {
        self.current_phase.as_mut().map(|phase| &mut phase.stats)
    }

    fn active(&mut self) -> &mut PhaseStats {
        match self.current_phase.as_mut() {
            Some(phase) => &mut phase.stats,
            None => &mut self.folded,
        }
    }

    fn running_elapsed(&self, phase: &RunningPhase) -> Duration {
        match (phase.started_at, self.active_elapsed()) {
            (Some(started_at), Some(now)) => now.saturating_sub(started_at),
            _ => phase.stats.elapsed(),
        }
    }

    fn phase_snapshot(&self, phase: &RunningPhase) -> PhaseTelemetry {
        phase
            .stats
            .snapshot_with_elapsed(self.running_elapsed(phase))
    }

    run_totals!(
        /// Steps taken across all phases.
        step_count,
        /// Candidate moves actually yielded by cursors across all phases.
        /// Unrequested logical neighborhood tails are not generated work.
        moves_generated,
        moves_evaluated,
        /// Candidates evaluated ahead of the sequential decision point by
        /// parallel evaluation and then discarded.
        moves_speculative,
        moves_accepted,
        moves_applied,
        moves_not_doable,
        moves_acceptor_rejected,
        moves_forager_ignored,
        moves_hard_improving,
        moves_hard_neutral,
        moves_hard_worse,
        conflict_repair_provider_generated,
        conflict_repair_duplicate_filtered,
        conflict_repair_illegal_filtered,
        conflict_repair_not_doable_filtered,
        conflict_repair_hard_improving,
        conflict_repair_exposed,
        score_calculations,
        construction_slots_assigned,
        construction_slots_kept,
        construction_slots_no_doable,
    );

    /// Required scalar-assignment rows still unassigned, summed over groups.
    pub fn scalar_assignment_required_remaining(&self) -> u64 {
        self.scalar_assignment_required_remaining_by_group
            .values()
            .copied()
            .sum()
    }

    record_into_active!(
        /// Records one or more generated candidate moves and the time spent generating them.
        record_generated_batch(count: u64, duration: Duration),
        record_selector_generated(selector_index: usize, count: u64, duration: Duration),
        /// Records generation time that did not itself yield a counted move.
        record_generation_time(duration: Duration),
        /// Records a single generated candidate move and the time spent generating it.
        record_generated_move(duration: Duration),
        /// Records a move evaluation and the time spent evaluating it.
        record_evaluated_move(duration: Duration),
        record_selector_evaluated(selector_index: usize, duration: Duration),
        record_move_accepted(),
        record_selector_accepted(selector_index: usize),
        record_move_applied(),
        record_selector_applied(selector_index: usize),
        record_move_not_doable(),
        record_selector_not_doable(selector_index: usize),
        record_move_acceptor_rejected(),
        record_selector_acceptor_rejected(selector_index: usize),
        record_moves_forager_ignored(count: u64),
        record_move_hard_improving(),
        record_move_hard_neutral(),
        record_move_hard_worse(),
        record_conflict_repair_provider_generated(count: u64),
        record_conflict_repair_duplicate_filtered(),
        record_conflict_repair_illegal_filtered(),
        record_conflict_repair_not_doable_filtered(),
        record_conflict_repair_hard_improving(),
        record_conflict_repair_exposed(),
        /// Records a step completion.
        record_step(),
        /// Records a score calculation.
        record_score_calculation(),
        /// Records a candidate that parallel evaluation evaluated ahead of the
        /// sequential decision point and then discarded.
        record_move_speculative(),
        record_construction_slot_assigned(),
        record_construction_slot_kept(),
        record_construction_slot_no_doable(),
        record_move_kind_generated(move_label: &'static str),
        record_move_kind_evaluated(move_label: &'static str, score_ordering: Ordering),
        record_move_kind_evaluated_unscored(move_label: &'static str),
        record_move_kind_accepted(move_label: &'static str),
        record_move_kind_applied(move_label: &'static str, score_improvement: f64),
        record_move_kind_not_doable(move_label: &'static str),
        record_move_kind_acceptor_rejected(move_label: &'static str, score_ordering: Ordering),
        record_move_kind_forager_ignored(move_label: &'static str, count: u64),
        record_applied_move_trace(applied_move: AppliedMoveTelemetry),
    );

    pub fn record_selector_generated_with_label(
        &mut self,
        selector_index: usize,
        selector_label: impl Into<String>,
        count: u64,
        duration: Duration,
    ) {
        self.active().record_selector_generated_with_label(
            selector_index,
            selector_label,
            count,
            duration,
        );
    }

    /// Records the required rows a scalar-assignment group still leaves
    /// unassigned. The run value sums the latest count of every group; the
    /// running phase keeps the latest count it saw.
    pub fn record_scalar_assignment_required_remaining(
        &mut self,
        group_name: &'static str,
        count: u64,
    ) {
        self.scalar_assignment_required_remaining_by_group
            .insert(group_name, count);
        self.active()
            .record_scalar_assignment_required_remaining(count);
    }

    pub fn generation_time(&self) -> Duration {
        self.folded.generation_time()
            + self
                .phase()
                .map_or(Duration::ZERO, PhaseStats::generation_time)
    }

    pub fn evaluation_time(&self) -> Duration {
        self.folded.evaluation_time()
            + self
                .phase()
                .map_or(Duration::ZERO, PhaseStats::evaluation_time)
    }

    pub fn generated_throughput(&self) -> Throughput {
        Throughput {
            count: self.moves_generated(),
            elapsed: self.generation_time(),
        }
    }

    pub fn evaluated_throughput(&self) -> Throughput {
        Throughput {
            count: self.moves_evaluated(),
            elapsed: self.evaluation_time(),
        }
    }

    pub fn acceptance_rate(&self) -> f64 {
        let evaluated = self.moves_evaluated();
        if evaluated == 0 {
            0.0
        } else {
            self.moves_accepted() as f64 / evaluated as f64
        }
    }

    pub fn snapshot(&self) -> SolverTelemetry {
        self.snapshot_with_applied_move_trace(true)
    }

    pub fn snapshot_without_applied_move_trace(&self) -> SolverTelemetry {
        self.snapshot_with_applied_move_trace(false)
    }

    fn snapshot_with_applied_move_trace(
        &self,
        include_applied_move_trace: bool,
    ) -> SolverTelemetry {
        let mut totals = self.folded.clone();
        let mut phases = self.finished_phases.clone();
        if let Some(phase) = &self.current_phase {
            totals.absorb(&phase.stats);
            phases.push(self.phase_snapshot(phase));
        }
        SolverTelemetry {
            elapsed: self.elapsed(),
            step_count: totals.step_count,
            moves_generated: totals.moves_generated,
            moves_evaluated: totals.moves_evaluated,
            moves_speculative: totals.moves_speculative,
            moves_accepted: totals.moves_accepted,
            moves_applied: totals.moves_applied,
            moves_score_improving: totals.moves_score_improving(),
            moves_applied_improving: totals.moves_applied_improving(),
            moves_not_doable: totals.moves_not_doable,
            moves_acceptor_rejected: totals.moves_acceptor_rejected,
            moves_forager_ignored: totals.moves_forager_ignored,
            moves_hard_improving: totals.moves_hard_improving,
            moves_hard_neutral: totals.moves_hard_neutral,
            moves_hard_worse: totals.moves_hard_worse,
            conflict_repair_provider_generated: totals.conflict_repair_provider_generated,
            conflict_repair_duplicate_filtered: totals.conflict_repair_duplicate_filtered,
            conflict_repair_illegal_filtered: totals.conflict_repair_illegal_filtered,
            conflict_repair_not_doable_filtered: totals.conflict_repair_not_doable_filtered,
            conflict_repair_hard_improving: totals.conflict_repair_hard_improving,
            conflict_repair_exposed: totals.conflict_repair_exposed,
            score_calculations: totals.score_calculations,
            construction_slots_assigned: totals.construction_slots_assigned,
            construction_slots_kept: totals.construction_slots_kept,
            construction_slots_no_doable: totals.construction_slots_no_doable,
            scalar_assignment_required_remaining: self.scalar_assignment_required_remaining(),
            generation_time: totals.generation_time(),
            evaluation_time: totals.evaluation_time(),
            phases,
            selector_telemetry: totals.selector_telemetry().to_vec(),
            move_telemetry: totals.move_telemetry(),
            applied_move_trace: if include_applied_move_trace {
                totals.applied_move_trace().to_vec()
            } else {
                Vec::new()
            },
            candidate_trace: if include_applied_move_trace {
                self.candidate_trace
                    .as_ref()
                    .map(CandidateTraceTelemetry::snapshot)
            } else {
                None
            },
        }
    }

    pub fn moves_score_improving(&self) -> u64 {
        self.folded.moves_score_improving()
            + self.phase().map_or(0, PhaseStats::moves_score_improving)
    }

    pub fn moves_applied_improving(&self) -> u64 {
        self.folded.moves_applied_improving()
            + self.phase().map_or(0, PhaseStats::moves_applied_improving)
    }

    pub(crate) fn enable_candidate_trace(&mut self, candidate_trace: CandidateTraceTelemetry) {
        assert!(
            self.candidate_trace.is_none(),
            "candidate trace may be enabled only once per solve"
        );
        self.candidate_trace = Some(candidate_trace);
    }

    pub(crate) fn prepare_candidate_trace_pull(&mut self) -> CandidateTraceRecordDecision {
        match self.candidate_trace.as_mut() {
            Some(trace) => trace.prepare_pull(),
            None => CandidateTraceRecordDecision::Disabled,
        }
    }

    pub(crate) fn begin_candidate_trace_phase(&mut self) -> Option<usize> {
        self.candidate_trace
            .as_mut()
            .map(CandidateTraceTelemetry::begin_phase)
    }

    /// Finalizes the exact phase plan for an enabled candidate trace.
    ///
    /// A trace-free solve intentionally has no recorder to update. The
    /// one-way invariant itself is enforced by `CandidateTraceTelemetry` when
    /// a trace exists.
    pub(crate) fn finalize_candidate_trace_resolved_phase_plan(
        &mut self,
        resolved_phase_plan: CandidateTracePhasePlan,
    ) {
        if let Some(trace) = self.candidate_trace.as_mut() {
            trace.finalize_resolved_phase_plan(resolved_phase_plan);
        }
    }

    pub(crate) fn record_prepared_candidate_trace_pull(
        &mut self,
        pull: CandidatePullTelemetry,
    ) -> CandidateTracePullToken {
        let trace = self
            .candidate_trace
            .as_mut()
            .expect("a prepared candidate trace pull requires an enabled trace");
        trace.push_prepared(pull)
    }

    pub(crate) fn record_candidate_trace_disposition(
        &mut self,
        token: CandidateTracePullToken,
        disposition: CandidateTraceDisposition,
    ) {
        let trace = self
            .candidate_trace
            .as_mut()
            .expect("a candidate trace disposition requires an enabled trace");
        trace.record_disposition(token, disposition);
    }
}

/* Phase-level statistics.

Tracks metrics for a single solver phase.

# Example

```
use solverforge_solver::stats::PhaseStats;
use std::time::Duration;

let mut stats = PhaseStats::new(0, "LocalSearch");
stats.record_step();
stats.record_generated_move(Duration::from_millis(1));
stats.record_evaluated_move(Duration::from_millis(2));
stats.record_move_accepted();

assert_eq!(stats.phase_index, 0);
assert_eq!(stats.phase_type, "LocalSearch");
assert_eq!(stats.step_count, 1);
assert_eq!(stats.moves_accepted, 1);
```
*/
