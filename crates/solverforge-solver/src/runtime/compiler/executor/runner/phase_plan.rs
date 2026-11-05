//! A runner phase's position and its candidate-trace plan with the outcome
//! it reached.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::stats::CandidateTracePhasePlan;

use super::super::trace::{default_construction_plan, phase_with_outcome};
use super::RunnerPhase;

impl<S, V, DM, IDM> RunnerPhase<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    pub(super) fn phase_index(&self) -> usize {
        match self {
            Self::Construction { phase_index, .. }
            | Self::LocalSearch { phase_index, .. }
            | Self::Extension { phase_index, .. }
            | Self::DefaultRuntime { phase_index, .. } => *phase_index,
        }
    }

    pub(super) fn candidate_trace_plan(&self) -> CandidateTracePhasePlan {
        match self {
            Self::Construction {
                phase_index,
                declaration,
                outcome,
            }
            | Self::LocalSearch {
                phase_index,
                declaration,
                outcome,
                ..
            }
            | Self::Extension {
                phase_index,
                declaration,
                outcome,
            } => phase_with_outcome(*phase_index, outcome.trace_label(), declaration.clone()),
            Self::DefaultRuntime {
                phase_index,
                local_search_declaration,
                construction_execution,
                outcome,
                ..
            } => {
                let mut children = construction_execution
                    .as_ref()
                    .map(default_construction_plan)
                    .into_iter()
                    .collect::<Vec<_>>();
                if let Some(local_search) = local_search_declaration {
                    children.push(local_search.clone());
                }
                CandidateTracePhasePlan::known(
                    "solverforge.runtime.default_runtime",
                    [
                        ("outcome", outcome.trace_label().to_string()),
                        ("phase_index", phase_index.to_string()),
                    ],
                    children,
                )
            }
        }
    }
}
