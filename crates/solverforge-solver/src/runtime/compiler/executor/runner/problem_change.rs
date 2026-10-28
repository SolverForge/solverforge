//! Per-pass runner state reset after applied real-time problem changes.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use solverforge_scoring::Director;

use crate::builder::CustomSearchPhase;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::scope::{ProgressCallback, SolverScope};

use super::super::completion::publish_if_mandatory_complete;
use super::super::PreparedRuntimePhase;
use super::failure::panic_runtime_execution_error;
use super::{CompiledRuntimePhaseRunner, RunnerPhase, RunnerPhaseOutcome};

impl<S, V, DM, IDM, Extension> CompiledRuntimePhaseRunner<S, V, DM, IDM, Extension>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    Extension: CustomSearchPhase<S>,
{
    /// Runs the phase sequence, then applies any queued problem changes and
    /// runs the whole sequence again on the changed solution. The queue is
    /// drained or closed atomically after each pass, so a change is either
    /// applied by a later pass or rejected at submission.
    pub(super) fn solve_passes<D, ProgressCb>(
        &mut self,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    ) where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        loop {
            publish_if_mandatory_complete(
                &mut self.execution,
                &self.mandatory_bindings,
                &mut self.mandatory_completion_published,
                0,
                solver_scope,
            )
            .unwrap_or_else(|error| panic_runtime_execution_error(error));
            for index in 0..self.phases.len() {
                if solver_scope.should_terminate() {
                    break;
                }
                self.solve_phase(index, solver_scope);
            }
            let Some(changes) = solver_scope.take_problem_changes() else {
                break;
            };
            solver_scope
                .apply_problem_changes(changes)
                .unwrap_or_else(|error| panic_runtime_execution_error(error));
            self.reset_for_problem_change();
        }
    }

    /// Forgets everything a pass derived from the previous problem: the
    /// completion publication, bound list sources (elements may have been
    /// added or removed), recorded phase outcomes, retained selector stream
    /// state, and extension state.
    fn reset_for_problem_change(&mut self) {
        self.mandatory_completion_published = false;
        self.execution.reset_list_source_bindings();
        for (prepared, phase) in self.prepared_phases.iter_mut().zip(&mut self.phases) {
            if let PreparedRuntimePhase::Extension(extension) = prepared {
                extension.on_problem_change();
            }
            phase.reset_for_problem_change();
        }
        self.problem_change_passes += 1;
    }
}

impl<S, V, DM, IDM> RunnerPhase<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    fn reset_for_problem_change(&mut self) {
        match self {
            Self::Construction { outcome, .. } | Self::Extension { outcome, .. } => {
                *outcome = RunnerPhaseOutcome::Pending;
            }
            Self::LocalSearch {
                local_search,
                outcome,
                ..
            } => {
                local_search.reset_stream_state();
                *outcome = RunnerPhaseOutcome::Pending;
            }
            Self::DefaultRuntime {
                local_search,
                construction_execution,
                outcome,
                ..
            } => {
                if let Some(local_search) = local_search {
                    local_search.reset_stream_state();
                }
                *construction_execution = None;
                *outcome = RunnerPhaseOutcome::Pending;
            }
        }
    }
}
