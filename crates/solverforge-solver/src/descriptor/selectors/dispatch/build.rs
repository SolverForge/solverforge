fn build_descriptor_flat_selector<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
    random_seed: Option<u64>,
) -> DescriptorFlatSelector<S>
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
    ) where
        S: PlanningSolution + 'static,
        S::Score: Score,
    {
        assert!(
            !matched.is_empty(),
            "{label} selector matched no scalar planning variables for entity_class={:?} variable_name={:?}",
            entity_class,
            variable_name,
        );
    }

    fn collect<S>(
        cfg: &MoveSelectorConfig,
        descriptor: &SolutionDescriptor,
        bindings: &[VariableBinding],
        random_seed: Option<u64>,
        leaves: &mut Vec<DescriptorLeafSelector<S>>,
    ) where
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
                );
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
                );
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
                );
                for binding in matched {
                    assert!(
                        binding.nearby_value_candidates.is_some(),
                        "nearby_change_move selector requires nearby_value_candidates for {}::{}",
                        binding.entity_type_name,
                        binding.variable_name,
                    );
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
                );
                for binding in matched {
                    assert!(
                        binding.nearby_entity_candidates.is_some(),
                        "nearby_swap_move selector requires nearby_entity_candidates for {}::{}",
                        binding.entity_type_name,
                        binding.variable_name,
                    );
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
                );
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
                );
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
                );
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
                );
                for binding in matched {
                    if ruin_recreate.recreate_heuristic_type == RecreateHeuristicType::CheapestInsertion {
                        assert!(
                            binding.candidate_values.is_some()
                                || ruin_recreate.value_candidate_limit.is_some(),
                            "cheapest_insertion descriptor-driven ruin_recreate requires candidate_values or value_candidate_limit for {}::{}",
                            binding.entity_type_name,
                            binding.variable_name,
                        );
                    }
                    let rng = match scoped_seed(
                        random_seed,
                        binding.descriptor_index,
                        binding.variable_name,
                        "descriptor_ruin_recreate_move_selector",
                    ) {
                        Some(seed) => SmallRng::seed_from_u64(seed),
                        None => SmallRng::from_rng(&mut rand::rng()),
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
                            rng: RefCell::new(rng),
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::UnionMoveSelector(union) => {
                for child in &union.selectors {
                    collect::<S>(child, descriptor, bindings, random_seed, leaves);
                }
            }
            MoveSelectorConfig::LimitedNeighborhood(_) => {
                panic!("limited_neighborhood must be handled by the canonical runtime");
            }
            MoveSelectorConfig::ListChangeMoveSelector(_)
            | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
            | MoveSelectorConfig::ListSwapMoveSelector(_)
            | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
            | MoveSelectorConfig::SublistChangeMoveSelector(_)
            | MoveSelectorConfig::SublistSwapMoveSelector(_)
            | MoveSelectorConfig::ListReverseMoveSelector(_)
            | MoveSelectorConfig::KOptMoveSelector(_)
            | MoveSelectorConfig::ListRuinMoveSelector(_) => {
                panic!("list move selector configured against a scalar-variable context");
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                panic!(
                    "nested cartesian_product move selectors are not supported inside descriptor cartesian children"
                );
            }
            MoveSelectorConfig::ConflictRepairMoveSelector(_) => {
                panic!("conflict_repair_move_selector must be handled by the canonical runtime");
            }
            MoveSelectorConfig::CompoundConflictRepairMoveSelector(_) => {
                panic!(
                    "compound_conflict_repair_move_selector must be handled by the canonical runtime"
                );
            }
            MoveSelectorConfig::GroupedScalarMoveSelector(_) => {
                panic!("grouped_scalar_move_selector must be handled by the canonical runtime");
            }
        }
    }

    match config {
        Some(cfg) => collect::<S>(cfg, descriptor, &bindings, random_seed, &mut leaves),
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

    assert!(
        !leaves.is_empty(),
        "move selector configuration produced no scalar neighborhoods"
    );

    let selection_order = match config {
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
        _ => solverforge_config::UnionSelectionOrder::Sequential,
    };
    VecUnionSelector::with_selection_order(leaves, selection_order)
}

pub fn build_descriptor_move_selector<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
    random_seed: Option<u64>,
) -> DescriptorSelector<S>
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

    fn assert_cartesian_left_preview_safe(config: &MoveSelectorConfig) {
        assert!(
            !selector_requires_score_during_move(config),
            "cartesian_product left child cannot contain ruin_recreate_move_selector because preview directors do not calculate scores",
        );
    }

    fn collect_nodes<S>(
        config: Option<&MoveSelectorConfig>,
        descriptor: &SolutionDescriptor,
        random_seed: Option<u64>,
        nodes: &mut Vec<DescriptorSelectorNode<S>>,
    ) where
        S: PlanningSolution + 'static,
        S::Score: Score,
    {
        match config {
            Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
                for child in &union.selectors {
                    collect_nodes::<S>(Some(child), descriptor, random_seed, nodes);
                }
            }
            Some(MoveSelectorConfig::CartesianProductMoveSelector(cartesian)) => {
                assert_eq!(
                    cartesian.selectors.len(),
                    2,
                    "cartesian_product move selector requires exactly two child selectors"
                );
                assert_cartesian_left_preview_safe(&cartesian.selectors[0]);
                let left = build_descriptor_flat_selector::<S>(
                    Some(&cartesian.selectors[0]),
                    descriptor,
                    random_seed,
                );
                let right = build_descriptor_flat_selector::<S>(
                    Some(&cartesian.selectors[1]),
                    descriptor,
                    random_seed,
                );
                nodes.push(DescriptorSelectorNode::Cartesian(
                    CartesianProductSelector::new(left, right, wrap_descriptor_composite::<S>)
                        .with_require_hard_improvement(cartesian.require_hard_improvement),
                ));
            }
            other => {
                let flat = build_descriptor_flat_selector::<S>(other, descriptor, random_seed);
                nodes.extend(
                    flat.into_selectors()
                        .into_iter()
                        .map(DescriptorSelectorNode::Leaf),
                );
            }
        }
    }

    let mut nodes = Vec::new();
    collect_nodes::<S>(config, descriptor, random_seed, &mut nodes);
    assert!(
        !nodes.is_empty(),
        "move selector configuration produced no scalar neighborhoods"
    );
    let selection_order = match config {
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
        _ => solverforge_config::UnionSelectionOrder::Sequential,
    };
    VecUnionSelector::with_selection_order(nodes, selection_order)
}
