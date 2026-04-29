use std::time::{Duration, Instant};

use super::{SelectorTelemetry, SolverTelemetry, Throughput};

#[derive(Debug, Default)]
pub struct SolverStats {
    start_time: Option<Instant>,
    pause_started_at: Option<Instant>,
    // Total steps taken across all phases.
    pub step_count: u64,
    // Total moves generated across all phases.
    pub moves_generated: u64,
    // Total moves evaluated across all phases.
    pub moves_evaluated: u64,
    // Total moves accepted across all phases.
    pub moves_accepted: u64,
    // Total moves applied across all phases.
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
    // Total score calculations performed.
    pub score_calculations: u64,
    pub construction_slots_assigned: u64,
    pub construction_slots_kept: u64,
    pub construction_slots_no_doable: u64,
    generation_time: Duration,
    evaluation_time: Duration,
    selector_stats: Vec<SelectorTelemetry>,
}

impl SolverStats {
    /// Marks the start of solving.
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.pause_started_at = None;
    }

    pub fn elapsed(&self) -> Duration {
        match (self.start_time, self.pause_started_at) {
            (Some(start), Some(paused_at)) => paused_at.duration_since(start),
            (Some(start), None) => start.elapsed(),
            _ => Duration::default(),
        }
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

    /// Records one or more generated candidate moves and the time spent generating them.
    pub fn record_generated_batch(&mut self, count: u64, duration: Duration) {
        self.moves_generated += count;
        self.generation_time += duration;
    }

    pub fn record_selector_generated(
        &mut self,
        selector_index: usize,
        count: u64,
        duration: Duration,
    ) {
        self.record_generated_batch(count, duration);
        let selector = self.selector_stats_entry(selector_index);
        selector.moves_generated += count;
        selector.generation_time += duration;
    }

    /// Records generation time that did not itself yield a counted move.
    pub fn record_generation_time(&mut self, duration: Duration) {
        self.generation_time += duration;
    }

    /// Records a single generated candidate move and the time spent generating it.
    pub fn record_generated_move(&mut self, duration: Duration) {
        self.record_generated_batch(1, duration);
    }

    /// Records a move evaluation and the time spent evaluating it.
    pub fn record_evaluated_move(&mut self, duration: Duration) {
        self.moves_evaluated += 1;
        self.evaluation_time += duration;
    }

    pub fn record_selector_evaluated(&mut self, selector_index: usize, duration: Duration) {
        self.record_evaluated_move(duration);
        let selector = self.selector_stats_entry(selector_index);
        selector.moves_evaluated += 1;
        selector.evaluation_time += duration;
    }

    /// Records an accepted move.
    pub fn record_move_accepted(&mut self) {
        self.moves_accepted += 1;
    }

    pub fn record_selector_accepted(&mut self, selector_index: usize) {
        self.record_move_accepted();
        self.selector_stats_entry(selector_index).moves_accepted += 1;
    }

    pub fn record_move_applied(&mut self) {
        self.moves_applied += 1;
    }

    pub fn record_selector_applied(&mut self, selector_index: usize) {
        self.record_move_applied();
        self.selector_stats_entry(selector_index).moves_applied += 1;
    }

    pub fn record_move_not_doable(&mut self) {
        self.moves_not_doable += 1;
    }

    pub fn record_selector_not_doable(&mut self, selector_index: usize) {
        self.record_move_not_doable();
        self.selector_stats_entry(selector_index).moves_not_doable += 1;
    }

    pub fn record_move_acceptor_rejected(&mut self) {
        self.moves_acceptor_rejected += 1;
    }

    pub fn record_selector_acceptor_rejected(&mut self, selector_index: usize) {
        self.record_move_acceptor_rejected();
        self.selector_stats_entry(selector_index)
            .moves_acceptor_rejected += 1;
    }

    pub fn record_moves_forager_ignored(&mut self, count: u64) {
        self.moves_forager_ignored += count;
    }

    pub fn record_move_hard_improving(&mut self) {
        self.moves_hard_improving += 1;
    }

    pub fn record_move_hard_neutral(&mut self) {
        self.moves_hard_neutral += 1;
    }

    pub fn record_move_hard_worse(&mut self) {
        self.moves_hard_worse += 1;
    }

    pub fn record_conflict_repair_provider_generated(&mut self, count: u64) {
        self.conflict_repair_provider_generated += count;
    }

    pub fn record_conflict_repair_duplicate_filtered(&mut self) {
        self.conflict_repair_duplicate_filtered += 1;
    }

    pub fn record_conflict_repair_illegal_filtered(&mut self) {
        self.conflict_repair_illegal_filtered += 1;
    }

    pub fn record_conflict_repair_not_doable_filtered(&mut self) {
        self.conflict_repair_not_doable_filtered += 1;
    }

    pub fn record_conflict_repair_hard_improving(&mut self) {
        self.conflict_repair_hard_improving += 1;
    }

    pub fn record_conflict_repair_exposed(&mut self) {
        self.conflict_repair_exposed += 1;
    }

    /// Records a step completion.
    pub fn record_step(&mut self) {
        self.step_count += 1;
    }

    /// Records a score calculation.
    pub fn record_score_calculation(&mut self) {
        self.score_calculations += 1;
    }

    pub fn record_construction_slot_assigned(&mut self) {
        self.construction_slots_assigned += 1;
    }

    pub fn record_construction_slot_kept(&mut self) {
        self.construction_slots_kept += 1;
    }

    pub fn record_construction_slot_no_doable(&mut self) {
        self.construction_slots_no_doable += 1;
    }

    pub fn generated_throughput(&self) -> Throughput {
        Throughput {
            count: self.moves_generated,
            elapsed: self.generation_time,
        }
    }

    pub fn evaluated_throughput(&self) -> Throughput {
        Throughput {
            count: self.moves_evaluated,
            elapsed: self.evaluation_time,
        }
    }

    pub fn acceptance_rate(&self) -> f64 {
        if self.moves_evaluated == 0 {
            0.0
        } else {
            self.moves_accepted as f64 / self.moves_evaluated as f64
        }
    }

    pub fn generation_time(&self) -> Duration {
        self.generation_time
    }

    pub fn evaluation_time(&self) -> Duration {
        self.evaluation_time
    }

    pub fn snapshot(&self) -> SolverTelemetry {
        SolverTelemetry {
            elapsed: self.elapsed(),
            step_count: self.step_count,
            moves_generated: self.moves_generated,
            moves_evaluated: self.moves_evaluated,
            moves_accepted: self.moves_accepted,
            moves_applied: self.moves_applied,
            moves_not_doable: self.moves_not_doable,
            moves_acceptor_rejected: self.moves_acceptor_rejected,
            moves_forager_ignored: self.moves_forager_ignored,
            moves_hard_improving: self.moves_hard_improving,
            moves_hard_neutral: self.moves_hard_neutral,
            moves_hard_worse: self.moves_hard_worse,
            conflict_repair_provider_generated: self.conflict_repair_provider_generated,
            conflict_repair_duplicate_filtered: self.conflict_repair_duplicate_filtered,
            conflict_repair_illegal_filtered: self.conflict_repair_illegal_filtered,
            conflict_repair_not_doable_filtered: self.conflict_repair_not_doable_filtered,
            conflict_repair_hard_improving: self.conflict_repair_hard_improving,
            conflict_repair_exposed: self.conflict_repair_exposed,
            score_calculations: self.score_calculations,
            construction_slots_assigned: self.construction_slots_assigned,
            construction_slots_kept: self.construction_slots_kept,
            construction_slots_no_doable: self.construction_slots_no_doable,
            generation_time: self.generation_time,
            evaluation_time: self.evaluation_time,
            selector_telemetry: self.selector_stats.clone(),
        }
    }

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
