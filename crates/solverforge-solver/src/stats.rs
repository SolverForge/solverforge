/* Solver statistics (zero-erasure).

Stack-allocated statistics for solver and phase performance tracking.
*/

use std::time::{Duration, Instant};

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
    pub moves_generated: u64,
    pub moves_evaluated: u64,
    pub moves_accepted: u64,
    pub moves_applied: u64,
    pub generation_time: Duration,
    pub evaluation_time: Duration,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SolverTelemetry {
    pub elapsed: Duration,
    pub step_count: u64,
    pub moves_generated: u64,
    pub moves_evaluated: u64,
    pub moves_accepted: u64,
    pub moves_applied: u64,
    pub score_calculations: u64,
    pub generation_time: Duration,
    pub evaluation_time: Duration,
    pub selector_telemetry: Vec<SelectorTelemetry>,
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
            score_calculations: 0,
            generation_time: Duration::ZERO,
            evaluation_time: Duration::ZERO,
            selector_telemetry: Vec::new(),
        }
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
    // Total score calculations performed.
    pub score_calculations: u64,
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

    /// Records a step completion.
    pub fn record_step(&mut self) {
        self.step_count += 1;
    }

    /// Records a score calculation.
    pub fn record_score_calculation(&mut self) {
        self.score_calculations += 1;
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
            score_calculations: self.score_calculations,
            generation_time: self.generation_time,
            evaluation_time: self.evaluation_time,
            selector_telemetry: self.selector_stats.clone(),
        }
    }

    fn selector_stats_entry(&mut self, selector_index: usize) -> &mut SelectorTelemetry {
        if let Some(position) = self
            .selector_stats
            .iter()
            .position(|entry| entry.selector_index == selector_index)
        {
            return &mut self.selector_stats[position];
        }
        self.selector_stats.push(SelectorTelemetry {
            selector_index,
            selector_label: format!("selector-{selector_index}"),
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
    // Number of score calculations in this phase.
    pub score_calculations: u64,
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
            score_calculations: 0,
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

    /// Records a score calculation.
    pub fn record_score_calculation(&mut self) {
        self.score_calculations += 1;
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

    fn selector_stats_entry(&mut self, selector_index: usize) -> &mut SelectorTelemetry {
        if let Some(position) = self
            .selector_stats
            .iter()
            .position(|entry| entry.selector_index == selector_index)
        {
            return &mut self.selector_stats[position];
        }
        self.selector_stats.push(SelectorTelemetry {
            selector_index,
            selector_label: format!("selector-{selector_index}"),
            ..SelectorTelemetry::default()
        });
        self.selector_stats
            .last_mut()
            .expect("selector stats entry was just inserted")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solver_snapshot_preserves_exact_counts_and_durations() {
        let mut stats = SolverStats::default();
        stats.start();
        stats.record_step();
        stats.record_generated_batch(3, Duration::from_millis(4));
        stats.record_evaluated_move(Duration::from_millis(5));
        stats.record_move_accepted();
        stats.record_score_calculation();

        let snapshot = stats.snapshot();

        assert_eq!(snapshot.step_count, 1);
        assert_eq!(snapshot.moves_generated, 3);
        assert_eq!(snapshot.moves_evaluated, 1);
        assert_eq!(snapshot.moves_accepted, 1);
        assert_eq!(snapshot.score_calculations, 1);
        assert_eq!(snapshot.generation_time, Duration::from_millis(4));
        assert_eq!(snapshot.evaluation_time, Duration::from_millis(5));
    }

    #[test]
    fn phase_stats_track_generation_and_evaluation_separately() {
        let mut stats = PhaseStats::new(2, "LocalSearch");
        stats.record_step();
        stats.record_generated_batch(7, Duration::from_millis(8));
        stats.record_evaluated_move(Duration::from_millis(9));
        stats.record_move_accepted();
        stats.record_score_calculation();

        assert_eq!(stats.phase_index, 2);
        assert_eq!(stats.phase_type, "LocalSearch");
        assert_eq!(stats.step_count, 1);
        assert_eq!(stats.moves_generated, 7);
        assert_eq!(stats.moves_evaluated, 1);
        assert_eq!(stats.moves_accepted, 1);
        assert_eq!(stats.score_calculations, 1);
        assert_eq!(stats.generation_time(), Duration::from_millis(8));
        assert_eq!(stats.evaluation_time(), Duration::from_millis(9));
    }

    #[test]
    fn solver_snapshot_includes_selector_level_telemetry() {
        let mut stats = SolverStats::default();
        stats.start();
        stats.record_selector_generated(2, 3, Duration::from_millis(4));
        stats.record_selector_evaluated(2, Duration::from_millis(5));
        stats.record_selector_accepted(2);
        stats.record_selector_applied(2);

        let snapshot = stats.snapshot();

        assert_eq!(snapshot.moves_generated, 3);
        assert_eq!(snapshot.moves_evaluated, 1);
        assert_eq!(snapshot.moves_accepted, 1);
        assert_eq!(snapshot.moves_applied, 1);
        assert_eq!(snapshot.selector_telemetry.len(), 1);
        assert_eq!(snapshot.selector_telemetry[0].selector_index, 2);
        assert_eq!(snapshot.selector_telemetry[0].selector_label, "selector-2");
        assert_eq!(snapshot.selector_telemetry[0].moves_generated, 3);
        assert_eq!(snapshot.selector_telemetry[0].moves_evaluated, 1);
        assert_eq!(snapshot.selector_telemetry[0].moves_accepted, 1);
        assert_eq!(snapshot.selector_telemetry[0].moves_applied, 1);
    }

    #[test]
    fn unattributed_applied_moves_do_not_create_selector_zero_telemetry() {
        let mut stats = SolverStats::default();
        stats.record_generated_move(Duration::from_millis(1));
        stats.record_evaluated_move(Duration::from_millis(2));
        stats.record_move_accepted();
        stats.record_move_applied();

        let snapshot = stats.snapshot();

        assert_eq!(snapshot.moves_generated, 1);
        assert_eq!(snapshot.moves_evaluated, 1);
        assert_eq!(snapshot.moves_accepted, 1);
        assert_eq!(snapshot.moves_applied, 1);
        assert!(snapshot.selector_telemetry.is_empty());
    }

    #[test]
    fn throughput_helpers_use_stage_specific_durations() {
        let mut solver_stats = SolverStats::default();
        solver_stats.start();
        solver_stats.record_generated_batch(5, Duration::from_millis(7));
        solver_stats.record_evaluated_move(Duration::from_millis(11));

        let mut phase_stats = PhaseStats::new(1, "LocalSearch");
        phase_stats.record_generated_batch(3, Duration::from_millis(13));
        phase_stats.record_evaluated_move(Duration::from_millis(17));

        assert_eq!(
            solver_stats.generated_throughput(),
            Throughput {
                count: 5,
                elapsed: Duration::from_millis(7),
            }
        );
        assert_eq!(
            solver_stats.evaluated_throughput(),
            Throughput {
                count: 1,
                elapsed: Duration::from_millis(11),
            }
        );
        assert_eq!(
            phase_stats.generated_throughput(),
            Throughput {
                count: 3,
                elapsed: Duration::from_millis(13),
            }
        );
        assert_eq!(
            phase_stats.evaluated_throughput(),
            Throughput {
                count: 1,
                elapsed: Duration::from_millis(17),
            }
        );
    }

    #[test]
    fn whole_units_per_second_uses_integer_rate_math() {
        assert_eq!(whole_units_per_second(3, Duration::from_millis(2_000)), 1);
        assert_eq!(whole_units_per_second(9, Duration::from_secs(2)), 4);
        assert_eq!(whole_units_per_second(5, Duration::ZERO), 0);
    }

    #[test]
    fn format_duration_uses_exact_integer_units() {
        assert_eq!(format_duration(Duration::from_millis(750)), "750ms");
        assert_eq!(format_duration(Duration::from_millis(2_500)), "2s 500ms");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 5s");
        assert_eq!(format_duration(Duration::from_micros(42)), "42us");
    }
}
