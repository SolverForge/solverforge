//! Monomorphic enum for local search foragers.
//!
//! Provides zero-erasure dispatch over all forager variants.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_config::{ForagerConfig, PickEarlyType};
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::Move;

use super::forager::{AcceptedCountForager, FirstAcceptedForager, LocalSearchForager};

/// Forager that quits early when it finds a move improving the best score.
pub struct FirstBestScoreImprovingForager<S: PlanningSolution> {
    best_score: Option<S::Score>,
    accepted_move: Option<(usize, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for FirstBestScoreImprovingForager<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstBestScoreImprovingForager")
            .field("has_move", &self.accepted_move.is_some())
            .finish()
    }
}

impl<S: PlanningSolution> Clone for FirstBestScoreImprovingForager<S> {
    fn clone(&self) -> Self {
        Self {
            best_score: None,
            accepted_move: None,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution> FirstBestScoreImprovingForager<S> {
    pub fn new() -> Self {
        Self {
            best_score: None,
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

impl<S, M> LocalSearchForager<S, M> for FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self) {
        self.accepted_move = None;
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        if self.accepted_move.is_some() {
            return;
        }
        if let Some(ref best) = self.best_score {
            if score > *best {
                self.accepted_move = Some((index, score));
            }
        } else {
            self.accepted_move = Some((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_move.is_some()
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        self.accepted_move.take()
    }
}

/// Forager that quits early when it finds a move improving the last step score.
pub struct FirstLastStepScoreImprovingForager<S: PlanningSolution> {
    last_step_score: Option<S::Score>,
    accepted_move: Option<(usize, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for FirstLastStepScoreImprovingForager<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstLastStepScoreImprovingForager")
            .field("has_move", &self.accepted_move.is_some())
            .finish()
    }
}

impl<S: PlanningSolution> Clone for FirstLastStepScoreImprovingForager<S> {
    fn clone(&self) -> Self {
        Self {
            last_step_score: None,
            accepted_move: None,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution> FirstLastStepScoreImprovingForager<S> {
    pub fn new() -> Self {
        Self {
            last_step_score: None,
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

impl<S, M> LocalSearchForager<S, M> for FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self) {
        self.accepted_move = None;
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        if self.accepted_move.is_some() {
            return;
        }
        if let Some(ref last) = self.last_step_score {
            if score > *last {
                self.accepted_move = Some((index, score));
            }
        } else {
            self.accepted_move = Some((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_move.is_some()
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        self.accepted_move.take()
    }
}

/// Monomorphic enum wrapping all local search forager implementations.
pub enum LocalSearchForagerImpl<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    AcceptedCount(AcceptedCountForager<S>, PhantomData<fn() -> M>),
    FirstAccepted(FirstAcceptedForager<S>, PhantomData<fn() -> M>),
    FirstBestScoreImproving(FirstBestScoreImprovingForager<S>, PhantomData<fn() -> M>),
    FirstLastStepScoreImproving(
        FirstLastStepScoreImprovingForager<S>,
        PhantomData<fn() -> M>,
    ),
}

impl<S, M> LocalSearchForagerImpl<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn from_config(config: &ForagerConfig) -> Self {
        match config.pick_early_type {
            Some(PickEarlyType::FirstBestScoreImproving) => {
                LocalSearchForagerImpl::FirstBestScoreImproving(
                    FirstBestScoreImprovingForager::new(),
                    PhantomData,
                )
            }
            Some(PickEarlyType::FirstLastStepScoreImproving) => {
                LocalSearchForagerImpl::FirstLastStepScoreImproving(
                    FirstLastStepScoreImprovingForager::new(),
                    PhantomData,
                )
            }
            Some(PickEarlyType::Never) | None => {
                let limit = config.accepted_count_limit.unwrap_or(1000);
                if limit == 1 {
                    LocalSearchForagerImpl::FirstAccepted(FirstAcceptedForager::new(), PhantomData)
                } else {
                    LocalSearchForagerImpl::AcceptedCount(
                        AcceptedCountForager::new(limit),
                        PhantomData,
                    )
                }
            }
        }
    }

    pub fn accepted_count(limit: usize) -> Self {
        LocalSearchForagerImpl::AcceptedCount(AcceptedCountForager::new(limit), PhantomData)
    }

    pub fn set_best_score(&mut self, score: S::Score) {
        if let LocalSearchForagerImpl::FirstBestScoreImproving(ref mut f, _) = self {
            f.set_best_score(score);
        }
    }

    pub fn set_last_step_score(&mut self, score: S::Score) {
        if let LocalSearchForagerImpl::FirstLastStepScoreImproving(ref mut f, _) = self {
            f.set_last_step_score(score);
        }
    }
}

impl<S, M> Debug for LocalSearchForagerImpl<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AcceptedCount(inner, _) => inner.fmt(f),
            Self::FirstAccepted(inner, _) => inner.fmt(f),
            Self::FirstBestScoreImproving(inner, _) => inner.fmt(f),
            Self::FirstLastStepScoreImproving(inner, _) => inner.fmt(f),
        }
    }
}

impl<S, M> Clone for LocalSearchForagerImpl<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn clone(&self) -> Self {
        match self {
            Self::AcceptedCount(inner, _) => Self::AcceptedCount(inner.clone(), PhantomData),
            Self::FirstAccepted(inner, _) => Self::FirstAccepted(inner.clone(), PhantomData),
            Self::FirstBestScoreImproving(inner, _) => {
                Self::FirstBestScoreImproving(inner.clone(), PhantomData)
            }
            Self::FirstLastStepScoreImproving(inner, _) => {
                Self::FirstLastStepScoreImproving(inner.clone(), PhantomData)
            }
        }
    }
}

impl<S, M> LocalSearchForager<S, M> for LocalSearchForagerImpl<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self) {
        match self {
            Self::AcceptedCount(f, _) => {
                <AcceptedCountForager<S> as LocalSearchForager<S, M>>::step_started(f)
            }
            Self::FirstAccepted(f, _) => {
                <FirstAcceptedForager<S> as LocalSearchForager<S, M>>::step_started(f)
            }
            Self::FirstBestScoreImproving(f, _) => {
                <FirstBestScoreImprovingForager<S> as LocalSearchForager<S, M>>::step_started(f)
            }
            Self::FirstLastStepScoreImproving(f, _) => {
                <FirstLastStepScoreImprovingForager<S> as LocalSearchForager<S, M>>::step_started(f)
            }
        }
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        match self {
            Self::AcceptedCount(f, _) => {
                <AcceptedCountForager<S> as LocalSearchForager<S, M>>::add_move_index(
                    f, index, score,
                )
            }
            Self::FirstAccepted(f, _) => {
                <FirstAcceptedForager<S> as LocalSearchForager<S, M>>::add_move_index(
                    f, index, score,
                )
            }
            Self::FirstBestScoreImproving(f, _) => {
                <FirstBestScoreImprovingForager<S> as LocalSearchForager<S, M>>::add_move_index(
                    f, index, score,
                )
            }
            Self::FirstLastStepScoreImproving(f, _) => {
                <FirstLastStepScoreImprovingForager<S> as LocalSearchForager<S, M>>::add_move_index(
                    f, index, score,
                )
            }
        }
    }

    fn is_quit_early(&self) -> bool {
        match self {
            Self::AcceptedCount(f, _) => {
                <AcceptedCountForager<S> as LocalSearchForager<S, M>>::is_quit_early(f)
            }
            Self::FirstAccepted(f, _) => {
                <FirstAcceptedForager<S> as LocalSearchForager<S, M>>::is_quit_early(f)
            }
            Self::FirstBestScoreImproving(f, _) => {
                <FirstBestScoreImprovingForager<S> as LocalSearchForager<S, M>>::is_quit_early(f)
            }
            Self::FirstLastStepScoreImproving(f, _) => {
                <FirstLastStepScoreImprovingForager<S> as LocalSearchForager<S, M>>::is_quit_early(
                    f,
                )
            }
        }
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        match self {
            Self::AcceptedCount(f, _) => {
                <AcceptedCountForager<S> as LocalSearchForager<S, M>>::pick_move_index(f)
            }
            Self::FirstAccepted(f, _) => {
                <FirstAcceptedForager<S> as LocalSearchForager<S, M>>::pick_move_index(f)
            }
            Self::FirstBestScoreImproving(f, _) => {
                <FirstBestScoreImprovingForager<S> as LocalSearchForager<S, M>>::pick_move_index(f)
            }
            Self::FirstLastStepScoreImproving(f, _) => {
                <FirstLastStepScoreImprovingForager<S> as LocalSearchForager<S, M>>::pick_move_index(
                    f,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::ChangeMove;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Debug)]
    struct DummySolution {
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for DummySolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    type TestMove = ChangeMove<DummySolution, i32>;

    #[test]
    fn test_from_config_accepted_count() {
        let config = ForagerConfig {
            accepted_count_limit: Some(100),
            pick_early_type: None,
        };
        let forager = LocalSearchForagerImpl::<DummySolution, TestMove>::from_config(&config);
        assert!(matches!(
            forager,
            LocalSearchForagerImpl::AcceptedCount(_, _)
        ));
    }

    #[test]
    fn test_from_config_first_best_score_improving() {
        let config = ForagerConfig {
            accepted_count_limit: None,
            pick_early_type: Some(PickEarlyType::FirstBestScoreImproving),
        };
        let forager = LocalSearchForagerImpl::<DummySolution, TestMove>::from_config(&config);
        assert!(matches!(
            forager,
            LocalSearchForagerImpl::FirstBestScoreImproving(_, _)
        ));
    }

    #[test]
    fn test_first_best_score_improving_accepts_better() {
        let mut forager = FirstBestScoreImprovingForager::<DummySolution>::new();
        forager.set_best_score(SimpleScore::of(-10));
        <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::step_started(&mut forager);

        <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::add_move_index(&mut forager, 0, SimpleScore::of(-5));
        assert!(
            <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
                DummySolution,
                TestMove,
            >>::is_quit_early(&forager)
        );

        let picked = <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager);
        assert_eq!(picked, Some((0, SimpleScore::of(-5))));
    }

    #[test]
    fn test_first_best_score_improving_rejects_worse() {
        let mut forager = FirstBestScoreImprovingForager::<DummySolution>::new();
        forager.set_best_score(SimpleScore::of(-5));
        <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::step_started(&mut forager);

        <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::add_move_index(&mut forager, 0, SimpleScore::of(-10));
        assert!(
            !<FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
                DummySolution,
                TestMove,
            >>::is_quit_early(&forager)
        );
    }

    #[test]
    fn test_first_last_step_improving_accepts_better() {
        let mut forager = FirstLastStepScoreImprovingForager::<DummySolution>::new();
        forager.set_last_step_score(SimpleScore::of(-10));
        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::step_started(&mut forager);

        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::add_move_index(&mut forager, 0, SimpleScore::of(-5));
        assert!(
            <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
                DummySolution,
                TestMove,
            >>::is_quit_early(&forager)
        );
    }
}
