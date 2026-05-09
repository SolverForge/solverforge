use std::fmt::Debug;

use solverforge_config::{
    AcceptedCountForagerConfig, ChangeMoveConfig, CompoundConflictRepairMoveSelectorConfig,
    ForagerConfig, GroupedScalarMoveSelectorConfig, KOptMoveSelectorConfig, ListReverseMoveConfig,
    ListRuinMoveSelectorConfig, MoveSelectorConfig, NearbyChangeMoveConfig,
    NearbyListChangeMoveConfig, NearbyListSwapMoveConfig, NearbySwapMoveConfig,
    SublistChangeMoveConfig, SublistSwapMoveConfig, UnionMoveSelectorConfig, UnionSelectionOrder,
    VariableTargetConfig,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use crate::builder::acceptor::AnyAcceptor;
use crate::builder::forager::{AnyForager, ForagerBuilder};
use crate::builder::{LocalSearchStrategy, RuntimeModel};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::localsearch::{LateAcceptanceAcceptor, SimulatedAnnealingAcceptor};

const DEFAULT_SCALAR_NEARBY_LIMIT: usize = 10;
const DEFAULT_LIST_NEARBY_LIMIT: usize = 20;
const DEFAULT_LOCAL_SEARCH_LATE_ACCEPTANCE_SIZE: usize = 400;
const DEFAULT_LOCAL_SEARCH_ACCEPTED_COUNT: usize = 128;
const DEFAULT_SIMULATED_ANNEALING_DECAY_RATE: f64 = 0.999985;

fn scalar_target<S>(variable: &crate::builder::ScalarVariableSlot<S>) -> VariableTargetConfig {
    VariableTargetConfig {
        entity_class: Some(variable.entity_type_name.to_string()),
        variable_name: Some(variable.variable_name.to_string()),
    }
}

fn default_scalar_change_selector<S>(
    variable: &crate::builder::ScalarVariableSlot<S>,
) -> MoveSelectorConfig {
    MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
        value_candidate_limit: None,
        target: scalar_target(variable),
    })
}

fn default_scalar_swap_selector<S>(
    variable: &crate::builder::ScalarVariableSlot<S>,
) -> MoveSelectorConfig {
    MoveSelectorConfig::SwapMoveSelector(solverforge_config::SwapMoveConfig {
        target: scalar_target(variable),
    })
}

fn default_nearby_scalar_change_selector<S>(
    variable: &crate::builder::ScalarVariableSlot<S>,
) -> MoveSelectorConfig {
    MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
        max_nearby: DEFAULT_SCALAR_NEARBY_LIMIT,
        value_candidate_limit: None,
        target: scalar_target(variable),
    })
}

fn default_nearby_scalar_swap_selector<S>(
    variable: &crate::builder::ScalarVariableSlot<S>,
) -> MoveSelectorConfig {
    MoveSelectorConfig::NearbySwapMoveSelector(NearbySwapMoveConfig {
        max_nearby: DEFAULT_SCALAR_NEARBY_LIMIT,
        target: scalar_target(variable),
    })
}

fn default_nearby_list_change_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::NearbyListChangeMoveSelector(NearbyListChangeMoveConfig {
        max_nearby: DEFAULT_LIST_NEARBY_LIMIT,
        target: VariableTargetConfig::default(),
    })
}

fn default_nearby_list_swap_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::NearbyListSwapMoveSelector(NearbyListSwapMoveConfig {
        max_nearby: DEFAULT_LIST_NEARBY_LIMIT,
        target: VariableTargetConfig::default(),
    })
}

fn default_sublist_change_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::SublistChangeMoveSelector(SublistChangeMoveConfig::default())
}

fn default_sublist_swap_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::SublistSwapMoveSelector(SublistSwapMoveConfig::default())
}

fn default_list_reverse_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
        target: VariableTargetConfig::default(),
    })
}

fn default_list_k_opt_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::KOptMoveSelector(KOptMoveSelectorConfig {
        max_nearby: DEFAULT_LIST_NEARBY_LIMIT,
        ..KOptMoveSelectorConfig::default()
    })
}

fn default_list_ruin_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::ListRuinMoveSelector(ListRuinMoveSelectorConfig::default())
}

fn default_grouped_scalar_selector<S>(
    group: &crate::builder::ScalarGroupBinding<S>,
) -> MoveSelectorConfig {
    MoveSelectorConfig::GroupedScalarMoveSelector(GroupedScalarMoveSelectorConfig {
        group_name: group.group_name.to_string(),
        value_candidate_limit: None,
        max_moves_per_step: group.default_max_moves_per_step(),
        require_hard_improvement: false,
    })
}

fn default_compound_conflict_repair_selector<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
) -> MoveSelectorConfig {
    let mut constraints = model
        .conflict_repairs()
        .iter()
        .map(|repair| repair.constraint_name().to_string())
        .collect::<Vec<_>>();
    constraints.sort();
    constraints.dedup();
    MoveSelectorConfig::CompoundConflictRepairMoveSelector(
        CompoundConflictRepairMoveSelectorConfig {
            constraints,
            ..CompoundConflictRepairMoveSelectorConfig::default()
        },
    )
}

fn collapse_selectors(selectors: &mut Vec<MoveSelectorConfig>) -> Option<MoveSelectorConfig> {
    match selectors.len() {
        0 => None,
        1 => selectors.pop(),
        _ => Some(MoveSelectorConfig::UnionMoveSelector(
            UnionMoveSelectorConfig {
                selection_order: UnionSelectionOrder::StratifiedRandom,
                selectors: std::mem::take(selectors),
            },
        )),
    }
}

fn append_scalar_selectors<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    selectors: &mut Vec<MoveSelectorConfig>,
) where
    S: PlanningSolution,
{
    for variable in model
        .scalar_variables()
        .filter(|variable| variable.supports_nearby_change())
    {
        selectors.push(default_nearby_scalar_change_selector(variable));
    }

    for variable in model
        .scalar_variables()
        .filter(|variable| variable.supports_nearby_swap())
    {
        selectors.push(default_nearby_scalar_swap_selector(variable));
    }

    for group in model.scalar_groups() {
        selectors.push(default_grouped_scalar_selector(group));
    }

    if model.has_conflict_repairs() {
        selectors.push(default_compound_conflict_repair_selector(model));
    }

    for variable in model.scalar_variables() {
        selectors.push(default_scalar_change_selector(variable));
    }

    for variable in model.scalar_variables() {
        selectors.push(default_scalar_swap_selector(variable));
    }
}

pub(crate) fn default_move_selector_config<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Option<MoveSelectorConfig>
where
    S: PlanningSolution,
{
    let mut selectors = Vec::new();

    if model.has_list_variables() {
        selectors.push(default_nearby_list_change_selector());
        selectors.push(default_nearby_list_swap_selector());
        selectors.push(default_sublist_change_selector());
        selectors.push(default_sublist_swap_selector());
        selectors.push(default_list_reverse_selector());
    }

    if model.has_k_opt_variables() {
        selectors.push(default_list_k_opt_selector());
    }

    if model.has_list_ruin_variables() {
        selectors.push(default_list_ruin_selector());
    }

    append_scalar_selectors(model, &mut selectors);
    collapse_selectors(&mut selectors)
}

pub(crate) fn default_local_search_acceptor<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> AnyAcceptor<S>
where
    S: PlanningSolution,
{
    if model.has_list_variables() {
        return AnyAcceptor::LateAcceptance(LateAcceptanceAcceptor::<S>::new(
            DEFAULT_LOCAL_SEARCH_LATE_ACCEPTANCE_SIZE,
        ));
    }

    match random_seed {
        Some(seed) => {
            AnyAcceptor::SimulatedAnnealing(SimulatedAnnealingAcceptor::auto_calibrate_with_seed(
                DEFAULT_SIMULATED_ANNEALING_DECAY_RATE,
                seed,
            ))
        }
        None => AnyAcceptor::SimulatedAnnealing(SimulatedAnnealingAcceptor::default()),
    }
}

pub(crate) fn default_local_search_forager<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
) -> AnyForager<S>
where
    S: PlanningSolution,
{
    let limit = if model.has_list_variables()
        || model.has_nearby_scalar_change_variables()
        || model.has_nearby_scalar_swap_variables()
        || model.has_scalar_groups()
        || model.has_conflict_repairs()
    {
        DEFAULT_LOCAL_SEARCH_ACCEPTED_COUNT
    } else {
        1
    };

    ForagerBuilder::build(Some(&ForagerConfig::AcceptedCount(
        AcceptedCountForagerConfig { limit: Some(limit) },
    )))
}

pub(crate) fn default_local_search_phases<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> Vec<LocalSearchStrategy<S, V, DM, IDM>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    vec![crate::builder::build_local_search(None, model, random_seed)]
}
