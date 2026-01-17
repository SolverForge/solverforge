//! Monomorphic forager enum for local search.
//!
//! All forager types wrapped in a single enum for config-driven selection
//! without type erasure.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_config::{ForagerConfig, PickEarlyType};
use solverforge_core::domain::PlanningSolution;

use super::forager::{AcceptedCountForager, FirstAcceptedForager, LocalSearchForager};

/// Monomorphic forager enum - runtime selection without type erasure.
///
/// Wraps all local search forager types, enabling config-driven selection
/// while preserving concrete types throughout the solver pipeline.
pub enum ForagerImpl<S: PlanningSolution> {
    /// Collects N accepted moves, picks best score.
    AcceptedCount(AcceptedCountForager<S>),
    /// Stops at first accepted move.
    FirstAccepted(FirstAcceptedForager<S>),
    /// Picks best score among all evaluated moves.
    BestScore(BestScoreForager<S>),
    /// Picks first move that improves best score.
    FirstBestScoreImproving(FirstBestScoreImprovingForager<S>),
    /// Picks first move that improves last step score.
    FirstLastStepScoreImproving(FirstLastStepScoreImprovingForager<S>),
    /// Records moves until N non-improving, then picks best.
    RecordToNonImproving(RecordToNonImprovingForager<S>),
}

// ============================================================================
// BestScoreForager - evaluates all moves, picks best
// ============================================================================

/// Forager that evaluates all moves and picks the one with best score.
pub struct BestScoreForager<S: PlanningSolution> {
    best_move: Option<(usize, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> BestScoreForager<S> {
    pub fn new() -> Self {
        Self {
            best_move: None,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution> Default for BestScoreForager<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: PlanningSolution> Clone for BestScoreForager<S> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<S: PlanningSolution> Debug for BestScoreForager<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BestScoreForager")
            .field("has_move", &self.best_move.is_some())
            .finish()
    }
}

impl<S: PlanningSolution> LocalSearchForager<S> for BestScoreForager<S> {
    fn step_started(&mut self) {
        self.best_move = None;
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        let dominated = self.best_move.as_ref().is_some_and(|(_, best)| score <= *best);
        if !dominated {
            self.best_move = Some((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        false // Never quit early - evaluate all moves
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        self.best_move.take()
    }
}

// ============================================================================
// FirstBestScoreImprovingForager - quit on first improvement over best
// ============================================================================

/// Forager that picks the first move improving the global best score.
pub struct FirstBestScoreImprovingForager<S: PlanningSolution> {
    best_score: Option<S::Score>,
    accepted_move: Option<(usize, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> FirstBestScoreImprovingForager<S> {
    pub fn new() -> Self {
        Self {
            best_score: None,
            accepted_move: None,
            _phantom: PhantomData,
        }
    }

    pub fn with_best_score(best_score: S::Score) -> Self {
        Self {
            best_score: Some(best_score),
            accepted_move: None,
            _phantom: PhantomData,
        }
    }

    pub fn set_best_score(&mut self, score: S::Score) {
        self.best_score = Some(score);
    }
}

impl<S: PlanningSolution> Default for FirstBestScoreImprovingForager<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: PlanningSolution> Clone for FirstBestScoreImprovingForager<S> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<S: PlanningSolution> Debug for FirstBestScoreImprovingForager<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstBestScoreImprovingForager")
            .field("has_best_score", &self.best_score.is_some())
            .finish()
    }
}

impl<S: PlanningSolution> LocalSearchForager<S> for FirstBestScoreImprovingForager<S> {
    fn step_started(&mut self) {
        self.accepted_move = None;
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        if self.accepted_move.is_some() {
            return;
        }
        let dominated = self.best_score.as_ref().is_some_and(|best| score <= *best);
        if !dominated {
            self.accepted_move = Some((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_move.is_some()
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        let result = self.accepted_move.take();
        if let Some((_, score)) = &result {
            self.best_score = Some(*score);
        }
        result
    }
}

// ============================================================================
// FirstLastStepScoreImprovingForager - quit on first improvement over last step
// ============================================================================

/// Forager that picks the first move improving the last step's score.
pub struct FirstLastStepScoreImprovingForager<S: PlanningSolution> {
    last_step_score: Option<S::Score>,
    accepted_move: Option<(usize, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> FirstLastStepScoreImprovingForager<S> {
    pub fn new() -> Self {
        Self {
            last_step_score: None,
            accepted_move: None,
            _phantom: PhantomData,
        }
    }

    pub fn with_last_step_score(score: S::Score) -> Self {
        Self {
            last_step_score: Some(score),
            accepted_move: None,
            _phantom: PhantomData,
        }
    }

    pub fn set_last_step_score(&mut self, score: S::Score) {
        self.last_step_score = Some(score);
    }
}

impl<S: PlanningSolution> Default for FirstLastStepScoreImprovingForager<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: PlanningSolution> Clone for FirstLastStepScoreImprovingForager<S> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<S: PlanningSolution> Debug for FirstLastStepScoreImprovingForager<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstLastStepScoreImprovingForager")
            .field("has_last_step_score", &self.last_step_score.is_some())
            .finish()
    }
}

impl<S: PlanningSolution> LocalSearchForager<S> for FirstLastStepScoreImprovingForager<S> {
    fn step_started(&mut self) {
        self.accepted_move = None;
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        if self.accepted_move.is_some() {
            return;
        }
        let dominated = self.last_step_score.as_ref().is_some_and(|last| score <= *last);
        if !dominated {
            self.accepted_move = Some((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_move.is_some()
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        let result = self.accepted_move.take();
        if let Some((_, score)) = &result {
            self.last_step_score = Some(*score);
        }
        result
    }
}

// ============================================================================
// RecordToNonImprovingForager - record until N non-improving, pick best
// ============================================================================

/// Forager that records moves until N consecutive non-improving moves.
pub struct RecordToNonImprovingForager<S: PlanningSolution> {
    non_improving_limit: usize,
    non_improving_count: usize,
    best_move: Option<(usize, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> RecordToNonImprovingForager<S> {
    pub fn new(non_improving_limit: usize) -> Self {
        Self {
            non_improving_limit,
            non_improving_count: 0,
            best_move: None,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution> Clone for RecordToNonImprovingForager<S> {
    fn clone(&self) -> Self {
        Self::new(self.non_improving_limit)
    }
}

impl<S: PlanningSolution> Debug for RecordToNonImprovingForager<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordToNonImprovingForager")
            .field("non_improving_limit", &self.non_improving_limit)
            .field("non_improving_count", &self.non_improving_count)
            .finish()
    }
}

impl<S: PlanningSolution> LocalSearchForager<S> for RecordToNonImprovingForager<S> {
    fn step_started(&mut self) {
        self.non_improving_count = 0;
        self.best_move = None;
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        let dominated = self.best_move.as_ref().is_some_and(|(_, best)| score <= *best);
        if dominated {
            self.non_improving_count += 1;
        } else {
            self.non_improving_count = 0;
            self.best_move = Some((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.non_improving_count >= self.non_improving_limit
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        self.best_move.take()
    }
}

// ============================================================================
// ForagerImpl - monomorphic enum implementation
// ============================================================================

impl<S: PlanningSolution> ForagerImpl<S> {
    /// Creates an accepted count forager.
    pub fn accepted_count(limit: usize) -> Self {
        Self::AcceptedCount(AcceptedCountForager::new(limit))
    }

    /// Creates a first accepted forager.
    pub fn first_accepted() -> Self {
        Self::FirstAccepted(FirstAcceptedForager::new())
    }

    /// Creates a best score forager.
    pub fn best_score() -> Self {
        Self::BestScore(BestScoreForager::new())
    }

    /// Creates a first best score improving forager.
    pub fn first_best_score_improving() -> Self {
        Self::FirstBestScoreImproving(FirstBestScoreImprovingForager::new())
    }

    /// Creates a first last step score improving forager.
    pub fn first_last_step_score_improving() -> Self {
        Self::FirstLastStepScoreImproving(FirstLastStepScoreImprovingForager::new())
    }

    /// Creates a record to non-improving forager.
    pub fn record_to_non_improving(limit: usize) -> Self {
        Self::RecordToNonImproving(RecordToNonImprovingForager::new(limit))
    }

    /// Creates a forager from configuration.
    pub fn from_config(config: Option<&ForagerConfig>) -> Self {
        match config {
            Some(cfg) => {
                let limit = cfg.accepted_count_limit.unwrap_or(1);
                match cfg.pick_early_type {
                    Some(PickEarlyType::FirstBestScoreImproving) => {
                        Self::first_best_score_improving()
                    }
                    Some(PickEarlyType::FirstLastStepScoreImproving) => {
                        Self::first_last_step_score_improving()
                    }
                    Some(PickEarlyType::Never) | None => {
                        if limit == usize::MAX {
                            Self::best_score()
                        } else {
                            Self::accepted_count(limit)
                        }
                    }
                }
            }
            None => Self::default(),
        }
    }
}

impl<S: PlanningSolution> Default for ForagerImpl<S> {
    fn default() -> Self {
        Self::accepted_count(1)
    }
}

impl<S: PlanningSolution> Debug for ForagerImpl<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AcceptedCount(fo) => fo.fmt(f),
            Self::FirstAccepted(fo) => fo.fmt(f),
            Self::BestScore(fo) => fo.fmt(f),
            Self::FirstBestScoreImproving(fo) => fo.fmt(f),
            Self::FirstLastStepScoreImproving(fo) => fo.fmt(f),
            Self::RecordToNonImproving(fo) => fo.fmt(f),
        }
    }
}

impl<S: PlanningSolution> Clone for ForagerImpl<S> {
    fn clone(&self) -> Self {
        match self {
            Self::AcceptedCount(fo) => Self::AcceptedCount(fo.clone()),
            Self::FirstAccepted(fo) => Self::FirstAccepted(fo.clone()),
            Self::BestScore(fo) => Self::BestScore(fo.clone()),
            Self::FirstBestScoreImproving(fo) => Self::FirstBestScoreImproving(fo.clone()),
            Self::FirstLastStepScoreImproving(fo) => Self::FirstLastStepScoreImproving(fo.clone()),
            Self::RecordToNonImproving(fo) => Self::RecordToNonImproving(fo.clone()),
        }
    }
}

impl<S: PlanningSolution> LocalSearchForager<S> for ForagerImpl<S> {
    fn step_started(&mut self) {
        match self {
            Self::AcceptedCount(fo) => fo.step_started(),
            Self::FirstAccepted(fo) => fo.step_started(),
            Self::BestScore(fo) => fo.step_started(),
            Self::FirstBestScoreImproving(fo) => fo.step_started(),
            Self::FirstLastStepScoreImproving(fo) => fo.step_started(),
            Self::RecordToNonImproving(fo) => fo.step_started(),
        }
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        match self {
            Self::AcceptedCount(fo) => fo.add_move_index(index, score),
            Self::FirstAccepted(fo) => fo.add_move_index(index, score),
            Self::BestScore(fo) => fo.add_move_index(index, score),
            Self::FirstBestScoreImproving(fo) => fo.add_move_index(index, score),
            Self::FirstLastStepScoreImproving(fo) => fo.add_move_index(index, score),
            Self::RecordToNonImproving(fo) => fo.add_move_index(index, score),
        }
    }

    fn is_quit_early(&self) -> bool {
        match self {
            Self::AcceptedCount(fo) => fo.is_quit_early(),
            Self::FirstAccepted(fo) => fo.is_quit_early(),
            Self::BestScore(fo) => fo.is_quit_early(),
            Self::FirstBestScoreImproving(fo) => fo.is_quit_early(),
            Self::FirstLastStepScoreImproving(fo) => fo.is_quit_early(),
            Self::RecordToNonImproving(fo) => fo.is_quit_early(),
        }
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        match self {
            Self::AcceptedCount(fo) => fo.pick_move_index(),
            Self::FirstAccepted(fo) => fo.pick_move_index(),
            Self::BestScore(fo) => fo.pick_move_index(),
            Self::FirstBestScoreImproving(fo) => fo.pick_move_index(),
            Self::FirstLastStepScoreImproving(fo) => fo.pick_move_index(),
            Self::RecordToNonImproving(fo) => fo.pick_move_index(),
        }
    }
}
