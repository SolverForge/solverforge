//! A runner phase's position and its candidate-trace plan with the outcome
//! it reached.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::stats::CandidateTracePhasePlan;
use crate::CandidateTracePhaseAttribute;

use super::super::trace::{
    default_construction_plan, effective_accepted_count_limit, phase_with_outcome,
};
use super::local_search::RuntimeLocalSearch;
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
            Self::LocalSearch {
                phase_index,
                declaration,
                local_search,
                outcome,
            } => {
                let mut declaration = declaration.clone();
                insert_attribute(
                    &mut declaration,
                    "forager_accepted_count_limit",
                    accepted_count_limit(local_search),
                );
                phase_with_outcome(*phase_index, outcome.trace_label(), declaration)
            }
            Self::Construction {
                phase_index,
                declaration,
                outcome,
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

/// The accepted-count horizon a lowered local-search phase runs with; a
/// descent phase has no forager and reports `none`.
fn accepted_count_limit<S, V, DM, IDM>(local_search: &RuntimeLocalSearch<S, V, DM, IDM>) -> String
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    match local_search {
        RuntimeLocalSearch::AcceptorForager {
            components,
            acceptor,
            forager,
            ..
        } => effective_accepted_count_limit(
            acceptor
                .as_ref()
                .is_some_and(|acceptor| acceptor.is_tabu_search()),
            forager.as_ref().map(|forager| forager.config()),
            *components,
        ),
        RuntimeLocalSearch::VariableNeighborhoodDescent { .. } => "none".to_string(),
    }
}

/// Adds `key` to a plan node, keeping its attributes sorted and unique.
fn insert_attribute(plan: &mut CandidateTracePhasePlan, key: &str, value: String) {
    let position = plan
        .attributes
        .binary_search_by(|attribute| attribute.key.as_str().cmp(key))
        .expect_err("a resolved attribute is added once");
    plan.attributes.insert(
        position,
        CandidateTracePhaseAttribute {
            key: key.to_string(),
            value,
        },
    );
}
