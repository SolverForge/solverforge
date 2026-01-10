//! Solver-level scope.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::statistics::StatisticsCollector;

/// Top-level scope for the entire solving process.
///
/// Generic over `D: ScoreDirector<S>` for zero type erasure.
pub struct SolverScope<S: PlanningSolution, D: ScoreDirector<S>> {
    score_director: D,
    best_solution: Option<S>,
    best_score: Option<S::Score>,
    rng: StdRng,
    start_time: Option<Instant>,
    total_step_count: u64,
    statistics: Option<Arc<StatisticsCollector<S::Score>>>,
    terminate_early_flag: Option<Arc<AtomicBool>>,
}

impl<S: PlanningSolution, D: ScoreDirector<S>> SolverScope<S, D> {
    pub fn new(score_director: D) -> Self {
        Self {
            score_director,
            best_solution: None,
            best_score: None,
            rng: StdRng::from_os_rng(),
            start_time: None,
            total_step_count: 0,
            statistics: None,
            terminate_early_flag: None,
        }
    }

    pub fn with_seed(score_director: D, seed: u64) -> Self {
        Self {
            score_director,
            best_solution: None,
            best_score: None,
            rng: StdRng::seed_from_u64(seed),
            start_time: None,
            total_step_count: 0,
            statistics: None,
            terminate_early_flag: None,
        }
    }

    pub fn with_statistics(mut self, collector: Arc<StatisticsCollector<S::Score>>) -> Self {
        self.statistics = Some(collector);
        self
    }

    pub fn statistics(&self) -> Option<&Arc<StatisticsCollector<S::Score>>> {
        self.statistics.as_ref()
    }

    pub fn record_move(&self, accepted: bool) {
        if let Some(stats) = &self.statistics {
            stats.record_move(accepted);
        }
    }

    pub fn record_score_calculation(&self) {
        if let Some(stats) = &self.statistics {
            stats.record_score_calculation();
        }
    }

    pub fn start_solving(&mut self) {
        self.start_time = Some(Instant::now());
        self.total_step_count = 0;
    }

    pub fn elapsed(&self) -> Option<std::time::Duration> {
        self.start_time.map(|t| t.elapsed())
    }

    pub fn score_director(&self) -> &D {
        &self.score_director
    }

    pub fn score_director_mut(&mut self) -> &mut D {
        &mut self.score_director
    }

    pub fn working_solution(&self) -> &S {
        self.score_director.working_solution()
    }

    pub fn working_solution_mut(&mut self) -> &mut S {
        self.score_director.working_solution_mut()
    }

    pub fn calculate_score(&mut self) -> S::Score {
        self.score_director.calculate_score()
    }

    pub fn best_solution(&self) -> Option<&S> {
        self.best_solution.as_ref()
    }

    pub fn best_score(&self) -> Option<&S::Score> {
        self.best_score.as_ref()
    }

    pub fn update_best_solution(&mut self) {
        let current_score = self.score_director.calculate_score();
        let is_better = match &self.best_score {
            None => true,
            Some(best) => current_score > *best,
        };

        if is_better {
            self.best_solution = Some(self.score_director.clone_working_solution());
            self.best_score = Some(current_score.clone());

            if let Some(stats) = &self.statistics {
                stats.record_improvement(current_score);
            }
        }
    }

    pub fn set_best_solution(&mut self, solution: S, score: S::Score) {
        self.best_solution = Some(solution);
        self.best_score = Some(score);
    }

    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    pub fn increment_step_count(&mut self) -> u64 {
        self.total_step_count += 1;
        self.total_step_count
    }

    pub fn total_step_count(&self) -> u64 {
        self.total_step_count
    }

    pub fn take_best_solution(self) -> Option<S> {
        self.best_solution
    }

    pub fn take_best_or_working_solution(self) -> S {
        self.best_solution
            .unwrap_or_else(|| self.score_director.clone_working_solution())
    }

    pub fn set_terminate_early_flag(&mut self, flag: Arc<AtomicBool>) {
        self.terminate_early_flag = Some(flag);
    }

    pub fn is_terminate_early(&self) -> bool {
        self.terminate_early_flag
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
    }
}
