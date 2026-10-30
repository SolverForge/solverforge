//! Conflict-repair outcome counts gathered while candidates are pulled.
//!
//! The provider cursor counts each outcome at the point that decides it. A
//! cursor cannot reach the phase statistics, so the counts accumulate in the
//! solve-owned provider resources and the local-search step moves them into
//! [`PhaseStats`] once its candidate pulls are done.

use crate::builder::context::ProviderNormalizationState;
use crate::stats::PhaseStats;

/// Conflict-repair outcomes counted since the last flush.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ConflictRepairCounts {
    /// Raw candidates the repair providers returned.
    pub(crate) provider_generated: u64,
    /// Candidates dropped as exact repeats of an earlier candidate.
    pub(crate) duplicate_filtered: u64,
    /// Candidates dropped as structurally illegal.
    pub(crate) illegal_filtered: u64,
    /// Candidates dropped because they were not doable on the solution.
    pub(crate) not_doable_filtered: u64,
    /// Candidates the cursor handed to its consumer.
    pub(crate) exposed: u64,
}

impl ConflictRepairCounts {
    /// Adds the drops one normalization scope decided.
    pub(crate) fn record_normalization(&mut self, state: &ProviderNormalizationState) {
        self.duplicate_filtered += state.duplicate_filtered;
        self.illegal_filtered += state.illegal_filtered;
    }

    /// Moves every count into `stats` and resets this accumulator.
    pub(crate) fn flush_into(&mut self, stats: &mut PhaseStats) {
        if *self == Self::default() {
            return;
        }
        let counts = std::mem::take(self);
        stats.record_conflict_repair_provider_generated(counts.provider_generated);
        stats.record_conflict_repair_duplicate_filtered(counts.duplicate_filtered);
        stats.record_conflict_repair_illegal_filtered(counts.illegal_filtered);
        stats.record_conflict_repair_not_doable_filtered(counts.not_doable_filtered);
        stats.record_conflict_repair_exposed(counts.exposed);
    }
}
