//! Construction phase factory.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::Move;
use crate::phase::construction::{
    ConstructionForager, ConstructionHeuristicPhase, EntityPlacer, FirstFitForager,
};
use crate::phase::Phase;

use super::super::SolverPhaseFactory;

/// Factory for creating construction heuristic phases.
///
/// Construction heuristic phases build an initial solution by assigning
/// values to uninitialized planning variables. The factory provides
/// fresh phase instances for each solve.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `M` - The move type
/// * `P` - The placer type
/// * `Fo` - The forager type
/// * `PF` - The placer factory closure type
/// * `FF` - The forager factory closure type
pub struct ConstructionPhaseFactory<S, M, P, Fo, PF, FF>
where
    S: PlanningSolution,
    M: Move<S>,
{
    placer_factory: PF,
    forager_factory: FF,
    _marker: PhantomData<(S, M, P, Fo)>,
}

impl<S, M, P, Fo, PF, FF> ConstructionPhaseFactory<S, M, P, Fo, PF, FF>
where
    S: PlanningSolution,
    M: Move<S>,
    PF: Fn() -> P + Send + Sync,
    FF: Fn() -> Fo + Send + Sync,
{
    /// Creates a new construction phase factory with custom placer and forager factories.
    pub fn new(placer_factory: PF, forager_factory: FF) -> Self {
        Self {
            placer_factory,
            forager_factory,
            _marker: PhantomData,
        }
    }
}

impl<S, M, P, PF> ConstructionPhaseFactory<S, M, P, FirstFitForager<S, M>, PF, fn() -> FirstFitForager<S, M>>
where
    S: PlanningSolution,
    M: Move<S>,
    PF: Fn() -> P + Send + Sync,
{
    /// Creates a factory with FirstFit forager.
    ///
    /// FirstFit accepts the first valid assignment for each entity,
    /// making it fast but potentially producing lower quality initial solutions.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::ConstructionPhaseFactory;
    /// use solverforge_solver::phase::construction::QueuedEntityPlacer;
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::{FromSolutionEntitySelector, StaticTypedValueSelector};
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
    /// type ES = FromSolutionEntitySelector;
    /// type VS = StaticTypedValueSelector<Sol, i32>;
    /// type Placer = QueuedEntityPlacer<Sol, i32, ES, VS>;
    /// type Move = ChangeMove<Sol, i32>;
    ///
    /// let factory = ConstructionPhaseFactory::<Sol, Move, Placer, _, _, _>::first_fit(|| {
    ///     QueuedEntityPlacer::new(
    ///         FromSolutionEntitySelector::new(0),
    ///         StaticTypedValueSelector::new(vec![1, 2, 3]),
    ///         get_v, set_v, 0, "v",
    ///     )
    /// });
    /// ```
    pub fn first_fit(placer_factory: PF) -> Self {
        Self {
            placer_factory,
            forager_factory: FirstFitForager::new,
            _marker: PhantomData,
        }
    }
}

impl<S, M, D, P, Fo, PF, FF> SolverPhaseFactory<S, D, ConstructionHeuristicPhase<S, M, P, Fo>>
    for ConstructionPhaseFactory<S, M, P, Fo, PF, FF>
where
    S: PlanningSolution,
    M: Move<S>,
    D: ScoreDirector<S>,
    P: EntityPlacer<S, M, D> + Send,
    Fo: ConstructionForager<S, M, D> + Send,
    PF: Fn() -> P + Send + Sync,
    FF: Fn() -> Fo + Send + Sync,
{
    fn create_phase(&self) -> ConstructionHeuristicPhase<S, M, P, Fo> {
        let placer = (self.placer_factory)();
        let forager = (self.forager_factory)();
        ConstructionHeuristicPhase::new(placer, forager)
    }
}
