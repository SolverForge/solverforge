use std::fmt::Debug;
use std::hash::Hash;

use solverforge_config::{ConstructionHeuristicConfig, ConstructionHeuristicType};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::domain::SolutionDescriptor;

use crate::builder::{ListVariableContext, ModelContext};
use crate::descriptor_scalar::{collect_bindings, find_resolved_binding, ResolvedVariableBinding};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConstructionRoute {
    DescriptorScalar,
    GenericMixed,
    SpecializedList,
}

#[derive(Debug, Clone)]
pub(crate) struct ConstructionCapabilities<S, V, DM, IDM> {
    pub(crate) route: ConstructionRoute,
    pub(crate) scalar_bindings: Vec<ResolvedVariableBinding<S>>,
    pub(crate) list_variables: Vec<ListVariableContext<S, V, DM, IDM>>,
    pub(crate) entity_class: Option<String>,
    pub(crate) variable_name: Option<String>,
}

pub(crate) fn select_construction_capabilities<S, V, DM, IDM>(
    config: Option<&ConstructionHeuristicConfig>,
    descriptor: &solverforge_core::domain::SolutionDescriptor,
    model: &ModelContext<S, V, DM, IDM>,
) -> ConstructionCapabilities<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
{
    let heuristic = config
        .map(|cfg| cfg.construction_heuristic_type)
        .unwrap_or(ConstructionHeuristicType::FirstFit);
    let entity_class = config.and_then(|cfg| cfg.target.entity_class.clone());
    let variable_name = config.and_then(|cfg| cfg.target.variable_name.clone());
    let explicit_target = entity_class.is_some() || variable_name.is_some();
    let scalar_bindings = find_resolved_binding(
        &resolve_scalar_bindings(descriptor, model),
        entity_class.as_deref(),
        variable_name.as_deref(),
    );
    let list_variables: Vec<_> = model
        .list_variables()
        .filter(|ctx| {
            !explicit_target
                || ctx.matches_target(entity_class.as_deref(), variable_name.as_deref())
        })
        .cloned()
        .collect();

    if explicit_target && scalar_bindings.is_empty() && list_variables.is_empty() {
        panic!(
            "construction heuristic matched no planning variables for entity_class={:?} variable_name={:?}",
            entity_class,
            variable_name
        );
    }

    let route = match heuristic {
        ConstructionHeuristicType::ListRoundRobin
        | ConstructionHeuristicType::ListCheapestInsertion
        | ConstructionHeuristicType::ListRegretInsertion
        | ConstructionHeuristicType::ListClarkeWright
        | ConstructionHeuristicType::ListKOpt => {
            validate_list_route(heuristic, &list_variables, scalar_bindings.is_empty());
            ConstructionRoute::SpecializedList
        }
        ConstructionHeuristicType::FirstFitDecreasing
        | ConstructionHeuristicType::WeakestFit
        | ConstructionHeuristicType::WeakestFitDecreasing
        | ConstructionHeuristicType::StrongestFit
        | ConstructionHeuristicType::StrongestFitDecreasing
        | ConstructionHeuristicType::AllocateEntityFromQueue
        | ConstructionHeuristicType::AllocateToValueFromQueue => {
            validate_scalar_route(heuristic, &scalar_bindings, list_variables.is_empty());
            ConstructionRoute::DescriptorScalar
        }
        ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::CheapestInsertion => {
            if scalar_bindings.is_empty() && list_variables.is_empty() {
                panic!(
                    "construction heuristic {:?} matched no planning variables",
                    heuristic
                );
            }
            if !scalar_bindings.is_empty() && list_variables.is_empty() {
                ConstructionRoute::DescriptorScalar
            } else {
                ConstructionRoute::GenericMixed
            }
        }
    };

    ConstructionCapabilities {
        route,
        scalar_bindings,
        list_variables,
        entity_class,
        variable_name,
    }
}

fn resolve_scalar_bindings<S, V, DM, IDM>(
    descriptor: &SolutionDescriptor,
    model: &ModelContext<S, V, DM, IDM>,
) -> Vec<ResolvedVariableBinding<S>>
where
    S: PlanningSolution + 'static,
{
    collect_bindings(descriptor)
        .into_iter()
        .map(|binding| {
            let runtime_ctx = model.scalar_variables().find(|ctx| {
                ctx.descriptor_index == binding.descriptor_index
                    && ctx.variable_index == binding.variable_index
            });
            ResolvedVariableBinding::new(binding).with_runtime_construction_hooks(
                runtime_ctx.and_then(|ctx| ctx.construction_entity_order_key),
                runtime_ctx.and_then(|ctx| ctx.construction_value_order_key),
            )
        })
        .collect()
}

fn validate_list_route<S, V, DM, IDM>(
    heuristic: ConstructionHeuristicType,
    list_variables: &[ListVariableContext<S, V, DM, IDM>],
    scalar_target_empty: bool,
) where
    S: PlanningSolution,
{
    if list_variables.is_empty() {
        if scalar_target_empty {
            panic!(
                "list construction heuristic {:?} matched no planning list variables",
                heuristic
            );
        }
        panic!(
            "list construction heuristic {:?} configured against scalar planning variables",
            heuristic
        );
    }

    let mut missing = Vec::new();
    for ctx in list_variables {
        let mut required = Vec::new();
        match heuristic {
            ConstructionHeuristicType::ListClarkeWright => {
                if ctx.cw_depot_fn.is_none() {
                    required.push("cw_depot_fn");
                }
                if ctx.cw_distance_fn.is_none() {
                    required.push("cw_distance_fn");
                }
                if ctx.cw_element_load_fn.is_none() {
                    required.push("cw_element_load_fn");
                }
                if ctx.cw_capacity_fn.is_none() {
                    required.push("cw_capacity_fn");
                }
                if ctx.cw_assign_route_fn.is_none() {
                    required.push("cw_assign_route_fn");
                }
            }
            ConstructionHeuristicType::ListKOpt => {
                if ctx.k_opt_get_route.is_none() {
                    required.push("k_opt_get_route");
                }
                if ctx.k_opt_set_route.is_none() {
                    required.push("k_opt_set_route");
                }
                if ctx.k_opt_depot_fn.is_none() {
                    required.push("k_opt_depot_fn");
                }
                if ctx.k_opt_distance_fn.is_none() {
                    required.push("k_opt_distance_fn");
                }
            }
            _ => {}
        }

        if !required.is_empty() {
            missing.push(format!(
                "  - {}.{} missing {}",
                ctx.entity_type_name,
                ctx.variable_name,
                required.join(", ")
            ));
        }
    }

    if !missing.is_empty() {
        panic!(
            "construction heuristic {:?} requires validated list capabilities:\n{}",
            heuristic,
            missing.join("\n")
        );
    }
}

fn validate_scalar_route<S>(
    heuristic: ConstructionHeuristicType,
    scalar_bindings: &[ResolvedVariableBinding<S>],
    list_target_empty: bool,
) where
    S: PlanningSolution,
{
    if scalar_bindings.is_empty() {
        if list_target_empty {
            panic!(
                "scalar construction heuristic {:?} matched no scalar planning variables",
                heuristic
            );
        }
        panic!(
            "scalar construction heuristic {:?} configured against planning list variables",
            heuristic
        );
    }

    let mut missing = Vec::new();
    for binding in scalar_bindings {
        let mut required = Vec::new();
        if heuristic_requires_entity_order_key(heuristic) && !binding.has_entity_order_key() {
            required.push("construction_entity_order_key");
        }
        if heuristic_requires_value_order_key(heuristic) && !binding.has_value_order_key() {
            required.push("construction_value_order_key");
        }
        if !required.is_empty() {
            missing.push(format!(
                "  - {}.{} missing {}",
                binding.entity_type_name,
                binding.variable_name,
                required.join(", ")
            ));
        }
    }

    if !missing.is_empty() {
        panic!(
            "construction heuristic {:?} requires validated scalar capabilities:\n{}",
            heuristic,
            missing.join("\n")
        );
    }
}

fn heuristic_requires_entity_order_key(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::FirstFitDecreasing
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFitDecreasing
            | ConstructionHeuristicType::AllocateEntityFromQueue
    )
}

fn heuristic_requires_value_order_key(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing
            | ConstructionHeuristicType::AllocateToValueFromQueue
    )
}
