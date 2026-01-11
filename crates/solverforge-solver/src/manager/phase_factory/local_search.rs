//! Local search phase factory with zero type erasure.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::{Move, MoveSelector};
use crate::phase::localsearch::{
    AcceptedCountForager, Acceptor, HillClimbingAcceptor, LateAcceptanceAcceptor,
    LocalSearchForager, LocalSearchPhase, SimulatedAnnealingAcceptor, TabuSearchAcceptor,
};

use super::super::PhaseFactory;

/// Zero-erasure factory for local search phases.
///
/// All types flow through generics - MoveSelector `MS`, Acceptor `A`,
/// and Forager `Fo` are all concrete types.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `M` - The move type
/// * `MS` - The move selector type (concrete)
/// * `A` - The acceptor type (concrete)
/// * `Fo` - The forager type (concrete)
pub struct LocalSearchPhaseFactory<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    move_selector: MS,
    acceptor: A,
    forager: Fo,
    step_limit: Option<u64>,
    _marker: PhantomData<fn() -> (S, M)>,
}

impl<S, M, MS, A, Fo> LocalSearchPhaseFactory<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    /// Creates a new factory with concrete components.
    pub fn new(move_selector: MS, acceptor: A, forager: Fo) -> Self {
        Self {
            move_selector,
            acceptor,
            forager,
            step_limit: None,
            _marker: PhantomData,
        }
    }

    /// Sets step limit.
    pub fn with_step_limit(mut self, limit: u64) -> Self {
        self.step_limit = Some(limit);
        self
    }
}

// Convenience constructors with specific acceptors

impl<S, M, MS> LocalSearchPhaseFactory<S, M, MS, HillClimbingAcceptor, AcceptedCountForager<S>>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M>,
{
    /// Creates a hill climbing local search.
    pub fn hill_climbing(move_selector: MS) -> Self {
        Self::new(
            move_selector,
            HillClimbingAcceptor::new(),
            AcceptedCountForager::new(1),
        )
    }
}

impl<S, M, MS> LocalSearchPhaseFactory<S, M, MS, TabuSearchAcceptor<S>, AcceptedCountForager<S>>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M>,
{
    /// Creates a tabu search local search.
    pub fn tabu_search(move_selector: MS, tabu_size: usize) -> Self {
        Self::new(
            move_selector,
            TabuSearchAcceptor::new(tabu_size),
            AcceptedCountForager::new(1),
        )
    }
}

impl<S, M, MS>
    LocalSearchPhaseFactory<S, M, MS, SimulatedAnnealingAcceptor, AcceptedCountForager<S>>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M>,
{
    /// Creates a simulated annealing local search.
    pub fn simulated_annealing(
        move_selector: MS,
        starting_temp: f64,
        decay_rate: f64,
    ) -> Self {
        Self::new(
            move_selector,
            SimulatedAnnealingAcceptor::new(starting_temp, decay_rate),
            AcceptedCountForager::new(1),
        )
    }
}

impl<S, M, MS>
    LocalSearchPhaseFactory<S, M, MS, LateAcceptanceAcceptor<S>, AcceptedCountForager<S>>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M>,
{
    /// Creates a late acceptance local search.
    pub fn late_acceptance(move_selector: MS, size: usize) -> Self {
        Self::new(
            move_selector,
            LateAcceptanceAcceptor::new(size),
            AcceptedCountForager::new(1),
        )
    }
}

impl<S, D, M, MS, A, Fo> PhaseFactory<S, D> for LocalSearchPhaseFactory<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S> + Send + Sync + 'static,
    MS: MoveSelector<S, M> + Clone + Send + Sync + 'static,
    A: Acceptor<S> + Clone + Send + Sync + 'static,
    Fo: LocalSearchForager<S, M> + Clone + Send + Sync + 'static,
{
    type Phase = LocalSearchPhase<S, M, MS, A, Fo>;

    fn create(&self) -> Self::Phase {
        LocalSearchPhase::new(
            self.move_selector.clone(),
            self.acceptor.clone(),
            self.forager.clone(),
            self.step_limit,
        )
    }
}
