//! State-dependent omitted-construction staging.
//!
//! The immutable default bindings live in the parent module.  This module
//! resolves only the current solution-state decisions and never rediscovers a
//! schema, source stream, or host callback.

use std::fmt;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation,
    VariableTargetConfig,
};
use solverforge_core::domain::PlanningSolution;

use crate::builder::context::list_access::ListAccess;
use crate::builder::RuntimeScalarSlotId;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::construction::ScalarOrMixedSlotOrder;

use super::super::graph::{CompiledConstruction, ListConstructionKind};
use super::super::types::CompiledListSlot;
use super::{DefaultAssignmentBinding, DefaultRuntimeBindings, DefaultScalarBinding};

/// Ordered default construction boundaries. The runner resolves each boundary
/// from the current working solution, executes it, then moves to the next.
///
/// This is intentionally more granular than a one-shot "default
/// construction" expansion. Required assignment completion can make optional
/// assignment legal, and list construction can make route K-opt legal. A
/// plan resolved from the initial solution must not predict either later
/// stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultPreconstructionStage {
    ListConstruction,
    AssignmentRequired,
    AssignmentOptional,
    ScalarFirstFit,
}

impl DefaultPreconstructionStage {
    pub(crate) const ORDERED: [Self; 4] = [
        Self::ListConstruction,
        Self::AssignmentRequired,
        Self::AssignmentOptional,
        Self::ScalarFirstFit,
    ];
}

/// The exact stage that produced a resolved default-construction plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultConstructionStage {
    Preconstruction(DefaultPreconstructionStage),
    PostConstructionKOpt,
}

/// Why a per-solve default construction child exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultConstructionStepKind {
    ListConstruction(ConstructionHeuristicType),
    AssignmentRequired,
    AssignmentOptional,
    ScalarFirstFit,
    ListKOpt,
}

/// Frozen list policy labels attached to one resolved default step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DefaultListPolicyProvenance {
    pub route_read: &'static str,
    pub route_replace: &'static str,
    pub route_feasibility: &'static str,
    pub savings_metric_class: &'static str,
    pub ownership: &'static str,
    pub construction_order: &'static str,
    pub precedence: &'static str,
}

/// One exact child selected by per-solve default construction expansion.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedDefaultConstructionStep<S, V, DM, IDM> {
    pub kind: DefaultConstructionStepKind,
    pub construction: CompiledConstruction<S, V, DM, IDM>,
    /// Required-only selection changes which assignment rows are considered;
    /// it never changes the configured termination policy.
    pub required_only: bool,
    pub target: Option<RuntimeScalarSlotId>,
    pub list_policies: Option<DefaultListPolicyProvenance>,
}

/// The solution-state-dependent default construction sequence.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedDefaultConstructionPlan<S, V, DM, IDM> {
    pub stage: DefaultConstructionStage,
    pub steps: Vec<ResolvedDefaultConstructionStep<S, V, DM, IDM>>,
}

pub(crate) fn resolve_default_preconstruction_stage<S, V, DM, IDM>(
    bindings: &DefaultRuntimeBindings<S, V, DM, IDM>,
    stage: DefaultPreconstructionStage,
    solution: &S,
) -> ResolvedDefaultConstructionPlan<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let steps = match stage {
        DefaultPreconstructionStage::ListConstruction => bindings
            .list_slots
            .iter()
            .map(list_construction_step)
            .collect(),
        DefaultPreconstructionStage::AssignmentRequired => bindings
            .assignment_groups
            .iter()
            .filter(|binding| {
                binding
                    .group
                    .assignment()
                    .is_some_and(|assignment| assignment.remaining_required_count(solution) > 0)
            })
            .map(|binding| assignment_step(binding, &bindings.group_scalar_bindings, true))
            .collect(),
        DefaultPreconstructionStage::AssignmentOptional => bindings
            .assignment_groups
            .iter()
            .filter(|binding| {
                binding.group.assignment().is_some_and(|assignment| {
                    assignment.remaining_required_count(solution) == 0
                        && assignment.unassigned_count(solution) > 0
                })
            })
            .map(|binding| assignment_step(binding, &bindings.group_scalar_bindings, false))
            .collect(),
        DefaultPreconstructionStage::ScalarFirstFit => bindings
            .scalar_slots
            .iter()
            .filter(|binding| !binding.assignment_owned)
            .map(scalar_first_fit_step)
            .collect(),
    };

    ResolvedDefaultConstructionPlan {
        stage: DefaultConstructionStage::Preconstruction(stage),
        steps,
    }
}

pub(crate) fn resolve_default_postconstruction_kopt<S, V, DM, IDM>(
    bindings: &DefaultRuntimeBindings<S, V, DM, IDM>,
    solution: &S,
) -> ResolvedDefaultConstructionPlan<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let steps = bindings
        .list_slots
        .iter()
        .filter(|slot| {
            slot.supports(crate::builder::context::list_access::ListAccessCapability::Route)
                && list_has_content(slot, solution)
        })
        .map(kopt_step)
        .collect();

    ResolvedDefaultConstructionPlan {
        stage: DefaultConstructionStage::PostConstructionKOpt,
        steps,
    }
}

fn list_construction_step<S, V, DM, IDM>(
    slot: &CompiledListSlot<S, V, DM, IDM>,
) -> ResolvedDefaultConstructionStep<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let heuristic = default_list_construction_heuristic(slot);
    let target = slot.identity();
    ResolvedDefaultConstructionStep {
        kind: DefaultConstructionStepKind::ListConstruction(heuristic),
        construction: CompiledConstruction::List {
            kind: ListConstructionKind::from_heuristic(heuristic)
                .expect("default list construction must map to a compiled list kind"),
            config: list_config(heuristic, &target),
            slots: vec![slot.clone()],
        },
        required_only: false,
        target: Some(target),
        list_policies: Some(list_policies(slot)),
    }
}

fn scalar_first_fit_step<S, V, DM, IDM>(
    binding: &DefaultScalarBinding<S>,
) -> ResolvedDefaultConstructionStep<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    let target = binding.slot.id();
    ResolvedDefaultConstructionStep {
        kind: DefaultConstructionStepKind::ScalarFirstFit,
        construction: CompiledConstruction::ScalarOrMixed {
            config: scalar_first_fit_config(&target),
            schedule: binding.schedule,
            scalar_slots: vec![binding.slot.clone()],
            list_slots: Vec::new(),
            slot_order: vec![ScalarOrMixedSlotOrder::Scalar {
                scalar_index: 0,
                construction_slot_index: binding.construction_slot_index,
            }],
        },
        required_only: false,
        target: Some(target),
        list_policies: None,
    }
}

fn kopt_step<S, V, DM, IDM>(
    slot: &CompiledListSlot<S, V, DM, IDM>,
) -> ResolvedDefaultConstructionStep<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let target = slot.identity();
    ResolvedDefaultConstructionStep {
        kind: DefaultConstructionStepKind::ListKOpt,
        construction: CompiledConstruction::List {
            kind: ListConstructionKind::KOpt,
            config: list_config(ConstructionHeuristicType::ListKOpt, &target),
            slots: vec![slot.clone()],
        },
        required_only: false,
        target: Some(target),
        list_policies: Some(list_policies(slot)),
    }
}

fn list_has_content<S, V, DM, IDM>(slot: &CompiledListSlot<S, V, DM, IDM>, solution: &S) -> bool
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    (0..slot.entity_count(solution)).any(|entity| slot.list_len(solution, entity) > 0)
}

fn default_list_construction_heuristic<S, V, DM, IDM>(
    slot: &CompiledListSlot<S, V, DM, IDM>,
) -> ConstructionHeuristicType {
    use crate::builder::context::list_access::ListAccessCapability;

    if slot.supports(ListAccessCapability::Savings) {
        ConstructionHeuristicType::ListClarkeWright
    } else if slot.precedence_policy().is_explicit()
        && slot.construction_order_policy().is_explicit()
        && slot.ownership_policy().is_explicit()
    {
        ConstructionHeuristicType::ListRegretInsertion
    } else if slot.precedence_policy().is_explicit()
        && slot.construction_order_policy().is_explicit()
    {
        ConstructionHeuristicType::ListRoundRobin
    } else {
        ConstructionHeuristicType::ListCheapestInsertion
    }
}

fn assignment_step<S, V, DM, IDM>(
    binding: &DefaultAssignmentBinding<S>,
    scalar_bindings: &[crate::descriptor::ResolvedVariableBinding<S>],
    required_only: bool,
) -> ResolvedDefaultConstructionStep<S, V, DM, IDM> {
    let config = ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::CheapestInsertion,
        construction_obligation: ConstructionObligation::AssignWhenCandidateExists,
        group_name: Some(binding.group.group_name.to_string()),
        ..ConstructionHeuristicConfig::default()
    };
    ResolvedDefaultConstructionStep {
        kind: if required_only {
            DefaultConstructionStepKind::AssignmentRequired
        } else {
            DefaultConstructionStepKind::AssignmentOptional
        },
        construction: CompiledConstruction::GroupedScalar {
            config,
            group_index: binding.group_index,
            group: binding.group.clone(),
            scalar_bindings: scalar_bindings.to_vec(),
        },
        required_only,
        target: None,
        list_policies: None,
    }
}

fn scalar_first_fit_config(slot: &RuntimeScalarSlotId) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::FirstFit,
        target: target_config(slot),
        ..ConstructionHeuristicConfig::default()
    }
}

fn list_config(
    heuristic: ConstructionHeuristicType,
    slot: &RuntimeScalarSlotId,
) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type: heuristic,
        target: target_config(slot),
        ..ConstructionHeuristicConfig::default()
    }
}

fn target_config(slot: &RuntimeScalarSlotId) -> VariableTargetConfig {
    VariableTargetConfig {
        entity_class: Some(slot.entity_class.to_string()),
        variable_name: Some(slot.variable_name.to_string()),
    }
}

fn list_policies<S, V, DM, IDM>(
    slot: &CompiledListSlot<S, V, DM, IDM>,
) -> DefaultListPolicyProvenance {
    DefaultListPolicyProvenance {
        route_read: slot.route_read_policy().trace_label(),
        route_replace: slot.route_replace_policy().trace_label(),
        route_feasibility: slot.route_feasibility_policy().trace_label(),
        savings_metric_class: slot.savings_metric_class_policy().trace_label(),
        ownership: slot.ownership_policy().trace_label(),
        construction_order: slot.construction_order_policy().trace_label(),
        precedence: slot.precedence_policy().trace_label(),
    }
}
