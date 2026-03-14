// ─── PhaseTimer ───────────────────────────────────────────────────────────────

use std::time::{Duration, Instant};

use super::{print_phase_end, print_phase_start};

pub struct PhaseTimer {
    start: Instant,
    phase_name: String,
    phase_index: usize,
    steps_accepted: u64,
    moves_evaluated: u64,
    last_score: String,
}

impl PhaseTimer {
    pub fn start(phase_name: impl Into<String>, phase_index: usize) -> Self {
        let name = phase_name.into();
        print_phase_start(&name, phase_index);
        Self {
            start: Instant::now(),
            phase_name: name,
            phase_index,
            steps_accepted: 0,
            moves_evaluated: 0,
            last_score: String::new(),
        }
    }

    pub fn record_accepted(&mut self, score: &str) {
        self.steps_accepted += 1;
        self.last_score = score.to_string();
    }

    pub fn record_move(&mut self) {
        self.moves_evaluated += 1;
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    pub fn moves_evaluated(&self) -> u64 {
        self.moves_evaluated
    }
    pub fn steps_accepted(&self) -> u64 {
        self.steps_accepted
    }

    pub fn finish(self) {
        print_phase_end(
            &self.phase_name,
            self.phase_index,
            self.start.elapsed(),
            self.steps_accepted,
            self.moves_evaluated,
            &self.last_score,
        );
    }
}
