use solverforge_config::{MoveSelectorConfig, SelectionOrder};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::builder::{RuntimeCandidateMetricBinding, RuntimeModel, RuntimeProviderHandle};

use super::graph::{
    CompiledProviderPlan, CompiledSelectorNode, ListLeafKind, ProviderBindingPlan,
    ProviderBindingPolicy, ProviderMoveKind, ProviderPullTiming, ProviderSchedule, ScalarLeafKind,
    GROUP_PROVIDER_ROTATION_SALT, PYTHON_PROVIDER_CANDIDATE_CONTRACT,
    STATIC_GROUP_PROVIDER_ROTATION_SALT,
};
use super::providers::{
    compile_conflict_repair, scalar_group_provider_slots, unscoped_dynamic_provider_slots,
};
use super::slots::{compile_list_leaf, compile_scalar_leaf};
use super::types::{RuntimeCompileError, RuntimeCompileErrorKind};

pub(super) fn compile_selector<S, V, DM, IDM>(
    config: &MoveSelectorConfig,
    default_candidate_order: SelectionOrder,
    path: &str,
    descriptor: &SolutionDescriptor,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Result<CompiledSelectorNode<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    let candidate_order = config.selection_order().unwrap_or(default_candidate_order);
    let candidate_metric = compile_candidate_metric(config, candidate_order, path, model)?;
    match config {
        MoveSelectorConfig::ChangeMoveSelector(selector) => compile_scalar_leaf(
            ScalarLeafKind::Change,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        ),
        MoveSelectorConfig::SwapMoveSelector(selector) => compile_scalar_leaf(
            ScalarLeafKind::Swap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        ),
        MoveSelectorConfig::NearbyChangeMoveSelector(selector) => compile_scalar_leaf(
            ScalarLeafKind::NearbyChange,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        ),
        MoveSelectorConfig::NearbySwapMoveSelector(selector) => compile_scalar_leaf(
            ScalarLeafKind::NearbySwap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        ),
        MoveSelectorConfig::PillarChangeMoveSelector(selector) => compile_scalar_leaf(
            ScalarLeafKind::PillarChange,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        ),
        MoveSelectorConfig::PillarSwapMoveSelector(selector) => compile_scalar_leaf(
            ScalarLeafKind::PillarSwap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        ),
        MoveSelectorConfig::RuinRecreateMoveSelector(selector) => compile_scalar_leaf(
            ScalarLeafKind::RuinRecreate,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        ),
        MoveSelectorConfig::ListChangeMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::Change,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::NearbyListChangeMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::NearbyChange,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::ListSwapMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::Swap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::ListPermuteMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::Permute,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::ListPrecedenceMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::Precedence,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::NearbyListSwapMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::NearbySwap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::SublistChangeMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::SublistChange,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::SublistSwapMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::SublistSwap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::ListReverseMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::Reverse,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::KOptMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::KOpt,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::ListRuinMoveSelector(selector) => compile_list_leaf(
            ListLeafKind::Ruin,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        ),
        MoveSelectorConfig::GroupedScalarMoveSelector(grouped) => {
            let registry = model.runtime_provider_registry();
            let callback_indices = registry.group_indices(&grouped.group_name);
            let static_indices = registry
                .static_groups()
                .iter()
                .enumerate()
                .filter_map(|(index, binding)| {
                    (binding.group_name == grouped.group_name).then_some(index)
                })
                .collect::<Vec<_>>();
            let assignment = model
                .scalar_groups()
                .iter()
                .enumerate()
                .find(|(_, group)| group.group_name == grouped.group_name && group.is_assignment())
                .map(|(index, group)| (index, group.clone()));
            if assignment.is_some() && !callback_indices.is_empty() {
                return Err(RuntimeCompileError {
                    path: path.to_string(),
                    kind: RuntimeCompileErrorKind::DuplicateProviderGroupName {
                        group_name: grouped.group_name.clone(),
                    },
                });
            }
            if !callback_indices.is_empty() && !static_indices.is_empty() {
                return Err(RuntimeCompileError {
                    path: path.to_string(),
                    kind: RuntimeCompileErrorKind::DuplicateProviderGroupName {
                        group_name: grouped.group_name.clone(),
                    },
                });
            }
            if let Some((group_index, group)) = assignment {
                // Assignment is the one intentional distinct algorithm. It
                // never enters a generic provider node or callback universe.
                return Ok(CompiledSelectorNode::GroupedScalar {
                    config: config.clone(),
                    candidate_order,
                    candidate_metric,
                    group_index,
                    group,
                });
            }
            if let Some(&static_index) = static_indices.first() {
                let binding = &registry.static_groups()[static_index];
                let group = model
                    .scalar_groups()
                    .get(binding.declared_index)
                    .expect("frozen static group binding must refer to its runtime model group");
                return Ok(CompiledSelectorNode::Provider {
                    config: config.clone(),
                    candidate_order,
                    candidate_metric,
                    plan: CompiledProviderPlan {
                        move_kind: ProviderMoveKind::Grouped,
                        schedule: ProviderSchedule::Group {
                            value_candidate_limit: grouped.value_candidate_limit,
                            requested_max_moves_per_step: grouped.max_moves_per_step,
                        },
                        bindings: vec![ProviderBindingPlan {
                            handle: RuntimeProviderHandle::StaticGroup(static_index),
                            declared_schema_index: binding.declared_index,
                            allowed_slots: scalar_group_provider_slots(model, group, path)?,
                            policy: ProviderBindingPolicy::StaticGroup {
                                rotation_seed_salt: STATIC_GROUP_PROVIDER_ROTATION_SALT
                                    ^ grouped.group_name.len() as u64,
                                declared_max_moves_per_step: group.limits.max_moves_per_step,
                                pull_timing: ProviderPullTiming::OpenCursor,
                            },
                            candidate_contract: PYTHON_PROVIDER_CANDIDATE_CONTRACT,
                        }],
                    },
                });
            }
            if !callback_indices.is_empty() {
                let allowed_slots = unscoped_dynamic_provider_slots(model, path)?;
                let bindings = callback_indices
                    .into_iter()
                    .map(|index| {
                        let binding = &registry.groups()[index];
                        ProviderBindingPlan {
                            handle: RuntimeProviderHandle::CallbackGroup(index),
                            declared_schema_index: binding.declared_index,
                            allowed_slots: allowed_slots.clone(),
                            policy: ProviderBindingPolicy::CallbackGroup {
                                rotation_seed_salt: GROUP_PROVIDER_ROTATION_SALT
                                    ^ grouped.group_name.len() as u64,
                                pull_timing: ProviderPullTiming::FirstReachableNext,
                            },
                            candidate_contract: PYTHON_PROVIDER_CANDIDATE_CONTRACT,
                        }
                    })
                    .collect();
                return Ok(CompiledSelectorNode::Provider {
                    config: config.clone(),
                    candidate_order,
                    candidate_metric,
                    plan: CompiledProviderPlan {
                        move_kind: ProviderMoveKind::Grouped,
                        schedule: ProviderSchedule::Group {
                            value_candidate_limit: grouped.value_candidate_limit,
                            requested_max_moves_per_step: grouped.max_moves_per_step,
                        },
                        bindings,
                    },
                });
            }
            Err(RuntimeCompileError {
                path: path.to_string(),
                kind: RuntimeCompileErrorKind::MissingScalarGroup {
                    group_name: grouped.group_name.clone(),
                },
            })
        }
        MoveSelectorConfig::ConflictRepairMoveSelector(_) => {
            compile_conflict_repair(config, candidate_order, candidate_metric, path, model)
        }
        MoveSelectorConfig::CompoundConflictRepairMoveSelector(_) => {
            compile_conflict_repair(config, candidate_order, candidate_metric, path, model)
        }
        MoveSelectorConfig::LimitedNeighborhood(limit) => {
            let selector = compile_selector(
                limit.selector.as_ref(),
                default_candidate_order,
                &format!("{path}.selector"),
                descriptor,
                model,
            )?;
            Ok(CompiledSelectorNode::Limited {
                selected_count_limit: limit.selected_count_limit,
                selector: Box::new(selector),
            })
        }
        MoveSelectorConfig::UnionMoveSelector(union) => {
            if union.selectors.is_empty() {
                return Err(RuntimeCompileError {
                    path: path.to_string(),
                    kind: RuntimeCompileErrorKind::EmptyUnion,
                });
            }
            let children = union
                .selectors
                .iter()
                .enumerate()
                .map(|(index, child)| {
                    compile_selector(
                        child,
                        default_candidate_order,
                        &format!("{path}.selectors[{index}]"),
                        descriptor,
                        model,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let weighting_error = match union.weighting {
                solverforge_config::UnionWeighting::Equal if !union.weights.is_empty() => {
                    Some("equal union weighting does not accept explicit weights")
                }
                solverforge_config::UnionWeighting::Fixed
                    if union.weights.len() != children.len() =>
                {
                    Some("fixed union weight count must match selector count")
                }
                solverforge_config::UnionWeighting::Fixed if union.weights.contains(&0) => {
                    Some("fixed union weights must all be positive")
                }
                solverforge_config::UnionWeighting::CandidateCount if !union.weights.is_empty() => {
                    Some("candidate_count union weighting does not accept explicit weights")
                }
                solverforge_config::UnionWeighting::CandidateCount
                    if children.iter().any(CompiledSelectorNode::contains_provider) =>
                {
                    Some("candidate_count union weighting requires statically countable children")
                }
                solverforge_config::UnionWeighting::Fixed
                | solverforge_config::UnionWeighting::CandidateCount
                    if !matches!(
                        union.selection_order,
                        solverforge_config::UnionSelectionOrder::Random
                            | solverforge_config::UnionSelectionOrder::StratifiedRandom
                    ) =>
                {
                    Some("weighted union selection requires random or stratified_random order")
                }
                _ => None,
            };
            if let Some(message) = weighting_error {
                return Err(RuntimeCompileError {
                    path: path.to_string(),
                    kind: RuntimeCompileErrorKind::LocalSearchShape {
                        message: message.to_string(),
                    },
                });
            }
            Ok(CompiledSelectorNode::Union {
                selection_order: union.selection_order,
                weighting: union.weighting,
                weights: union.weights.clone(),
                children,
            })
        }
        MoveSelectorConfig::CartesianProductMoveSelector(cartesian) => {
            if cartesian.selectors.len() != 2 {
                return Err(RuntimeCompileError {
                    path: path.to_string(),
                    kind: RuntimeCompileErrorKind::InvalidCartesianArity {
                        actual: cartesian.selectors.len(),
                    },
                });
            }
            let left = compile_selector(
                &cartesian.selectors[0],
                default_candidate_order,
                &format!("{path}.selectors[0]"),
                descriptor,
                model,
            )?;
            if left.requires_score_during_move() {
                return Err(RuntimeCompileError {
                    path: format!("{path}.selectors[0]"),
                    kind: RuntimeCompileErrorKind::PreviewUnsafeCartesianLeft,
                });
            }
            // Compilation only binds the right child.  The future cursor opens
            // it only after the left preview is legal, preserving deferred
            // provider invocation for nested Cartesian selectors.
            let right = compile_selector(
                &cartesian.selectors[1],
                default_candidate_order,
                &format!("{path}.selectors[1]"),
                descriptor,
                model,
            )?;
            Ok(CompiledSelectorNode::Cartesian {
                require_hard_improvement: cartesian.require_hard_improvement,
                left: Box::new(left),
                right: Box::new(right),
            })
        }
    }
}

fn compile_candidate_metric<S, V, DM, IDM>(
    config: &MoveSelectorConfig,
    candidate_order: SelectionOrder,
    path: &str,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Result<Option<RuntimeCandidateMetricBinding<S>>, RuntimeCompileError> {
    let metric_name = config.selection_metric();
    match candidate_order {
        SelectionOrder::Sorted | SelectionOrder::Probabilistic => {
            let Some(metric_name) = metric_name else {
                return Err(RuntimeCompileError {
                    path: path.to_string(),
                    kind: RuntimeCompileErrorKind::LocalSearchShape {
                        message: format!(
                            "{} selection order requires selection_metric",
                            format!("{candidate_order:?}").to_ascii_lowercase()
                        ),
                    },
                });
            };
            let Some(metric) = model.candidate_metrics().get(metric_name) else {
                return Err(RuntimeCompileError {
                    path: format!("{path}.selection_metric"),
                    kind: RuntimeCompileErrorKind::LocalSearchShape {
                        message: format!("candidate metric `{metric_name}` is not registered"),
                    },
                });
            };
            Ok(Some(metric.clone()))
        }
        SelectionOrder::Original | SelectionOrder::Random | SelectionOrder::Shuffled => {
            if metric_name.is_some() {
                return Err(RuntimeCompileError {
                    path: format!("{path}.selection_metric"),
                    kind: RuntimeCompileErrorKind::LocalSearchShape {
                        message: "selection_metric is valid only with sorted or probabilistic selection order"
                            .to_string(),
                    },
                });
            }
            Ok(None)
        }
    }
}
