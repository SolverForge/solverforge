//! Local search phase factory.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::Move;
use crate::heuristic::selector::MoveSelector;
use crate::phase::localsearch::{
    AcceptedCountForager, Acceptor, HillClimbingAcceptor, LocalSearchForager, LocalSearchPhase,
};

use super::super::SolverPhaseFactory;

/// Factory for creating local search phases.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `M` - The move type
/// * `MS` - The move selector type
/// * `A` - The acceptor type
/// * `Fo` - The forager type
/// * `MSF` - The move selector factory closure type
/// * `AF` - The acceptor factory closure type
/// * `FoF` - The forager factory closure type
pub struct LocalSearchPhaseFactory<S, M, MS, A, Fo, MSF, AF, FoF>
where
    S: PlanningSolution,
    M: Move<S>,
{
    move_selector_factory: MSF,
    acceptor_factory: AF,
    forager_factory: FoF,
    step_limit: Option<u64>,
    _marker: PhantomData<(S, M, MS, A, Fo)>,
}

impl<S, M, MS, A, Fo, MSF, AF, FoF> LocalSearchPhaseFactory<S, M, MS, A, Fo, MSF, AF, FoF>
where
    S: PlanningSolution,
    M: Move<S>,
    MSF: Fn() -> MS + Send + Sync,
    AF: Fn() -> A + Send + Sync,
    FoF: Fn() -> Fo + Send + Sync,
{
    /// Creates a new local search phase factory with custom factories.
    pub fn new(move_selector_factory: MSF, acceptor_factory: AF, forager_factory: FoF) -> Self {
        Self {
            move_selector_factory,
            acceptor_factory,
            forager_factory,
            step_limit: None,
            _marker: PhantomData,
        }
    }

    /// Sets the step limit for this phase.
    pub fn with_step_limit(mut self, limit: u64) -> Self {
        self.step_limit = Some(limit);
        self
    }
}

/// Type alias for hill climbing factory with default forager.
pub type HillClimbingFactory<S, M, MS, MSF> = LocalSearchPhaseFactory<
    S,
    M,
    MS,
    HillClimbingAcceptor,
    AcceptedCountForager<S, M>,
    MSF,
    fn() -> HillClimbingAcceptor,
    fn() -> AcceptedCountForager<S, M>,
>;

impl<S, M, MS, MSF> HillClimbingFactory<S, M, MS, MSF>
where
    S: PlanningSolution,
    M: Move<S>,
    MSF: Fn() -> MS + Send + Sync,
{
    /// Creates a factory with hill climbing acceptor.
    ///
    /// Hill climbing only accepts moves that improve the score.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchPhaseFactory;
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
    /// use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
    /// use solverforge_solver::heuristic::selector::typed_value::StaticTypedValueSelector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone)]
    /// struct Sol { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    ///
    /// impl PlanningSolution for Sol {
    ///     type Score = SimpleScore;
    ///     fn score(&self) -> Option<Self::Score> { self.score }
    ///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// }
    ///
    /// fn get_v(s: &Sol, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// fn set_v(s: &mut Sol, idx: usize, v: Option<i32>) {
    ///     if let Some(x) = s.values.get_mut(idx) { *x = v; }
    /// }
    ///
    /// type MS = ChangeMoveSelector<Sol, i32, FromSolutionEntitySelector, StaticTypedValueSelector<i32>>;
    /// type Move = ChangeMove<Sol, i32>;
    ///
    /// let factory = LocalSearchPhaseFactory::<Sol, Move, MS, _>::hill_climbing(|| {
    ///     ChangeMoveSelector::simple(get_v, set_v, 0, "v", vec![1, 2, 3])
    /// });
    /// ```
    pub fn hill_climbing(move_selector_factory: MSF) -> Self {
        Self {
            move_selector_factory,
            acceptor_factory: HillClimbingAcceptor::new,
            forager_factory: || AcceptedCountForager::new(1),
            step_limit: None,
            _marker: PhantomData,
        }
    }
}

impl<S, M, D, MS, A, Fo, MSF, AF, FoF>
    SolverPhaseFactory<S, D, LocalSearchPhase<S, M, MS, A, Fo>>
    for LocalSearchPhaseFactory<S, M, MS, A, Fo, MSF, AF, FoF>
where
    S: PlanningSolution,
    M: Move<S>,
    D: ScoreDirector<S>,
    MS: MoveSelector<S, M> + Send + Sync,
    A: Acceptor<S> + Send + Sync,
    Fo: LocalSearchForager<S, M> + Send + Sync,
    MSF: Fn() -> MS + Send + Sync,
    AF: Fn() -> A + Send + Sync,
    FoF: Fn() -> Fo + Send + Sync,
{
    fn create_phase(&self) -> LocalSearchPhase<S, M, MS, A, Fo> {
        let move_selector = (self.move_selector_factory)();
        let acceptor = (self.acceptor_factory)();
        let forager = (self.forager_factory)();
        LocalSearchPhase::new(move_selector, acceptor, forager, self.step_limit)
    }
}
