//! Immutable bindings and per-solve expansion for omitted runtime phases.
//!
//! The compiler freezes every potentially-defaulted slot once. A later
//! instantiation resolves only solution-state predicates (unassigned list
//! elements, required assignment count, and route content), never schema
//! discovery or a host-language construction implementation.

use solverforge_config::{SolverConfig, VariableTargetConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::ParseableScore;

use crate::builder::{RuntimeModel, RuntimeScalarSlot, ScalarGroupBinding, VariableSlot};
use crate::phase::construction::ScalarConstructionSchedule;

use super::default_local_search::{
    compile_default_local_search_components, compile_default_local_search_plan,
    DefaultLocalSearchComponents, DefaultLocalSearchPlan,
};
use super::slots::{matching_list_slots, resolved_scalar_bindings};
use super::types::{CompiledListSlot, CompiledScalarSlot, RuntimeCompileError};

mod stages;

pub(crate) use stages::{
    resolve_default_postconstruction_kopt, resolve_default_preconstruction_stage,
};
pub(crate) use stages::{
    DefaultConstructionStage, DefaultConstructionStepKind, DefaultListPolicyProvenance,
    DefaultPreconstructionStage, ResolvedDefaultConstructionPlan,
};

/// One scalar slot frozen for default construction/local-search expansion.
#[derive(Clone, Debug)]
pub(crate) struct DefaultScalarBinding<S> {
    pub slot: CompiledScalarSlot<S>,
    pub assignment_owned: bool,
    /// Frozen construction-frontier coordinate in runtime-model declaration
    /// order, shared by typed and dynamic scalar construction.
    pub construction_slot_index: usize,
    pub schedule: ScalarConstructionSchedule,
}

/// One assignment group frozen with its registered declaration order.
#[derive(Clone, Debug)]
pub(crate) struct DefaultAssignmentBinding<S> {
    pub group_index: usize,
    pub group: ScalarGroupBinding<S>,
}

/// Full schema-order default profile. It has no solution, callback cursor, or
/// mutable state, so it can be shared safely by native and dynamic runtime
/// instantiation without rebuilding either model shape.
#[derive(Clone, Debug)]
pub(crate) struct DefaultRuntimeBindings<S, V, DM, IDM> {
    pub list_slots: Vec<CompiledListSlot<S, V, DM, IDM>>,
    pub scalar_slots: Vec<DefaultScalarBinding<S>>,
    /// Descriptor-resolved scalar bindings used by every grouped construction
    /// child. They are compiled once instead of being reconstructed at the
    /// execution boundary.
    pub group_scalar_bindings: Vec<crate::descriptor::ResolvedVariableBinding<S>>,
    pub assignment_groups: Vec<DefaultAssignmentBinding<S>>,
    /// Frozen acceptor/forager policy for every omitted local-search
    /// component. It is available even when an explicit local-search phase
    /// supplies its own selector.
    pub local_search_components: DefaultLocalSearchComponents,
    /// A selector graph exists only when the configuration omits a
    /// local-search selector and the model exposes at least one matching
    /// capability. Absence is ordinary no-work graph data, not an alternate
    /// runtime path.
    pub local_search_plan: Option<DefaultLocalSearchPlan>,
    /// Frozen recursive selector nodes in the exact declaration order of
    /// `local_search_plan`. Omitted local search lowers these directly; it
    /// never recompiles schema/configuration at a solve boundary.
    pub local_search_nodes: Vec<super::graph::CompiledSelectorNode<S, V, DM, IDM>>,
    /// Omitted local search is a policy declaration, not an eagerly built
    /// phase. The compiled runner resolves eligibility against the
    /// effective solver termination immediately before it would enter the
    /// default local-search stage.
    pub local_search_policy: DefaultLocalSearchPolicy,
}

/// The one legal omitted-local-search policy.
///
/// Default local search has no safe completion boundary when the effective
/// solver policy is unbounded. Keeping that fact as immutable graph data
/// prevents the eventual executor from rebuilding the old local-search
/// strategy merely to rediscover whether it may run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultLocalSearchPolicy {
    RequireEffectiveSolverTermination,
}

/// Per-solve result of the immutable omitted-local-search policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultLocalSearchEligibility {
    Eligible,
    IneligibleWithoutEffectiveTermination,
}

impl DefaultLocalSearchPolicy {
    pub(crate) fn eligibility<S>(self, config: &SolverConfig) -> DefaultLocalSearchEligibility
    where
        S: PlanningSolution,
        S::Score: ParseableScore,
    {
        match self {
            Self::RequireEffectiveSolverTermination => {
                if crate::run::parse_configured_termination::<S>(config.termination.as_ref())
                    .has_effective_limit()
                {
                    DefaultLocalSearchEligibility::Eligible
                } else {
                    DefaultLocalSearchEligibility::IneligibleWithoutEffectiveTermination
                }
            }
        }
    }
}

pub(super) fn compile_default_runtime_bindings<S, V, DM, IDM>(
    descriptor: &SolutionDescriptor,
    model: &RuntimeModel<S, V, DM, IDM>,
    compile_omitted_local_search: bool,
    random_seed: Option<u64>,
) -> Result<DefaultRuntimeBindings<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    let target = VariableTargetConfig::default();
    let list_slots = matching_list_slots(model, descriptor, &target, "default_runtime")?;
    let scalar_slots: Vec<DefaultScalarBinding<S>> = model
        .variables()
        .iter()
        .enumerate()
        .filter_map(|(construction_slot_index, variable)| match variable {
            VariableSlot::Scalar(slot) => Some(DefaultScalarBinding {
                slot: RuntimeScalarSlot::Static(*slot),
                assignment_owned: model.assignment_group_covers_scalar_variable(slot),
                construction_slot_index,
                schedule: ScalarConstructionSchedule::DescriptorPlacement,
            }),
            VariableSlot::DynamicScalar(slot) => Some(DefaultScalarBinding {
                slot: RuntimeScalarSlot::Dynamic(slot.clone()),
                assignment_owned: model.assignment_group_covers_dynamic_scalar_variable(slot),
                construction_slot_index,
                schedule: ScalarConstructionSchedule::DescriptorPlacement,
            }),
            VariableSlot::List(_) | VariableSlot::DynamicList(_) => None,
        })
        .collect();
    let group_scalar_bindings = resolved_scalar_bindings(descriptor, model);
    let assignment_groups = model
        .assignment_scalar_groups()
        .map(|(group_index, group)| DefaultAssignmentBinding {
            group_index,
            group: group.clone(),
        })
        .collect();
    let local_search_components =
        compile_default_local_search_components(model, &list_slots, &scalar_slots, random_seed);
    let (local_search_plan, local_search_nodes) = if compile_omitted_local_search {
        match compile_default_local_search_plan(
            descriptor,
            model,
            &list_slots,
            &scalar_slots,
            local_search_components,
        )? {
            Some((plan, nodes)) => (Some(plan), nodes),
            None => (None, Vec::new()),
        }
    } else {
        (None, Vec::new())
    };
    Ok(DefaultRuntimeBindings {
        list_slots,
        scalar_slots,
        group_scalar_bindings,
        assignment_groups,
        local_search_components,
        local_search_plan,
        local_search_nodes,
        local_search_policy: DefaultLocalSearchPolicy::RequireEffectiveSolverTermination,
    })
}
