//! Exact candidate-trace plans for frozen compiled runtime execution.
//!
//! These functions consume compiled declarations and executor records only.
//! They never inspect a live model, rebuild a selector, or turn an extension
//! into a made-up candidate source.

use solverforge_config::{ConstructionHeuristicConfig, LocalSearchConfig};
use solverforge_core::domain::PlanningSolution;

use crate::runtime::compiler::default_local_search::{
    selection_order_label, selector_config_signature, slot_signature,
};
use crate::runtime::compiler::{
    CompiledLocalSearch, CompiledSelectorNode, DefaultConstructionStage,
    DefaultConstructionStepKind, DefaultLocalSearchComponents,
};
use crate::stats::CandidateTracePhasePlan;

use super::{
    DefaultConstructionStageExecutionRecord, DefaultRuntimeConstructionExecution,
    PreparedConstruction, ResolvedConstructionExecutionStep,
};

pub(super) fn compiled_selector_plan<S, V, DM, IDM>(
    selector: &CompiledSelectorNode<S, V, DM, IDM>,
) -> CandidateTracePhasePlan
where
    S: PlanningSolution,
{
    match selector {
        CompiledSelectorNode::Scalar {
            kind,
            config,
            candidate_order,
            candidate_metric,
            slots,
        } => CandidateTracePhasePlan::known(
            "solverforge.runtime.selector.scalar",
            [
                ("config", selector_config_signature(config)),
                (
                    "candidate_order",
                    format!("{candidate_order:?}").to_ascii_lowercase(),
                ),
                (
                    "selection_metric",
                    candidate_metric
                        .as_ref()
                        .map(|metric| metric.name().to_string())
                        .unwrap_or_default(),
                ),
                ("kind", scalar_leaf_kind_label(*kind).to_string()),
                ("slot_count", slots.len().to_string()),
                (
                    "slots",
                    slots
                        .iter()
                        .map(|slot| slot_signature(&slot.id()))
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ],
            Vec::new(),
        ),
        CompiledSelectorNode::List {
            kind,
            config,
            candidate_order,
            candidate_metric,
            slots,
        } => CandidateTracePhasePlan::known(
            "solverforge.runtime.selector.list",
            [
                ("config", selector_config_signature(config)),
                (
                    "candidate_order",
                    format!("{candidate_order:?}").to_ascii_lowercase(),
                ),
                (
                    "selection_metric",
                    candidate_metric
                        .as_ref()
                        .map(|metric| metric.name().to_string())
                        .unwrap_or_default(),
                ),
                ("kind", list_leaf_kind_label(*kind).to_string()),
                ("slot_count", slots.len().to_string()),
                (
                    "slots",
                    slots
                        .iter()
                        .map(|slot| slot_signature(&slot.identity()))
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ],
            Vec::new(),
        ),
        CompiledSelectorNode::GroupedScalar {
            config,
            candidate_order,
            candidate_metric,
            group_index,
            group,
        } => CandidateTracePhasePlan::known(
            "solverforge.runtime.selector.grouped_scalar",
            [
                ("config", selector_config_signature(config)),
                (
                    "candidate_order",
                    format!("{candidate_order:?}").to_ascii_lowercase(),
                ),
                (
                    "selection_metric",
                    candidate_metric
                        .as_ref()
                        .map(|metric| metric.name().to_string())
                        .unwrap_or_default(),
                ),
                ("group_index", group_index.to_string()),
                ("group_name", group.group_name.to_string()),
            ],
            Vec::new(),
        ),
        CompiledSelectorNode::Provider {
            config,
            candidate_order,
            candidate_metric,
            plan,
        } => CandidateTracePhasePlan::known(
            "solverforge.runtime.selector.provider",
            [
                ("config", selector_config_signature(config)),
                (
                    "candidate_order",
                    format!("{candidate_order:?}").to_ascii_lowercase(),
                ),
                (
                    "selection_metric",
                    candidate_metric
                        .as_ref()
                        .map(|metric| metric.name().to_string())
                        .unwrap_or_default(),
                ),
                ("move_kind", format!("{:?}", plan.move_kind)),
                ("schedule", format!("{:?}", plan.schedule)),
                (
                    "bindings",
                    plan.bindings
                        .iter()
                        .map(|binding| {
                            format!(
                                "{:?}:{}:{:?}:{:?}",
                                binding.handle,
                                binding.declared_schema_index,
                                binding.policy,
                                binding.allowed_slots
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("|"),
                ),
            ],
            Vec::new(),
        ),
        CompiledSelectorNode::Limited {
            selected_count_limit,
            selector,
        } => CandidateTracePhasePlan::known(
            "solverforge.runtime.selector.limited",
            [("selected_count_limit", selected_count_limit.to_string())],
            vec![compiled_selector_plan(selector)],
        ),
        CompiledSelectorNode::Union {
            selection_order,
            weighting,
            weights,
            children,
        } => CandidateTracePhasePlan::known(
            "solverforge.runtime.selector.union",
            [
                (
                    "selection_order",
                    selection_order_label(*selection_order).to_string(),
                ),
                ("weighting", format!("{weighting:?}").to_ascii_lowercase()),
                (
                    "weights",
                    weights
                        .iter()
                        .map(u64::to_string)
                        .collect::<Vec<_>>()
                        .join("|"),
                ),
                ("selector_count", children.len().to_string()),
            ],
            children.iter().map(compiled_selector_plan).collect(),
        ),
        CompiledSelectorNode::Cartesian {
            require_hard_improvement,
            left,
            right,
        } => CandidateTracePhasePlan::known(
            "solverforge.runtime.selector.cartesian",
            [(
                "require_hard_improvement",
                require_hard_improvement.to_string(),
            )],
            vec![compiled_selector_plan(left), compiled_selector_plan(right)],
        ),
    }
}

pub(super) fn local_search_plan<S, V, DM, IDM>(
    local_search: &CompiledLocalSearch<S, V, DM, IDM>,
    components: DefaultLocalSearchComponents,
    omitted_selector_children: Vec<CandidateTracePhasePlan>,
) -> CandidateTracePhasePlan
where
    S: PlanningSolution,
{
    match local_search {
        CompiledLocalSearch::AcceptorForager { config, selector } => {
            let children = match selector {
                crate::runtime::compiler::CompiledAcceptorForagerSelector::Explicit(selector) => {
                    vec![compiled_selector_plan(selector)]
                }
                crate::runtime::compiler::CompiledAcceptorForagerSelector::OmittedDefault => {
                    omitted_selector_children
                }
            };
            CandidateTracePhasePlan::known(
                "solverforge.runtime.local_search.acceptor_forager",
                local_search_attributes(config, components, children.len()),
                children,
            )
        }
        CompiledLocalSearch::VariableNeighborhoodDescent {
            config,
            neighborhoods,
        } => CandidateTracePhasePlan::known(
            "solverforge.runtime.local_search.variable_neighborhood_descent",
            [
                ("config", format!("{config:?}")),
                ("neighborhood_count", neighborhoods.len().to_string()),
            ],
            neighborhoods.iter().map(compiled_selector_plan).collect(),
        ),
    }
}

pub(super) fn prepared_construction_plan<S, V, DM, IDM>(
    construction: &PreparedConstruction<S, V, DM, IDM>,
) -> CandidateTracePhasePlan
where
    S: PlanningSolution,
{
    let (kind, config, target_count) = match construction {
        PreparedConstruction::ScalarOrMixed {
            config,
            scalar_slots,
            list_slots,
            ..
        } => (
            "solverforge.runtime.construction.scalar_or_mixed",
            config,
            scalar_slots.len() + list_slots.len(),
        ),
        PreparedConstruction::RoundRobin { config, slots } => (
            "solverforge.runtime.construction.round_robin",
            config,
            slots.len(),
        ),
        PreparedConstruction::CheapestInsertion { config, slots } => (
            "solverforge.runtime.construction.cheapest_insertion",
            config,
            slots.len(),
        ),
        PreparedConstruction::RegretInsertion { config, slots } => (
            "solverforge.runtime.construction.regret_insertion",
            config,
            slots.len(),
        ),
        PreparedConstruction::ClarkeWright { config, slots } => (
            // This canonical name deliberately identifies the savings path;
            // it must never collapse into generic/cheapest insertion trace.
            "solverforge.runtime.construction.clarke_wright",
            config,
            slots.len(),
        ),
        PreparedConstruction::KOpt { config, slots } => (
            "solverforge.runtime.construction.k_opt",
            config,
            slots.len(),
        ),
        PreparedConstruction::GroupedScalar { config, .. } => {
            ("solverforge.runtime.construction.grouped_scalar", config, 1)
        }
    };
    CandidateTracePhasePlan::known(
        kind,
        [
            ("config", construction_config_signature(config)),
            ("target_count", target_count.to_string()),
        ],
        Vec::new(),
    )
}

pub(super) fn default_construction_plan(
    execution: &DefaultRuntimeConstructionExecution,
) -> CandidateTracePhasePlan {
    CandidateTracePhasePlan::known(
        "solverforge.runtime.default_construction",
        [("ran_child_phase", execution.ran_child_phase.to_string())],
        execution.stages.iter().map(default_stage_plan).collect(),
    )
}

pub(super) fn phase_with_outcome(
    phase_index: usize,
    outcome: &str,
    child: CandidateTracePhasePlan,
) -> CandidateTracePhasePlan {
    CandidateTracePhasePlan::known(
        "solverforge.runtime.phase",
        [
            ("outcome", outcome.to_string()),
            ("phase_index", phase_index.to_string()),
        ],
        vec![child],
    )
}

pub(super) fn extension_plan(kind: &str) -> CandidateTracePhasePlan {
    // Extensions have no candidate-trace contract. Preserve that fact rather
    // than fabricating an equivalent selector/phase plan.
    CandidateTracePhasePlan::opaque(format!("solverforge.runtime.extension.{kind}"))
}

fn default_stage_plan(record: &DefaultConstructionStageExecutionRecord) -> CandidateTracePhasePlan {
    CandidateTracePhasePlan::known(
        "solverforge.runtime.default_construction.stage",
        [
            ("outcome", record.outcome.as_str().to_string()),
            ("stage", default_stage_label(record.stage).to_string()),
        ],
        record.steps.iter().map(default_step_plan).collect(),
    )
}

fn default_step_plan(step: &ResolvedConstructionExecutionStep) -> CandidateTracePhasePlan {
    let mut attributes = vec![
        (
            "kind".to_string(),
            default_step_kind_label(step.kind).to_string(),
        ),
        ("outcome".to_string(), step.outcome.as_str().to_string()),
        ("required_only".to_string(), step.required_only.to_string()),
        (
            "target".to_string(),
            step.target
                .as_ref()
                .map_or_else(|| "none".to_string(), slot_signature),
        ),
    ];
    if let Some(policies) = &step.list_policies {
        match step.kind {
            DefaultConstructionStepKind::ListConstruction(
                solverforge_config::ConstructionHeuristicType::ListClarkeWright,
            ) => attributes.extend([
                ("ownership".to_string(), policies.ownership.to_string()),
                (
                    "route_feasibility".to_string(),
                    policies.route_feasibility.to_string(),
                ),
                ("route_read".to_string(), policies.route_read.to_string()),
                (
                    "route_replace".to_string(),
                    policies.route_replace.to_string(),
                ),
                (
                    "savings_metric_class".to_string(),
                    policies.savings_metric_class.to_string(),
                ),
            ]),
            DefaultConstructionStepKind::ListConstruction(_) => attributes.extend([
                (
                    "construction_order".to_string(),
                    policies.construction_order.to_string(),
                ),
                ("ownership".to_string(), policies.ownership.to_string()),
                ("precedence".to_string(), policies.precedence.to_string()),
            ]),
            DefaultConstructionStepKind::ListKOpt => attributes.extend([
                (
                    "route_feasibility".to_string(),
                    policies.route_feasibility.to_string(),
                ),
                ("route_read".to_string(), policies.route_read.to_string()),
                (
                    "route_replace".to_string(),
                    policies.route_replace.to_string(),
                ),
            ]),
            DefaultConstructionStepKind::AssignmentRequired
            | DefaultConstructionStepKind::AssignmentOptional
            | DefaultConstructionStepKind::ScalarFirstFit => {}
        }
    }
    CandidateTracePhasePlan::known(
        default_step_kind_trace_name(step.kind),
        attributes,
        Vec::new(),
    )
}

fn local_search_attributes(
    config: &LocalSearchConfig,
    components: DefaultLocalSearchComponents,
    selector_count: usize,
) -> Vec<(String, String)> {
    vec![
        ("config".to_string(), format!("{config:?}")),
        (
            "acceptor".to_string(),
            config.acceptor.as_ref().map_or_else(
                || format!("default:{:?}", components.acceptor),
                |acceptor| format!("configured:{acceptor:?}"),
            ),
        ),
        (
            "forager".to_string(),
            config.forager.as_ref().map_or_else(
                || {
                    if config.acceptor.as_ref().is_some_and(|acceptor| {
                        matches!(acceptor, solverforge_config::AcceptorConfig::TabuSearch(_))
                    }) {
                        "implicit_tabu:BestScore".to_string()
                    } else {
                        format!("default:{:?}", components.forager)
                    }
                },
                |forager| format!("configured:{forager:?}"),
            ),
        ),
        ("selector_count".to_string(), selector_count.to_string()),
    ]
}

fn construction_config_signature(config: &ConstructionHeuristicConfig) -> String {
    format!("{config:?}")
}

fn scalar_leaf_kind_label(kind: crate::runtime::compiler::ScalarLeafKind) -> &'static str {
    use crate::runtime::compiler::ScalarLeafKind;
    match kind {
        ScalarLeafKind::Change => "change",
        ScalarLeafKind::Swap => "swap",
        ScalarLeafKind::NearbyChange => "nearby_change",
        ScalarLeafKind::NearbySwap => "nearby_swap",
        ScalarLeafKind::PillarChange => "pillar_change",
        ScalarLeafKind::PillarSwap => "pillar_swap",
        ScalarLeafKind::RuinRecreate => "ruin_recreate",
    }
}

fn list_leaf_kind_label(kind: crate::runtime::compiler::ListLeafKind) -> &'static str {
    use crate::runtime::compiler::ListLeafKind;
    match kind {
        ListLeafKind::Change => "change",
        ListLeafKind::NearbyChange => "nearby_change",
        ListLeafKind::Swap => "swap",
        ListLeafKind::Permute => "permute",
        ListLeafKind::Precedence => "precedence",
        ListLeafKind::NearbySwap => "nearby_swap",
        ListLeafKind::SublistChange => "sublist_change",
        ListLeafKind::SublistSwap => "sublist_swap",
        ListLeafKind::Reverse => "reverse",
        ListLeafKind::KOpt => "k_opt",
        ListLeafKind::Ruin => "ruin",
    }
}

fn default_stage_label(stage: DefaultConstructionStage) -> &'static str {
    match stage {
        DefaultConstructionStage::Preconstruction(
            crate::runtime::compiler::DefaultPreconstructionStage::ListConstruction,
        ) => "list_construction",
        DefaultConstructionStage::Preconstruction(
            crate::runtime::compiler::DefaultPreconstructionStage::AssignmentRequired,
        ) => "assignment_required",
        DefaultConstructionStage::Preconstruction(
            crate::runtime::compiler::DefaultPreconstructionStage::AssignmentOptional,
        ) => "assignment_optional",
        DefaultConstructionStage::Preconstruction(
            crate::runtime::compiler::DefaultPreconstructionStage::ScalarFirstFit,
        ) => "scalar_first_fit",
        DefaultConstructionStage::PostConstructionKOpt => "postconstruction_k_opt",
    }
}

fn default_step_kind_label(kind: DefaultConstructionStepKind) -> &'static str {
    match kind {
        DefaultConstructionStepKind::ListConstruction(heuristic) => match heuristic {
            solverforge_config::ConstructionHeuristicType::ListRoundRobin => "list_round_robin",
            solverforge_config::ConstructionHeuristicType::ListCheapestInsertion => {
                "list_cheapest_insertion"
            }
            solverforge_config::ConstructionHeuristicType::ListRegretInsertion => {
                "list_regret_insertion"
            }
            solverforge_config::ConstructionHeuristicType::ListClarkeWright => "list_clarke_wright",
            other => panic!("default list construction retained invalid heuristic {other:?}"),
        },
        DefaultConstructionStepKind::AssignmentRequired => "assignment_required",
        DefaultConstructionStepKind::AssignmentOptional => "assignment_optional",
        DefaultConstructionStepKind::ScalarFirstFit => "scalar_first_fit",
        DefaultConstructionStepKind::ListKOpt => "list_k_opt",
    }
}

fn default_step_kind_trace_name(kind: DefaultConstructionStepKind) -> &'static str {
    match kind {
        DefaultConstructionStepKind::ListConstruction(
            solverforge_config::ConstructionHeuristicType::ListClarkeWright,
        ) => "solverforge.runtime.construction.clarke_wright",
        DefaultConstructionStepKind::ListConstruction(
            solverforge_config::ConstructionHeuristicType::ListRoundRobin,
        ) => "solverforge.runtime.construction.round_robin",
        DefaultConstructionStepKind::ListConstruction(
            solverforge_config::ConstructionHeuristicType::ListCheapestInsertion,
        ) => "solverforge.runtime.construction.cheapest_insertion",
        DefaultConstructionStepKind::ListConstruction(
            solverforge_config::ConstructionHeuristicType::ListRegretInsertion,
        ) => "solverforge.runtime.construction.regret_insertion",
        DefaultConstructionStepKind::AssignmentRequired
        | DefaultConstructionStepKind::AssignmentOptional => {
            "solverforge.runtime.construction.grouped_scalar"
        }
        DefaultConstructionStepKind::ScalarFirstFit => {
            "solverforge.runtime.construction.scalar_or_mixed"
        }
        DefaultConstructionStepKind::ListKOpt => "solverforge.runtime.construction.k_opt",
        DefaultConstructionStepKind::ListConstruction(other) => {
            panic!("default list construction retained invalid heuristic {other:?}")
        }
    }
}
