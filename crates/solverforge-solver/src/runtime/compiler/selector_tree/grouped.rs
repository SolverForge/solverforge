use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn compile_grouped_or_provider_selector<S, V, DM, IDM>(
    config: &MoveSelectorConfig,
    grouped: &solverforge_config::GroupedScalarMoveSelectorConfig,
    candidate_order: SelectionOrder,
    candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
    path: &str,
    model: &RuntimeModel<S, V, DM, IDM>,
    builder: &mut SelectorGraphBuilder<S, V, DM, IDM>,
) -> Result<CompiledSelectorNodeId, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    let registry = model.runtime_provider_registry();
    let callback_indices = registry.group_indices(&grouped.group_name);
    let static_indices = registry
        .static_groups()
        .iter()
        .enumerate()
        .filter_map(|(index, binding)| (binding.group_name == grouped.group_name).then_some(index))
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
    if let Some((_, group)) = assignment {
        // Assignment is the one intentional distinct algorithm. It never
        // enters a generic provider node or callback universe.
        return Ok(builder.push(CompiledSelectorNode::GroupedScalar {
            config: config.clone(),
            candidate_order,
            candidate_metric,
            group,
        }));
    }
    if let Some(&static_index) = static_indices.first() {
        let binding = &registry.static_groups()[static_index];
        let group = model
            .scalar_groups()
            .get(binding.declared_index)
            .expect("frozen static group binding must refer to its runtime model group");
        return Ok(builder.push(CompiledSelectorNode::Provider {
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
        }));
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
        return Ok(builder.push(CompiledSelectorNode::Provider {
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
        }));
    }
    Err(RuntimeCompileError {
        path: path.to_string(),
        kind: RuntimeCompileErrorKind::MissingScalarGroup {
            group_name: grouped.group_name.clone(),
        },
    })
}

pub(super) fn compile_candidate_metric<S, V, DM, IDM>(
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
