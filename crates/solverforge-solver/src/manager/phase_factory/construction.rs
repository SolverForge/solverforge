//! Construction phase factory with zero type erasure.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::Move;
use crate::phase::construction::{
    BestFitForager, ConstructionForager, ConstructionHeuristicPhase, EntityPlacer, FirstFitForager,
};

use super::super::PhaseFactory;

/// Zero-erasure factory for construction heuristic phases.
///
/// All types flow through generics - Placer `P` and Forager `Fo` are concrete.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `M` - The move type
/// * `P` - The entity placer type (concrete)
/// * `Fo` - The forager type (concrete)
pub struct ConstructionPhaseFactory<S, D, M, P, Fo>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    placer: P,
    forager: Fo,
    _marker: PhantomData<fn(S, D, M)>,
}

impl<S, D, M, P, Fo> ConstructionPhaseFactory<S, D, M, P, Fo>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    /// Creates a new factory with concrete placer and forager.
    pub fn new(placer: P, forager: Fo) -> Self {
        Self {
            placer,
            forager,
            _marker: PhantomData,
        }
    }
}

impl<S, D, M, P> ConstructionPhaseFactory<S, D, M, P, FirstFitForager<S, M>>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S>,
    P: EntityPlacer<S, M>,
{
    /// Creates a factory with FirstFit forager.
    pub fn first_fit(placer: P) -> Self {
        Self::new(placer, FirstFitForager::new())
    }
}

impl<S, D, M, P> ConstructionPhaseFactory<S, D, M, P, BestFitForager<S, M>>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S>,
    P: EntityPlacer<S, M>,
{
    /// Creates a factory with BestFit forager.
    pub fn best_fit(placer: P) -> Self {
        Self::new(placer, BestFitForager::new())
    }
}

impl<S, D, M, P, Fo> PhaseFactory<S, D> for ConstructionPhaseFactory<S, D, M, P, Fo>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S> + Clone + Send + Sync + 'static,
    P: EntityPlacer<S, M> + Clone + Send + Sync + 'static,
    Fo: ConstructionForager<S, M> + Clone + Send + Sync + 'static,
{
    type Phase = ConstructionHeuristicPhase<S, M, P, Fo>;

    fn create(&self) -> Self::Phase {
        ConstructionHeuristicPhase::new(self.placer.clone(), self.forager.clone())
    }
}
