//! Construction heuristic phase.

mod forager;
mod placer;

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

pub use forager::{
    BestFitForager, ConstructionForager, FirstFeasibleForager, FirstFitForager,
    StrongestFitForager, WeakestFitForager,
};
pub use placer::{EntityPlacer, Placement, QueuedEntityPlacer, SortedEntityPlacer};

/// Type of forager to use in construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForagerType {
    FirstFit,
    BestFit,
}

/// Construction heuristic phase configuration.
#[derive(Debug, Clone)]
pub struct ConstructionHeuristicConfig {
    pub forager_type: ForagerType,
}

impl Default for ConstructionHeuristicConfig {
    fn default() -> Self {
        Self {
            forager_type: ForagerType::FirstFit,
        }
    }
}

/// Construction heuristic phase - generic over placer and forager types.
pub struct ConstructionHeuristicPhase<S, M, P, F>
where
    S: PlanningSolution,
    M: Move<S>,
{
    placer: P,
    forager: F,
    _phantom: PhantomData<(S, M)>,
}

impl<S, M, P, F> ConstructionHeuristicPhase<S, M, P, F>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new(placer: P, forager: F) -> Self {
        Self {
            placer,
            forager,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, P, F> Debug for ConstructionHeuristicPhase<S, M, P, F>
where
    S: PlanningSolution,
    M: Move<S>,
    P: Debug,
    F: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstructionHeuristicPhase")
            .field("placer", &self.placer)
            .field("forager", &self.forager)
            .finish()
    }
}

impl<S, M, D, P, F> Phase<S, D> for ConstructionHeuristicPhase<S, M, P, F>
where
    S: PlanningSolution,
    M: Move<S>,
    D: ScoreDirector<S>,
    P: EntityPlacer<S, M, D> + Send,
    F: ConstructionForager<S, M, D> + Send,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let placements = self.placer.get_placements(phase_scope.score_director());

        for placement in placements {
            if phase_scope.solver_scope().is_terminate_early() {
                break;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            let selected_move = self
                .forager
                .pick_move(&placement, step_scope.score_director_mut());

            if let Some(m) = selected_move {
                m.do_move(step_scope.score_director_mut());
                let step_score = step_scope.calculate_score();
                step_scope.set_step_score(step_score);
            }

            step_scope.complete();
        }

        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ConstructionHeuristic"
    }
}
