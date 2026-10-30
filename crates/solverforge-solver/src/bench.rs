//! Handles the selector-cursor zero-regression benchmark drives through the
//! hidden `__bench` module.
//!
//! Every handle starts from `CompiledRuntimeRunner::compile`, the same
//! compilation, preparation, and lowering a configured solve runs, and then
//! moves existing state out of that runner. No handle rebuilds a selector,
//! placer, or cursor: the benchmark pulls candidates from the cursor types a
//! compiled solve pulls them from.

mod construction;
mod neighborhood;

#[cfg(test)]
mod tests;

use std::fmt::Debug;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::Director;

use crate::builder::search::custom::{NoRuntimeExtensionPhase, NoTypedExtensions};
use crate::builder::SearchContext;
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCandidateRef;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::Phase;
use crate::run::compile_runtime_runner;
use crate::runtime::compiler::executor::CompiledRuntimePhaseRunner;
use crate::runtime_build_error::RuntimeBuildResult;
use crate::scope::{ProgressCallback, SolverScope};
use crate::stats::CandidateTracePhasePlan;

pub use construction::{
    CompiledScalarPlacement, CompiledScalarPlacementCursor, CompiledScalarPlacer,
};
pub use neighborhood::{CompiledNeighborhood, CompiledNeighborhoodCursor};

/// Inspects one live candidate through the public [`Move`] contract, so the
/// benchmark never names the compiled runtime's private move carriers.
pub trait CandidateVisitor<S: PlanningSolution> {
    /// Value the visitor returns.
    type Output;

    /// Visits one live candidate.
    fn visit<M: Move<S>>(self, candidate: MoveCandidateRef<'_, S, M>) -> Self::Output;
}

/// The phase runner a configured solve executes, compiled from a solver
/// configuration and a typed search context.
pub struct CompiledRuntimeRunner<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    runner: CompiledRuntimePhaseRunner<S, V, DM, IDM, NoRuntimeExtensionPhase>,
}

impl<S, V, DM, IDM> CompiledRuntimeRunner<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    /// Compiles, prepares, and lowers `config` over `context` exactly as a
    /// configured solve does.
    pub fn compile(
        config: &SolverConfig,
        context: SearchContext<S, V, DM, IDM>,
    ) -> RuntimeBuildResult<Self> {
        compile_runtime_runner(config, context, NoTypedExtensions).map(|runner| Self { runner })
    }
}

impl<S, V, DM, IDM> CompiledRuntimeRunner<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    /// The first phase's lowered acceptor/forager neighborhood, or `None`
    /// when the first configured phase is not one.
    pub fn into_local_search_neighborhood(self) -> Option<CompiledNeighborhood<S, V, DM, IDM>> {
        self.runner
            .into_first_local_search()
            .map(|(state, resources)| CompiledNeighborhood::new(state, resources))
    }

    /// The first phase's scalar construction placer, or `None` when the
    /// first configured phase is not a descriptor-placement scalar
    /// construction.
    pub fn into_scalar_construction_placer(self) -> Option<CompiledScalarPlacer<S>> {
        self.runner
            .into_first_scalar_construction()
            .map(CompiledScalarPlacer::new)
    }
}

impl<S, V, DM, IDM> Debug for CompiledRuntimeRunner<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledRuntimeRunner")
            .field("runner", &self.runner)
            .finish()
    }
}

impl<S, V, DM, IDM, D, ProgressCb> Phase<S, D, ProgressCb> for CompiledRuntimeRunner<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        Phase::<S, D, ProgressCb>::solve(&mut self.runner, solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        Phase::<S, D, ProgressCb>::phase_type_name(&self.runner)
    }

    fn defers_initial_best_solution_publication(&self) -> bool {
        Phase::<S, D, ProgressCb>::defers_initial_best_solution_publication(&self.runner)
    }

    fn on_solver_terminal(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        Phase::<S, D, ProgressCb>::on_solver_terminal(&mut self.runner, solver_scope);
    }

    fn candidate_trace_plan(&self) -> CandidateTracePhasePlan {
        Phase::<S, D, ProgressCb>::candidate_trace_plan(&self.runner)
    }
}
