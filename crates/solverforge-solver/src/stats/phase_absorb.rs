//! Folding one phase's statistics into an accumulator.
//!
//! Run totals are the fold of every finished phase into one accumulator, so a
//! counter recorded once per phase can never disagree with its run total.

use super::{PhaseStats, APPLIED_MOVE_TRACE_LIMIT};
use crate::stats::{MoveTelemetry, SelectorTelemetry};

impl PhaseStats {
    /// Adds `other`'s counters, durations, selector and move-kind entries,
    /// and applied-move trace to `self`. The required-remaining gauge is a
    /// level, not a count, so it takes `other`'s value instead of a sum.
    pub(crate) fn absorb(&mut self, other: &PhaseStats) {
        self.step_count += other.step_count;
        self.moves_generated += other.moves_generated;
        self.moves_evaluated += other.moves_evaluated;
        self.moves_speculative += other.moves_speculative;
        self.moves_accepted += other.moves_accepted;
        self.moves_applied += other.moves_applied;
        self.moves_not_doable += other.moves_not_doable;
        self.moves_acceptor_rejected += other.moves_acceptor_rejected;
        self.moves_forager_ignored += other.moves_forager_ignored;
        self.moves_hard_improving += other.moves_hard_improving;
        self.moves_hard_neutral += other.moves_hard_neutral;
        self.moves_hard_worse += other.moves_hard_worse;
        self.conflict_repair_provider_generated += other.conflict_repair_provider_generated;
        self.conflict_repair_duplicate_filtered += other.conflict_repair_duplicate_filtered;
        self.conflict_repair_illegal_filtered += other.conflict_repair_illegal_filtered;
        self.conflict_repair_not_doable_filtered += other.conflict_repair_not_doable_filtered;
        self.conflict_repair_hard_improving += other.conflict_repair_hard_improving;
        self.conflict_repair_exposed += other.conflict_repair_exposed;
        self.score_calculations += other.score_calculations;
        self.construction_slots_assigned += other.construction_slots_assigned;
        self.construction_slots_kept += other.construction_slots_kept;
        self.construction_slots_no_doable += other.construction_slots_no_doable;
        self.scalar_assignment_required_remaining = other.scalar_assignment_required_remaining;
        self.generation_time += other.generation_time;
        self.evaluation_time += other.evaluation_time;
        for selector in &other.selector_stats {
            let entry = self
                .selector_stats_entry_with_label(selector.selector_index, &selector.selector_label);
            add_selector(entry, selector);
        }
        for (label, entry) in &other.move_stats {
            add_move(self.move_stats_entry(label), entry);
        }
        let room = APPLIED_MOVE_TRACE_LIMIT.saturating_sub(self.applied_move_trace.len());
        self.applied_move_trace
            .extend(other.applied_move_trace.iter().take(room).copied());
    }
}

fn add_selector(into: &mut SelectorTelemetry, from: &SelectorTelemetry) {
    into.moves_generated += from.moves_generated;
    into.moves_evaluated += from.moves_evaluated;
    into.moves_accepted += from.moves_accepted;
    into.moves_applied += from.moves_applied;
    into.moves_not_doable += from.moves_not_doable;
    into.moves_acceptor_rejected += from.moves_acceptor_rejected;
    into.moves_forager_ignored += from.moves_forager_ignored;
    into.moves_hard_improving += from.moves_hard_improving;
    into.moves_hard_neutral += from.moves_hard_neutral;
    into.moves_hard_worse += from.moves_hard_worse;
    into.generation_time += from.generation_time;
    into.evaluation_time += from.evaluation_time;
}

fn add_move(into: &mut MoveTelemetry, from: &MoveTelemetry) {
    into.moves_generated += from.moves_generated;
    into.moves_evaluated += from.moves_evaluated;
    into.moves_accepted += from.moves_accepted;
    into.moves_applied += from.moves_applied;
    into.moves_not_doable += from.moves_not_doable;
    into.moves_acceptor_rejected += from.moves_acceptor_rejected;
    into.moves_forager_ignored += from.moves_forager_ignored;
    into.moves_score_improving += from.moves_score_improving;
    into.moves_applied_improving += from.moves_applied_improving;
    into.moves_score_equal += from.moves_score_equal;
    into.moves_score_worse += from.moves_score_worse;
    into.moves_rejected_improving += from.moves_rejected_improving;
    into.applied_score_improvement += from.applied_score_improvement;
}
