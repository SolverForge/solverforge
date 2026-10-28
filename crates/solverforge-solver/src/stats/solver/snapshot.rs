//! Telemetry snapshots of solve-wide statistics.

use super::super::{CandidateTraceTelemetry, PhaseTelemetry, SolverTelemetry};
use super::SolverStats;

impl SolverStats {
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
        let mut phases: Vec<PhaseTelemetry> = self
            .finished_phases
            .iter()
            .map(|phase| phase.stats.snapshot_with_elapsed(phase.elapsed))
            .collect();
        if let Some(phase) = &self.current_phase {
            totals.absorb(&phase.stats);
            phases.push(
                phase
                    .stats
                    .snapshot_with_elapsed(self.running_elapsed(phase)),
            );
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
            problem_changes_applied: self.problem_changes_applied,
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
}
