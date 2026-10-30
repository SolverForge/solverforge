use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use smallvec::SmallVec;

use super::{AppliedMoveTelemetry, CandidateTraceTelemetry, PhaseStats, Throughput};

mod candidate_trace;
mod snapshot;

/// Solve-wide statistics, kept per phase.
///
/// Every counter is recorded exactly once, into the phase that is running, or
/// into the solve-wide accumulator when no phase is (the initial score
/// calculation, for example). A finished phase folds into that accumulator and
/// moves into the finished list, so run totals are always the accumulator plus
/// the running phase, and the per-phase records always sum to them. Finished
/// phases stay `PhaseStats` until a snapshot asks for them, and the list keeps a
/// typical solve's phases inline, so closing a phase never allocates.
///
/// # Example
///
/// ```
/// use solverforge_solver::SolverStats;
/// use std::time::Duration;
///
/// let mut stats = SolverStats::default();
/// stats.start();
/// stats.begin_phase("LocalSearch");
/// stats.record_step();
/// stats.record_generated_move(Duration::from_millis(1));
/// stats.record_evaluated_move(Duration::from_millis(2));
/// stats.record_move_accepted();
/// stats.record_generated_move(Duration::from_millis(1));
/// stats.record_evaluated_move(Duration::from_millis(2));
/// stats.end_phase();
///
/// assert_eq!(stats.step_count(), 1);
/// assert_eq!(stats.moves_evaluated(), 2);
/// assert_eq!(stats.moves_accepted(), 1);
/// assert_eq!(stats.snapshot().phases[0].moves_evaluated, 2);
/// ```
#[derive(Debug)]
pub struct SolverStats {
    start_time: Option<Instant>,
    pause_started_at: Option<Instant>,
    // Work outside any phase plus every finished phase, folded together.
    folded: PhaseStats,
    finished_phases: SmallVec<[FinishedPhase; INLINE_PHASES]>,
    current_phase: Option<RunningPhase>,
    scalar_assignment_required_remaining_by_group: BTreeMap<&'static str, u64>,
    problem_changes_applied: u64,
    candidate_trace: Option<CandidateTraceTelemetry>,
}

/// Phases a solve keeps without allocating: construction, local search, and
/// room for a construction or improvement phase on each side.
const INLINE_PHASES: usize = 4;

#[derive(Debug)]
struct FinishedPhase {
    stats: PhaseStats,
    elapsed: Duration,
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
            finished_phases: SmallVec::new(),
            current_phase: None,
            scalar_assignment_required_remaining_by_group: BTreeMap::new(),
            problem_changes_applied: 0,
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

    /// Whether the solve clock is paused.
    pub fn is_paused(&self) -> bool {
        self.pause_started_at.is_some()
    }

    /// Stops the solve clock; a no-op before `start` or while already paused.
    pub fn pause(&mut self) {
        if self.start_time.is_some() && self.pause_started_at.is_none() {
            self.pause_started_at = Some(Instant::now());
        }
    }

    /// Restarts the solve clock, shifting the start instant forward by the pause
    /// so elapsed time excludes it; a no-op when not paused.
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
            let elapsed = self.running_elapsed(&phase);
            self.folded.absorb(&phase.stats);
            self.finished_phases.push(FinishedPhase {
                stats: phase.stats,
                elapsed,
            });
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

    /// Mutable statistics of the phase that is running, if any.
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

    run_totals!(
        /// Steps taken across all phases.
        step_count,
        /// Candidate moves actually yielded by cursors across all phases.
        /// Unrequested logical neighborhood tails are not generated work.
        moves_generated,
        /// Candidate moves that reached evaluation across all phases.
        moves_evaluated,
        /// Candidates evaluated ahead of the sequential decision point by
        /// parallel evaluation and then discarded.
        moves_speculative,
        /// Evaluated moves accepted across all phases.
        moves_accepted,
        /// Moves committed to the working solution across all phases.
        moves_applied,
        /// Moves rejected as not doable (or touching pinned entities) before
        /// scoring.
        moves_not_doable,
        /// Scored moves the acceptor rejected.
        moves_acceptor_rejected,
        /// Accepted moves that lost the forager's pick.
        moves_forager_ignored,
        /// Scored local-search moves whose hard score beat the reference score.
        moves_hard_improving,
        /// Scored local-search moves whose hard score equalled the reference.
        moves_hard_neutral,
        /// Scored local-search moves whose hard score fell below the reference.
        moves_hard_worse,
        /// Conflict-repair candidates the provider generated. No shipped engine
        /// path records it yet, so it stays 0.
        conflict_repair_provider_generated,
        /// Conflict-repair candidates dropped as duplicates. No shipped engine
        /// path records it yet, so it stays 0.
        conflict_repair_duplicate_filtered,
        /// Conflict-repair candidates dropped as illegal. No shipped engine
        /// path records it yet, so it stays 0.
        conflict_repair_illegal_filtered,
        /// Conflict-repair candidates dropped as not doable. No shipped engine
        /// path records it yet, so it stays 0.
        conflict_repair_not_doable_filtered,
        /// Conflict-repair candidates that improved the hard score. No shipped
        /// engine path records it yet, so it stays 0.
        conflict_repair_hard_improving,
        /// Conflict-repair candidates exposed to the search. No shipped engine
        /// path records it yet, so it stays 0.
        conflict_repair_exposed,
        /// Score calculations across the solve, the initial one included.
        score_calculations,
        /// Construction slots filled by an applied move.
        construction_slots_assigned,
        /// Construction slots whose current value was legally kept.
        construction_slots_kept,
        /// Construction slots completed without a doable candidate.
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
        /// Records `count` moves generated by selector `selector_index` and their
        /// generation time.
        record_selector_generated(selector_index: usize, count: u64, duration: Duration),
        /// Records generation time that did not itself yield a counted move.
        record_generation_time(duration: Duration),
        /// Records a single generated candidate move and the time spent generating it.
        record_generated_move(duration: Duration),
        /// Records a move evaluation and the time spent evaluating it.
        record_evaluated_move(duration: Duration),
        /// Records one move evaluated from selector `selector_index` and its
        /// evaluation time.
        record_selector_evaluated(selector_index: usize, duration: Duration),
        /// Records an accepted move.
        record_move_accepted(),
        /// Records one accepted move from selector `selector_index`.
        record_selector_accepted(selector_index: usize),
        /// Records a move committed to the working solution.
        record_move_applied(),
        /// Records one committed move from selector `selector_index`.
        record_selector_applied(selector_index: usize),
        /// Records a move rejected as not doable before scoring.
        record_move_not_doable(),
        /// Records one not-doable move from selector `selector_index`.
        record_selector_not_doable(selector_index: usize),
        /// Records a scored move the acceptor rejected.
        record_move_acceptor_rejected(),
        /// Records one acceptor-rejected move from selector `selector_index`.
        record_selector_acceptor_rejected(selector_index: usize),
        /// Records `count` accepted moves the forager did not pick.
        record_moves_forager_ignored(count: u64),
        /// Records a scored move whose hard score beat the reference score.
        record_move_hard_improving(),
        /// Records a scored move whose hard score equalled the reference score.
        record_move_hard_neutral(),
        /// Records a scored move whose hard score fell below the reference score.
        record_move_hard_worse(),
        /// Adds `count` to the conflict-repair generated counter.
        record_conflict_repair_provider_generated(count: u64),
        /// Increments the conflict-repair duplicate-filtered counter.
        record_conflict_repair_duplicate_filtered(),
        /// Increments the conflict-repair illegal-filtered counter.
        record_conflict_repair_illegal_filtered(),
        /// Increments the conflict-repair not-doable-filtered counter.
        record_conflict_repair_not_doable_filtered(),
        /// Increments the conflict-repair hard-improving counter.
        record_conflict_repair_hard_improving(),
        /// Increments the conflict-repair exposed counter.
        record_conflict_repair_exposed(),
        /// Records a step completion.
        record_step(),
        /// Records a score calculation.
        record_score_calculation(),
        /// Records a candidate that parallel evaluation evaluated ahead of the
        /// sequential decision point and then discarded.
        record_move_speculative(),
        /// Records a construction slot filled by an applied move.
        record_construction_slot_assigned(),
        /// Records a construction slot whose current value was legally kept.
        record_construction_slot_kept(),
        /// Records a construction slot completed without a doable candidate.
        record_construction_slot_no_doable(),
        /// Records one generated move of kind `move_label`.
        record_move_kind_generated(move_label: &'static str),
        /// Records one scored move of kind `move_label`, classified by how its
        /// score compares with the reference (`Greater` means improving).
        record_move_kind_evaluated(move_label: &'static str, score_ordering: Ordering),
        /// Records one evaluated move of kind `move_label` that was never scored.
        record_move_kind_evaluated_unscored(move_label: &'static str),
        /// Records one accepted move of kind `move_label`.
        record_move_kind_accepted(move_label: &'static str),
        /// Records one applied move of kind `move_label`; a positive
        /// `score_improvement` also counts it as improving and adds the gain.
        record_move_kind_applied(move_label: &'static str, score_improvement: f64),
        /// Records one not-doable move of kind `move_label`.
        record_move_kind_not_doable(move_label: &'static str),
        /// Records one acceptor-rejected move of kind `move_label`; a `Greater`
        /// `score_ordering` also counts it as a rejected improving move.
        record_move_kind_acceptor_rejected(move_label: &'static str, score_ordering: Ordering),
        /// Records `count` forager-ignored moves of kind `move_label`.
        record_move_kind_forager_ignored(move_label: &'static str, count: u64),
        /// Appends an applied move to the trace; dropped once it holds eight
        /// entries.
        record_applied_move_trace(applied_move: AppliedMoveTelemetry),
    );

    /// Records `count` moves generated by selector `selector_index` in the
    /// running phase (or the solve-wide accumulator outside phases) and labels
    /// the selector `selector_label` unless it already carries a descriptive
    /// label.
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

    /// Time spent generating moves across the solve.
    pub fn generation_time(&self) -> Duration {
        self.folded.generation_time()
            + self
                .phase()
                .map_or(Duration::ZERO, PhaseStats::generation_time)
    }

    /// Time spent evaluating moves across the solve.
    pub fn evaluation_time(&self) -> Duration {
        self.folded.evaluation_time()
            + self
                .phase()
                .map_or(Duration::ZERO, PhaseStats::evaluation_time)
    }

    /// Moves generated across the solve and the time spent generating them.
    pub fn generated_throughput(&self) -> Throughput {
        Throughput {
            count: self.moves_generated(),
            elapsed: self.generation_time(),
        }
    }

    /// Moves evaluated across the solve and the time spent evaluating them.
    pub fn evaluated_throughput(&self) -> Throughput {
        Throughput {
            count: self.moves_evaluated(),
            elapsed: self.evaluation_time(),
        }
    }

    /// Accepted moves divided by evaluated moves across the solve; `0.0`
    /// before any evaluation.
    pub fn acceptance_rate(&self) -> f64 {
        let evaluated = self.moves_evaluated();
        if evaluated == 0 {
            0.0
        } else {
            self.moves_accepted() as f64 / evaluated as f64
        }
    }

    /// Problem changes applied to this solve so far, across every batch.
    pub fn problem_changes_applied(&self) -> u64 {
        self.problem_changes_applied
    }

    /// Adds `count` applied problem changes to the solve's total.
    pub fn record_problem_changes_applied(&mut self, count: u64) {
        self.problem_changes_applied += count;
    }

    /// Scored moves whose score beat the reference score, across the solve.
    pub fn moves_score_improving(&self) -> u64 {
        self.folded.moves_score_improving()
            + self.phase().map_or(0, PhaseStats::moves_score_improving)
    }

    /// Applied moves that improved a feasible score, across the solve.
    pub fn moves_applied_improving(&self) -> u64 {
        self.folded.moves_applied_improving()
            + self.phase().map_or(0, PhaseStats::moves_applied_improving)
    }
}
