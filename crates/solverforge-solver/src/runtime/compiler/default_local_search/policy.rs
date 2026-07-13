use solverforge_config::SelectionOrder;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::builder::{RuntimeModel, ScalarAccessCapability};

use super::super::defaults::DefaultScalarBinding;
use super::super::graph::CompiledSelectorNode;
use super::super::selector_tree::compile_selector;
use super::super::types::{CompiledListSlot, RuntimeCompileError, RuntimeCompileErrorKind};
use super::{
    DefaultLocalSearchAcceptorPolicy, DefaultLocalSearchComponents,
    DefaultLocalSearchForagerPolicy, DefaultLocalSearchPlan,
};

mod list;
mod scalar;

const DEFAULT_LOCAL_SEARCH_LATE_ACCEPTANCE_SIZE: usize = 400;
const DEFAULT_LOCAL_SEARCH_ACCEPTED_COUNT: usize = 256;
const DEFAULT_SIMULATED_ANNEALING_DECAY_RATE: f64 = 0.999_985;

pub(crate) fn compile_default_local_search_components<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    list_slots: &[CompiledListSlot<S, V, DM, IDM>],
    scalar_slots: &[DefaultScalarBinding<S>],
    random_seed: Option<u64>,
) -> DefaultLocalSearchComponents
where
    S: PlanningSolution,
{
    let has_lists = !list_slots.is_empty();
    let has_groups = !model.scalar_groups().is_empty();
    let has_precedence = list_slots.iter().any(list::supports_precedence_moves);
    let has_nearby_scalar = scalar_slots.iter().any(|binding| {
        !binding.assignment_owned
            && (binding
                .slot
                .has_capability(ScalarAccessCapability::NearbyValue)
                || binding
                    .slot
                    .has_capability(ScalarAccessCapability::NearbyEntity))
    });
    let registry = model.runtime_provider_registry();
    let has_conflict_repairs = !model.conflict_repairs().is_empty()
        || !registry.repairs().is_empty()
        || !registry.static_repairs().is_empty();

    let acceptor = if has_lists {
        DefaultLocalSearchAcceptorPolicy::LateAcceptance {
            history_size: DEFAULT_LOCAL_SEARCH_LATE_ACCEPTANCE_SIZE,
        }
    } else if has_groups {
        DefaultLocalSearchAcceptorPolicy::DiversifiedLateAcceptance {
            history_size: DEFAULT_LOCAL_SEARCH_LATE_ACCEPTANCE_SIZE,
        }
    } else {
        DefaultLocalSearchAcceptorPolicy::SimulatedAnnealing {
            decay_rate_bits: DEFAULT_SIMULATED_ANNEALING_DECAY_RATE.to_bits(),
            random_seed,
        }
    };

    let forager = if has_groups && !has_lists {
        DefaultLocalSearchForagerPolicy::FirstLastStepScoreImproving {
            accepted_count_limit: None,
        }
    } else if has_precedence {
        DefaultLocalSearchForagerPolicy::FirstLastStepScoreImproving {
            accepted_count_limit: Some(DEFAULT_LOCAL_SEARCH_ACCEPTED_COUNT),
        }
    } else {
        DefaultLocalSearchForagerPolicy::AcceptedCount {
            limit: if has_lists || has_nearby_scalar || has_conflict_repairs {
                DEFAULT_LOCAL_SEARCH_ACCEPTED_COUNT
            } else {
                1
            },
        }
    };

    DefaultLocalSearchComponents { acceptor, forager }
}

pub(crate) fn compile_default_local_search_plan<S, V, DM, IDM>(
    descriptor: &SolutionDescriptor,
    model: &RuntimeModel<S, V, DM, IDM>,
    list_slots: &[CompiledListSlot<S, V, DM, IDM>],
    scalar_slots: &[DefaultScalarBinding<S>],
    components: DefaultLocalSearchComponents,
) -> Result<
    Option<(
        DefaultLocalSearchPlan,
        Vec<CompiledSelectorNode<S, V, DM, IDM>>,
    )>,
    RuntimeCompileError,
>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    let mut declarations = Vec::new();

    list::append_list_policy(list_slots, &mut declarations);
    scalar::append_nearby_scalar_policy(scalar_slots, &mut declarations);
    scalar::append_group_policy(model, &mut declarations)?;
    scalar::append_conflict_repair_policy(model, &mut declarations)?;
    scalar::append_ordinary_scalar_policy(scalar_slots, &mut declarations);

    if declarations.is_empty() {
        return Ok(None);
    }
    let selection_order = if declarations.len() == 1 {
        solverforge_config::UnionSelectionOrder::Sequential
    } else {
        solverforge_config::UnionSelectionOrder::StratifiedRandom
    };
    let nodes = declarations
        .iter()
        .enumerate()
        .map(|(index, declaration)| {
            compile_selector(
                &declaration.config,
                SelectionOrder::Random,
                &format!("default_runtime.local_search.selectors[{index}]"),
                descriptor,
                model,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some((
        DefaultLocalSearchPlan {
            components,
            selection_order,
            selectors: declarations,
        },
        nodes,
    )))
}

pub(super) fn default_selector_error(message: impl Into<String>) -> RuntimeCompileError {
    RuntimeCompileError {
        path: "default_runtime.local_search".to_string(),
        kind: RuntimeCompileErrorKind::LocalSearchShape {
            message: message.into(),
        },
    }
}
