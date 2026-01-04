//! Construction phase factory.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::Move;
use crate::phase::construction::{
    BestFitForager, ConstructionForager, ConstructionHeuristicPhase, EntityPlacer, FirstFitForager,
    ForagerType,
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
/// * `M` - The move type (typically [`ChangeMove`](crate::heuristic::ChangeMove))
/// * `F` - The closure type that creates entity placers
///
/// # Example
///
/// ```
/// use solverforge_solver::manager::{ConstructionPhaseFactory, SolverPhaseFactory};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_solver::heuristic::selector::{
///     FromSolutionEntitySelector, StaticTypedValueSelector,
/// };
/// use solverforge_solver::phase::construction::QueuedEntityPlacer;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Sol { values: Vec<Option<i32>>, score: Option<SimpleScore> }
/// impl PlanningSolution for Sol {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_v(s: &Sol, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
/// fn set_v(s: &mut Sol, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
///
/// type M = ChangeMove<Sol, i32>;
///
/// // Create a first-fit construction phase factory
/// let factory = ConstructionPhaseFactory::<Sol, M, _>::first_fit(|| {
///     let entity_sel = Box::new(FromSolutionEntitySelector::new(0));
///     let value_sel = Box::new(StaticTypedValueSelector::new(vec![1, 2, 3]));
///     Box::new(QueuedEntityPlacer::new(entity_sel, value_sel, get_v, set_v, 0, "value"))
/// });
///
/// // Create phase when solving
/// let phase = factory.create_phase();
/// assert_eq!(phase.phase_type_name(), "ConstructionHeuristic");
/// ```
///
/// # Forager Types
///
/// - [`ForagerType::FirstFit`](crate::phase::construction::ForagerType::FirstFit):
///   Accepts the first valid assignment (fast)
/// - [`ForagerType::BestFit`](crate::phase::construction::ForagerType::BestFit):
///   Evaluates all options and picks the best (better quality)
pub struct ConstructionPhaseFactory<S, M, F>
where
    S: PlanningSolution,
    M: Move<S> + Clone + Send + Sync + 'static,
    F: Fn() -> Box<dyn EntityPlacer<S, M>> + Send + Sync,
{
    forager_type: ForagerType,
    placer_factory: F,
    _marker: PhantomData<(S, M)>,
}

impl<S, M, F> ConstructionPhaseFactory<S, M, F>
where
    S: PlanningSolution,
    M: Move<S> + Clone + Send + Sync + 'static,
    F: Fn() -> Box<dyn EntityPlacer<S, M>> + Send + Sync,
{
    /// Creates a new construction phase factory with the specified forager type.
    ///
    /// # Arguments
    ///
    /// * `forager_type` - The type of forager to use (FirstFit or BestFit)
    /// * `placer_factory` - A closure that creates entity placers
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::ConstructionPhaseFactory;
    /// use solverforge_solver::phase::construction::{ForagerType, QueuedEntityPlacer};
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::{FromSolutionEntitySelector, StaticTypedValueSelector};
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// let factory = ConstructionPhaseFactory::<S, M, _>::new(ForagerType::BestFit, || {
    ///     let es = Box::new(FromSolutionEntitySelector::new(0));
    ///     let vs = Box::new(StaticTypedValueSelector::new(vec![1, 2]));
    ///     Box::new(QueuedEntityPlacer::new(es, vs, get_v, set_v, 0, "v"))
    /// });
    /// ```
    pub fn new(forager_type: ForagerType, placer_factory: F) -> Self {
        Self {
            forager_type,
            placer_factory,
            _marker: PhantomData,
        }
    }

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
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// let factory = ConstructionPhaseFactory::<S, M, _>::first_fit(|| {
    ///     let es = Box::new(FromSolutionEntitySelector::new(0));
    ///     let vs = Box::new(StaticTypedValueSelector::new(vec![1, 2, 3]));
    ///     Box::new(QueuedEntityPlacer::new(es, vs, get_v, set_v, 0, "v"))
    /// });
    /// ```
    pub fn first_fit(placer_factory: F) -> Self {
        Self::new(ForagerType::FirstFit, placer_factory)
    }

    /// Creates a factory with BestFit forager.
    ///
    /// BestFit evaluates all possible values for each entity and selects
    /// the one that produces the best score. Slower but produces better
    /// initial solutions.
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
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// let factory = ConstructionPhaseFactory::<S, M, _>::best_fit(|| {
    ///     let es = Box::new(FromSolutionEntitySelector::new(0));
    ///     let vs = Box::new(StaticTypedValueSelector::new(vec![1, 2, 3]));
    ///     Box::new(QueuedEntityPlacer::new(es, vs, get_v, set_v, 0, "v"))
    /// });
    /// ```
    pub fn best_fit(placer_factory: F) -> Self {
        Self::new(ForagerType::BestFit, placer_factory)
    }
}

impl<S, M, F> SolverPhaseFactory<S> for ConstructionPhaseFactory<S, M, F>
where
    S: PlanningSolution + 'static,
    M: Move<S> + Clone + Send + Sync + 'static,
    F: Fn() -> Box<dyn EntityPlacer<S, M>> + Send + Sync,
{
    fn create_phase(&self) -> Box<dyn Phase<S>> {
        let placer = (self.placer_factory)();

        let forager: Box<dyn ConstructionForager<S, M>> = match self.forager_type {
            ForagerType::FirstFit => Box::new(FirstFitForager::new()),
            ForagerType::BestFit => Box::new(BestFitForager::new()),
        };

        Box::new(ConstructionHeuristicPhase::new(placer, forager))
    }
}
