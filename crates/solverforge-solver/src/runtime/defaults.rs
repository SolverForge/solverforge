use std::fmt::Debug;
use std::hash::Hash;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation,
    VariableTargetConfig,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::builder::{RuntimeModel, ScalarVariableSlot};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::scope::{ProgressCallback, SolverScope};

use super::Construction;

pub(super) fn solve_default_construction<S, V, DM, IDM, D, ProgressCb>(
    construction: &Construction<S, V, DM, IDM>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut ran_child_phase = false;

    for config in list_construction_configs(&construction.model, solver_scope.working_solution()) {
        ran_child_phase |= construction.solve_configured(Some(&config), solver_scope);
    }

    for group_index in assignment_group_indices(&construction.model) {
        let config = {
            let group = &construction.model.scalar_groups()[group_index];
            let Some(assignment) = group.assignment() else {
                continue;
            };
            (assignment.remaining_required_count(solver_scope.working_solution()) > 0).then(|| {
                assignment_group_config(
                    group.group_name,
                    ConstructionObligation::AssignWhenCandidateExists,
                )
            })
        };
        if let Some(config) = config {
            ran_child_phase |= construction.solve_configured(Some(&config), solver_scope);
        }
    }

    for group_index in assignment_group_indices(&construction.model) {
        let config = {
            let group = &construction.model.scalar_groups()[group_index];
            let Some(assignment) = group.assignment() else {
                continue;
            };
            (assignment.unassigned_count(solver_scope.working_solution()) > 0).then(|| {
                assignment_group_config(
                    group.group_name,
                    ConstructionObligation::PreserveUnassigned,
                )
            })
        };
        if let Some(config) = config {
            ran_child_phase |= construction.solve_configured(Some(&config), solver_scope);
        }
    }

    for config in descriptor_scalar_configs(&construction.model) {
        ran_child_phase |= construction.solve_configured(Some(&config), solver_scope);
    }

    for config in list_k_opt_configs(&construction.model, solver_scope.working_solution()) {
        ran_child_phase |= construction.solve_configured(Some(&config), solver_scope);
    }

    ran_child_phase
}

fn list_construction_configs<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    solution: &S,
) -> Vec<ConstructionHeuristicConfig> {
    model
        .list_variables()
        .filter(|variable| variable.has_unassigned_elements(solution))
        .map(|variable| {
            let construction_heuristic_type = if variable.supports_clarke_wright() {
                ConstructionHeuristicType::ListClarkeWright
            } else {
                ConstructionHeuristicType::ListCheapestInsertion
            };
            list_config(
                construction_heuristic_type,
                variable.entity_type_name,
                variable.variable_name,
            )
        })
        .collect()
}

fn list_k_opt_configs<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    solution: &S,
) -> Vec<ConstructionHeuristicConfig> {
    model
        .list_variables()
        .filter(|variable| variable.supports_k_opt() && variable.has_list_content(solution))
        .map(|variable| {
            list_config(
                ConstructionHeuristicType::ListKOpt,
                variable.entity_type_name,
                variable.variable_name,
            )
        })
        .collect()
}

fn descriptor_scalar_configs<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Vec<ConstructionHeuristicConfig> {
    model
        .scalar_variables()
        .filter(|variable| !assignment_group_covers_variable(model, variable))
        .map(|variable| {
            let target = VariableTargetConfig {
                entity_class: Some(variable.entity_type_name.to_string()),
                variable_name: Some(variable.variable_name.to_string()),
            };
            ConstructionHeuristicConfig {
                construction_heuristic_type: ConstructionHeuristicType::FirstFit,
                target,
                ..ConstructionHeuristicConfig::default()
            }
        })
        .collect()
}

fn assignment_group_covers_variable<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    variable: &ScalarVariableSlot<S>,
) -> bool {
    model.assignment_scalar_groups().any(|(_, group)| {
        group.members.iter().any(|member| {
            member.descriptor_index == variable.descriptor_index
                && member.variable_index == variable.variable_index
        })
    })
}

fn assignment_group_indices<S, V, DM, IDM>(model: &RuntimeModel<S, V, DM, IDM>) -> Vec<usize> {
    model
        .assignment_scalar_groups()
        .map(|(index, _)| index)
        .collect()
}

fn assignment_group_config(
    group_name: &'static str,
    construction_obligation: ConstructionObligation,
) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::CheapestInsertion,
        construction_obligation,
        group_name: Some(group_name.to_string()),
        ..ConstructionHeuristicConfig::default()
    }
}

fn list_config(
    construction_heuristic_type: ConstructionHeuristicType,
    entity_class: &'static str,
    variable_name: &'static str,
) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type,
        target: VariableTargetConfig {
            entity_class: Some(entity_class.to_string()),
            variable_name: Some(variable_name.to_string()),
        },
        ..ConstructionHeuristicConfig::default()
    }
}
