fn build_descriptor_flat_selector<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
    random_seed: Option<u64>,
) -> Result<DescriptorFlatSelector<S>, RuntimeBuildError>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    let bindings = collect_bindings(descriptor);
    let mut leaves = Vec::new();

    fn require_matches<S>(
        label: &str,
        entity_class: Option<&str>,
        variable_name: Option<&str>,
        matched: &[VariableBinding],
    ) -> Result<(), RuntimeBuildError>
    where
        S: PlanningSolution + 'static,
        S::Score: Score,
    {
        if matched.is_empty() {
            return Err(RuntimeBuildError::declaration(format!(
                "{label} selector matched no scalar planning variables for entity_class={entity_class:?} variable_name={variable_name:?}"
            )));
        }
        Ok(())
    }

    fn collect<S>(
        cfg: &MoveSelectorConfig,
        descriptor: &SolutionDescriptor,
        bindings: &[VariableBinding],
        random_seed: Option<u64>,
        leaves: &mut Vec<DescriptorLeafSelector<S>>,
    ) -> Result<(), RuntimeBuildError>
    where
        S: PlanningSolution + 'static,
        S::Score: Score,
    {
        match cfg {
            MoveSelectorConfig::ChangeMoveSelector(change) => {
                let matched = find_binding(
                    bindings,
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "change_move",
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                    &matched,
                )?;
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::Change(
                        DescriptorChangeMoveSelector::new(
                            binding,
                            descriptor.clone(),
                            change.value_candidate_limit,
                        ),
                    ));
                }
            }
            MoveSelectorConfig::SwapMoveSelector(swap) => {
                let matched = find_binding(
                    bindings,
                    swap.target.entity_class.as_deref(),
                    swap.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "swap_move",
                    swap.target.entity_class.as_deref(),
                    swap.target.variable_name.as_deref(),
                    &matched,
                )?;
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::Swap(
                        DescriptorSwapMoveSelector::new(binding, descriptor.clone()),
                    ));
                }
            }
            MoveSelectorConfig::NearbyChangeMoveSelector(nearby_change) => {
                let matched = find_binding(
                    bindings,
                    nearby_change.target.entity_class.as_deref(),
                    nearby_change.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "nearby_change_move",
                    nearby_change.target.entity_class.as_deref(),
                    nearby_change.target.variable_name.as_deref(),
                    &matched,
                )?;
                for binding in matched {
                    if binding.nearby_value_candidates.is_none() {
                        return Err(RuntimeBuildError::declaration(format!(
                            "nearby_change_move selector requires nearby_value_candidates for {}::{}",
                            binding.entity_type_name, binding.variable_name,
                        )));
                    }
                    leaves.push(DescriptorLeafSelector::NearbyChange(
                        DescriptorNearbyChangeMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            max_nearby: nearby_change.max_nearby,
                            value_candidate_limit: nearby_change.value_candidate_limit,
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::NearbySwapMoveSelector(nearby_swap) => {
                let matched = find_binding(
                    bindings,
                    nearby_swap.target.entity_class.as_deref(),
                    nearby_swap.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "nearby_swap_move",
                    nearby_swap.target.entity_class.as_deref(),
                    nearby_swap.target.variable_name.as_deref(),
                    &matched,
                )?;
                for binding in matched {
                    if binding.nearby_entity_candidates.is_none() {
                        return Err(RuntimeBuildError::declaration(format!(
                            "nearby_swap_move selector requires nearby_entity_candidates for {}::{}",
                            binding.entity_type_name, binding.variable_name,
                        )));
                    }
                    leaves.push(DescriptorLeafSelector::NearbySwap(
                        DescriptorNearbySwapMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            max_nearby: nearby_swap.max_nearby,
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::PillarChangeMoveSelector(pillar_change) => {
                let matched = find_binding(
                    bindings,
                    pillar_change.target.entity_class.as_deref(),
                    pillar_change.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "pillar_change_move",
                    pillar_change.target.entity_class.as_deref(),
                    pillar_change.target.variable_name.as_deref(),
                    &matched,
                )?;
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::PillarChange(
                        DescriptorPillarChangeMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            minimum_sub_pillar_size: pillar_change.minimum_sub_pillar_size,
                            maximum_sub_pillar_size: pillar_change.maximum_sub_pillar_size,
                            value_candidate_limit: pillar_change.value_candidate_limit,
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::PillarSwapMoveSelector(pillar_swap) => {
                let matched = find_binding(
                    bindings,
                    pillar_swap.target.entity_class.as_deref(),
                    pillar_swap.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "pillar_swap_move",
                    pillar_swap.target.entity_class.as_deref(),
                    pillar_swap.target.variable_name.as_deref(),
                    &matched,
                )?;
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::PillarSwap(
                        DescriptorPillarSwapMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            minimum_sub_pillar_size: pillar_swap.minimum_sub_pillar_size,
                            maximum_sub_pillar_size: pillar_swap.maximum_sub_pillar_size,
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::RuinRecreateMoveSelector(ruin_recreate) => {
                validate_ruin_recreate_bounds(
                    ruin_recreate.min_ruin_count,
                    ruin_recreate.max_ruin_count,
                )?;
                let matched = find_binding(
                    bindings,
                    ruin_recreate.target.entity_class.as_deref(),
                    ruin_recreate.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "ruin_recreate_move",
                    ruin_recreate.target.entity_class.as_deref(),
                    ruin_recreate.target.variable_name.as_deref(),
                    &matched,
                )?;
                for binding in matched {
                    if ruin_recreate.recreate_heuristic_type
                        == RecreateHeuristicType::CheapestInsertion
                        && binding.candidate_values.is_none()
                        && ruin_recreate.value_candidate_limit.is_none()
                    {
                        return Err(RuntimeBuildError::declaration(format!(
                            "cheapest_insertion descriptor-driven ruin_recreate requires candidate_values or value_candidate_limit for {}::{}",
                            binding.entity_type_name, binding.variable_name,
                        )));
                    }
                    let carry = match scoped_seed(
                        random_seed,
                        binding.descriptor_index,
                        binding.variable_name,
                        "descriptor_ruin_recreate_move_selector",
                    ) {
                        Some(seed) => {
                            crate::heuristic::selector::rng_advance::carry_from_seed(seed)
                        }
                        None => rand::rng().random(),
                    };
                    leaves.push(DescriptorLeafSelector::RuinRecreate(
                        DescriptorRuinRecreateMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            min_ruin_count: ruin_recreate.min_ruin_count,
                            max_ruin_count: ruin_recreate.max_ruin_count,
                            moves_per_step: ruin_recreate.moves_per_step.unwrap_or(10).max(1),
                            value_candidate_limit: ruin_recreate.value_candidate_limit,
                            recreate_heuristic_type: ruin_recreate.recreate_heuristic_type,
                            rng_carry: std::sync::atomic::AtomicU64::new(carry),
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::UnionMoveSelector(union) => {
                for child in &union.selectors {
                    collect::<S>(child, descriptor, bindings, random_seed, leaves)?;
                }
            }
            MoveSelectorConfig::LimitedNeighborhood(_) => {
                return Err(unsupported(
                    "limited_neighborhood must be handled by the canonical runtime",
                ));
            }
            MoveSelectorConfig::ListChangeMoveSelector(_)
            | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
            | MoveSelectorConfig::ListSwapMoveSelector(_)
            | MoveSelectorConfig::ListPermuteMoveSelector(_)
            | MoveSelectorConfig::ListPrecedenceMoveSelector(_)
            | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
            | MoveSelectorConfig::SublistChangeMoveSelector(_)
            | MoveSelectorConfig::SublistSwapMoveSelector(_)
            | MoveSelectorConfig::ListReverseMoveSelector(_)
            | MoveSelectorConfig::KOptMoveSelector(_)
            | MoveSelectorConfig::ListRuinMoveSelector(_) => {
                return Err(unsupported(
                    "list move selector configured against a scalar-variable model",
                ));
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                return Err(unsupported(
                    "nested cartesian_product move selectors are not supported inside descriptor cartesian children",
                ));
            }
            MoveSelectorConfig::ConflictRepairMoveSelector(_) => {
                return Err(unsupported(
                    "conflict_repair_move_selector must be handled by the canonical runtime",
                ));
            }
            MoveSelectorConfig::CompoundConflictRepairMoveSelector(_) => {
                return Err(unsupported(
                    "compound_conflict_repair_move_selector must be handled by the canonical runtime",
                ));
            }
            MoveSelectorConfig::GroupedScalarMoveSelector(_) => {
                return Err(unsupported(
                    "grouped_scalar_move_selector must be handled by the canonical runtime",
                ));
            }
        }
        Ok(())
    }

    match config {
        Some(cfg) => collect::<S>(cfg, descriptor, &bindings, random_seed, &mut leaves)?,
        None => {
            for binding in bindings {
                leaves.push(DescriptorLeafSelector::Change(
                    DescriptorChangeMoveSelector::new(binding.clone(), descriptor.clone(), None),
                ));
                leaves.push(DescriptorLeafSelector::Swap(
                    DescriptorSwapMoveSelector::new(binding, descriptor.clone()),
                ));
            }
        }
    }

    if leaves.is_empty() {
        return Err(no_neighborhoods());
    }

    let selection_order = match config {
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
        _ => solverforge_config::UnionSelectionOrder::default(),
    };
    Ok(VecUnionSelector::with_selection_order(
        leaves,
        selection_order,
    ))
}

fn unsupported(message: &str) -> RuntimeBuildError {
    RuntimeBuildError::declaration(message)
}

fn no_neighborhoods() -> RuntimeBuildError {
    RuntimeBuildError::declaration("move selector configuration produced no scalar neighborhoods")
}

/// Builds the descriptor-driven scalar selector tree for `config`.
///
/// Fails with [`RuntimeBuildError::Declaration`] for selectors this scalar
/// tree cannot build (list, limited, grouped and conflict-repair selectors
/// belong to the canonical runtime), for a target that matches no scalar
/// variable, and for a missing nearby or candidate hook.
pub fn build_descriptor_move_selector<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
    random_seed: Option<u64>,
) -> Result<DescriptorSelector<S>, RuntimeBuildError>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn selector_requires_score_during_move(config: &MoveSelectorConfig) -> bool {
        match config {
            MoveSelectorConfig::RuinRecreateMoveSelector(_) => true,
            MoveSelectorConfig::LimitedNeighborhood(limit) => {
                selector_requires_score_during_move(limit.selector.as_ref())
            }
            MoveSelectorConfig::UnionMoveSelector(union) => union
                .selectors
                .iter()
                .any(selector_requires_score_during_move),
            MoveSelectorConfig::CartesianProductMoveSelector(_) => true,
            _ => false,
        }
    }

    fn check_cartesian_left_preview_safe(
        config: &MoveSelectorConfig,
    ) -> Result<(), RuntimeBuildError> {
        if selector_requires_score_during_move(config) {
            return Err(RuntimeBuildError::declaration(
                "cartesian_product left child cannot contain ruin_recreate_move_selector because preview directors do not calculate scores",
            ));
        }
        Ok(())
    }

    fn collect_nodes<S>(
        config: Option<&MoveSelectorConfig>,
        descriptor: &SolutionDescriptor,
        random_seed: Option<u64>,
        nodes: &mut Vec<DescriptorSelectorNode<S>>,
    ) -> Result<(), RuntimeBuildError>
    where
        S: PlanningSolution + 'static,
        S::Score: Score,
    {
        match config {
            Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
                for child in &union.selectors {
                    collect_nodes::<S>(Some(child), descriptor, random_seed, nodes)?;
                }
            }
            Some(MoveSelectorConfig::CartesianProductMoveSelector(cartesian)) => {
                if cartesian.selectors.len() != 2 {
                    return Err(RuntimeBuildError::declaration(format!(
                        "cartesian_product move selector requires exactly two child selectors, found {}",
                        cartesian.selectors.len()
                    )));
                }
                check_cartesian_left_preview_safe(&cartesian.selectors[0])?;
                let left = build_descriptor_flat_selector::<S>(
                    Some(&cartesian.selectors[0]),
                    descriptor,
                    random_seed,
                )?;
                let right = build_descriptor_flat_selector::<S>(
                    Some(&cartesian.selectors[1]),
                    descriptor,
                    random_seed,
                )?;
                nodes.push(DescriptorSelectorNode::Cartesian(
                    CartesianProductSelector::new(left, right)
                        .with_require_hard_improvement(cartesian.require_hard_improvement),
                ));
            }
            other => {
                let flat = build_descriptor_flat_selector::<S>(other, descriptor, random_seed)?;
                nodes.extend(
                    flat.into_selectors()
                        .into_iter()
                        .map(DescriptorSelectorNode::Leaf),
                );
            }
        }
        Ok(())
    }

    let mut nodes = Vec::new();
    collect_nodes::<S>(config, descriptor, random_seed, &mut nodes)?;
    if nodes.is_empty() {
        return Err(no_neighborhoods());
    }
    let selection_order = match config {
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
        _ => solverforge_config::UnionSelectionOrder::default(),
    };
    Ok(VecUnionSelector::with_selection_order(
        nodes,
        selection_order,
    ))
}
