use std::time::{Duration, Instant};

use super::{SelectorTelemetry, Throughput};

#[derive(Debug)]
pub struct PhaseStats {
    // Index of this phase (0-based).
    pub phase_index: usize,
    // Type name of the phase.
    pub phase_type: &'static str,
    start_time: Instant,
    // Number of steps taken in this phase.
    pub step_count: u64,
    // Number of moves generated in this phase.
    pub moves_generated: u64,
    // Number of moves evaluated in this phase.
    pub moves_evaluated: u64,
    // Number of moves accepted in this phase.
    pub moves_accepted: u64,
    // Number of moves applied in this phase.
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
    // Number of score calculations in this phase.
    pub score_calculations: u64,
    pub construction_slots_assigned: u64,
    pub construction_slots_kept: u64,
    pub construction_slots_no_doable: u64,
    pub coverage_required_remaining: u64,
    generation_time: Duration,
    evaluation_time: Duration,
    selector_stats: Vec<SelectorTelemetry>,
}

impl PhaseStats {
    /// Creates new phase statistics.
    pub fn new(phase_index: usize, phase_type: &'static str) -> Self {
        Self {
            phase_index,
            phase_type,
            start_time: Instant::now(),
            step_count: 0,
            moves_generated: 0,
            moves_evaluated: 0,
            moves_accepted: 0,
            moves_applied: 0,
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
            coverage_required_remaining: 0,
            generation_time: Duration::default(),
            evaluation_time: Duration::default(),
            selector_stats: Vec::new(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Records a step completion.
    pub fn record_step(&mut self) {
        self.step_count += 1;
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

    pub fn record_coverage_required_remaining(&mut self, count: u64) {
        self.coverage_required_remaining = count;
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

    pub fn selector_telemetry(&self) -> &[SelectorTelemetry] {
        &self.selector_stats
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
