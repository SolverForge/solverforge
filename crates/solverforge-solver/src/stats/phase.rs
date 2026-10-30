use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::{AppliedMoveTelemetry, MoveTelemetry, SelectorTelemetry};

const APPLIED_MOVE_TRACE_LIMIT: usize = 8;

#[path = "phase_absorb.rs"]
mod absorb;
#[path = "phase_counters.rs"]
mod counters;

/// Phase-level statistics.
///
/// Tracks metrics for a single solver phase.
///
/// # Example
///
/// ```
/// use solverforge_solver::PhaseStats;
/// use std::time::Duration;
///
/// let mut stats = PhaseStats::new(0, "LocalSearch");
/// stats.record_step();
/// stats.record_generated_move(Duration::from_millis(1));
/// stats.record_evaluated_move(Duration::from_millis(2));
/// stats.record_move_accepted();
///
/// assert_eq!(stats.phase_index, 0);
/// assert_eq!(stats.phase_type, "LocalSearch");
/// assert_eq!(stats.step_count, 1);
/// assert_eq!(stats.moves_accepted, 1);
/// ```
#[derive(Debug, Clone)]
pub struct PhaseStats {
    /// Index of this phase (0-based).
    pub phase_index: usize,
    /// Type name of the phase.
    pub phase_type: &'static str,
    start_time: Instant,
    /// Number of steps taken in this phase.
    pub step_count: u64,
    /// Number of candidate moves actually yielded by cursors in this phase.
    /// Unrequested logical neighborhood tails are not generated work.
    pub moves_generated: u64,
    /// Number of moves evaluated in this phase.
    pub moves_evaluated: u64,
    /// Candidates parallel evaluation evaluated ahead of the sequential
    /// decision point in this phase and then discarded.
    pub moves_speculative: u64,
    /// Number of moves accepted in this phase.
    pub moves_accepted: u64,
    /// Number of moves applied in this phase.
    pub moves_applied: u64,
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
    /// Raw candidates conflict-repair providers returned, before filtering
    /// and before the per-match and per-step caps.
    pub conflict_repair_provider_generated: u64,
    /// Conflict-repair candidates dropped as exact repeats (same reason and
    /// resolved edits) of an earlier candidate in the same provider pull.
    pub conflict_repair_duplicate_filtered: u64,
    /// Conflict-repair candidates dropped as structurally illegal: no edits,
    /// or two edits on the same entity variable. An edit with an unknown slot
    /// or an illegal value fails the solve instead of being dropped.
    pub conflict_repair_illegal_filtered: u64,
    /// Conflict-repair candidates dropped as not doable on the working
    /// solution, such as a repair whose edits change no value.
    pub conflict_repair_not_doable_filtered: u64,
    /// Evaluated conflict-repair candidates whose hard score beat the step's
    /// reference score.
    pub conflict_repair_hard_improving: u64,
    /// Conflict-repair candidates handed on for evaluation, directly or to an
    /// enclosing cartesian composition.
    pub conflict_repair_exposed: u64,
    /// Number of score calculations in this phase.
    pub score_calculations: u64,
    /// Construction slots filled by an applied move.
    pub construction_slots_assigned: u64,
    /// Construction slots whose current value was legally kept.
    pub construction_slots_kept: u64,
    /// Construction slots completed without a doable candidate to assign.
    pub construction_slots_no_doable: u64,
    /// Latest count of required scalar-assignment rows still unassigned. A
    /// level, not a sum: each record overwrites it.
    pub scalar_assignment_required_remaining: u64,
    generation_time: Duration,
    evaluation_time: Duration,
    selector_stats: Vec<SelectorTelemetry>,
    move_stats: BTreeMap<&'static str, MoveTelemetry>,
    applied_move_trace: Vec<AppliedMoveTelemetry>,
}

impl PhaseStats {
    /// Records one generated move of kind `move_label`.
    pub fn record_move_kind_generated(&mut self, move_label: &'static str) {
        self.move_stats_entry(move_label).moves_generated += 1;
    }

    /// Records one scored move of kind `move_label`, classified by how its
    /// score compares with the reference (`Greater` means improving).
    pub fn record_move_kind_evaluated(
        &mut self,
        move_label: &'static str,
        score_ordering: Ordering,
    ) {
        let entry = self.move_stats_entry(move_label);
        entry.moves_evaluated += 1;
        match score_ordering {
            Ordering::Greater => entry.moves_score_improving += 1,
            Ordering::Equal => entry.moves_score_equal += 1,
            Ordering::Less => entry.moves_score_worse += 1,
        }
    }

    /// Records one evaluated move of kind `move_label` that was never scored
    /// (for example because it was not doable).
    pub fn record_move_kind_evaluated_unscored(&mut self, move_label: &'static str) {
        self.move_stats_entry(move_label).moves_evaluated += 1;
    }

    /// Records one accepted move of kind `move_label`.
    pub fn record_move_kind_accepted(&mut self, move_label: &'static str) {
        self.move_stats_entry(move_label).moves_accepted += 1;
    }

    /// Records one applied move of kind `move_label`. A positive
    /// `score_improvement` also counts the move as improving and adds the gain.
    pub fn record_move_kind_applied(&mut self, move_label: &'static str, score_improvement: f64) {
        let entry = self.move_stats_entry(move_label);
        entry.moves_applied += 1;
        if score_improvement > 0.0 {
            entry.moves_applied_improving += 1;
            entry.applied_score_improvement += score_improvement;
        }
    }

    /// Records one not-doable move of kind `move_label`.
    pub fn record_move_kind_not_doable(&mut self, move_label: &'static str) {
        self.move_stats_entry(move_label).moves_not_doable += 1;
    }

    /// Records one acceptor-rejected move of kind `move_label`; a `Greater`
    /// `score_ordering` also counts it as a rejected improving move.
    pub fn record_move_kind_acceptor_rejected(
        &mut self,
        move_label: &'static str,
        score_ordering: Ordering,
    ) {
        let entry = self.move_stats_entry(move_label);
        entry.moves_acceptor_rejected += 1;
        if score_ordering == Ordering::Greater {
            entry.moves_rejected_improving += 1;
        }
    }

    /// Records `count` forager-ignored moves of kind `move_label`; zero is a no-op.
    pub fn record_move_kind_forager_ignored(&mut self, move_label: &'static str, count: u64) {
        if count == 0 {
            return;
        }
        self.move_stats_entry(move_label).moves_forager_ignored += count;
    }

    /// Appends an applied move to the trace; dropped once the trace holds eight
    /// entries.
    pub fn record_applied_move_trace(&mut self, applied_move: AppliedMoveTelemetry) {
        if self.applied_move_trace.len() < APPLIED_MOVE_TRACE_LIMIT {
            self.applied_move_trace.push(applied_move);
        }
    }

    /// Records `count` moves generated by selector `selector_index` with their
    /// generation time, and labels the selector `selector_label` unless it
    /// already carries a descriptive label.
    pub fn record_selector_generated_with_label(
        &mut self,
        selector_index: usize,
        selector_label: impl Into<String>,
        count: u64,
        duration: Duration,
    ) {
        self.record_generated_batch(count, duration);
        let selector = self.selector_stats_entry_with_label(selector_index, selector_label);
        selector.moves_generated += count;
        selector.generation_time += duration;
    }

    fn selector_stats_entry(&mut self, selector_index: usize) -> &mut SelectorTelemetry {
        self.selector_stats_entry_with_label(selector_index, format!("selector-{selector_index}"))
    }

    fn selector_stats_entry_with_label(
        &mut self,
        selector_index: usize,
        selector_label: impl Into<String>,
    ) -> &mut SelectorTelemetry {
        let selector_label = selector_label.into();
        if let Some(position) = self
            .selector_stats
            .iter()
            .position(|entry| entry.selector_index == selector_index)
        {
            if self.selector_stats[position]
                .selector_label
                .starts_with("selector-")
                && !selector_label.starts_with("selector-")
            {
                self.selector_stats[position].selector_label = selector_label;
            }
            return &mut self.selector_stats[position];
        }
        self.selector_stats.push(SelectorTelemetry {
            selector_index,
            selector_label,
            ..SelectorTelemetry::default()
        });
        self.selector_stats
            .last_mut()
            .expect("selector stats entry was just inserted")
    }

    fn move_stats_entry(&mut self, move_label: &'static str) -> &mut MoveTelemetry {
        self.move_stats
            .entry(move_label)
            .or_insert_with(|| MoveTelemetry {
                move_label: move_label.to_string(),
                ..MoveTelemetry::default()
            })
    }
}
