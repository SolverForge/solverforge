#[cfg_attr(not(test), allow(dead_code))]
pub fn build_scalar_move_selector<S>(
    config: Option<&MoveSelectorConfig>,
    scalar_variables: &[ScalarVariableContext<S>],
    random_seed: Option<u64>,
) -> ScalarSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn collect_nodes<S: PlanningSolution + 'static>(
        config: Option<&MoveSelectorConfig>,
        scalar_variables: &[ScalarVariableContext<S>],
        random_seed: Option<u64>,
        nodes: &mut Vec<ScalarSelectorNode<S>>,
    ) where
        S::Score: Score,
    {
        match config {
            Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
                for child in &union.selectors {
                    collect_nodes(Some(child), scalar_variables, random_seed, nodes);
                }
            }
            Some(MoveSelectorConfig::CartesianProductMoveSelector(cartesian)) => {
                assert_eq!(
                    cartesian.selectors.len(),
                    2,
                    "cartesian_product move selector requires exactly two child selectors"
                );
                let left = build_scalar_flat_selector(
                    Some(&cartesian.selectors[0]),
                    scalar_variables,
                    random_seed,
                );
                let right = build_scalar_flat_selector(
                    Some(&cartesian.selectors[1]),
                    scalar_variables,
                    random_seed,
                );
                nodes.push(ScalarSelectorNode::Cartesian(
                    CartesianProductSelector::new(left, right, wrap_scalar_composite::<S>),
                ));
            }
            other => {
                let flat = build_scalar_flat_selector(other, scalar_variables, random_seed);
                nodes.extend(
                    flat.into_selectors()
                        .into_iter()
                        .map(ScalarSelectorNode::Leaf),
                );
            }
        }
    }

    let mut nodes = Vec::new();
    collect_nodes(config, scalar_variables, random_seed, &mut nodes);
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

fn build_sub_pillar_config(
    minimum_size: usize,
    maximum_size: usize,
) -> crate::heuristic::selector::SubPillarConfig {
    if minimum_size == 0 || maximum_size == 0 {
        crate::heuristic::selector::SubPillarConfig::none()
    } else {
        crate::heuristic::selector::SubPillarConfig {
            enabled: true,
            minimum_size: minimum_size.max(2),
            maximum_size: maximum_size.max(minimum_size.max(2)),
        }
    }
}

fn collect_scalar_leaf_selectors<S>(
    config: Option<&MoveSelectorConfig>,
    scalar_variables: &[ScalarVariableContext<S>],
    random_seed: Option<u64>,
    leaves: &mut Vec<ScalarLeafSelector<S>>,
) where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn matching_variables<S: PlanningSolution + 'static>(
        scalar_variables: &[ScalarVariableContext<S>],
        entity_class: Option<&str>,
        variable_name: Option<&str>,
    ) -> Vec<ScalarVariableContext<S>> {
        scalar_variables
            .iter()
            .copied()
            .filter(|ctx| ctx.matches_target(entity_class, variable_name))
            .collect()
    }

    fn push_change<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        value_candidate_limit: Option<usize>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        leaves.push(ScalarLeafSelector::Change(
            ChangeMoveSelector::new(
                FromSolutionEntitySelector::new(ctx.descriptor_index),
                ScalarCandidateSelector::new(*ctx, value_candidate_limit),
                ctx.getter,
                ctx.setter,
                ctx.descriptor_index,
                ctx.variable_index,
                ctx.variable_name,
            )
            .with_allows_unassigned(ctx.allows_unassigned),
        ));
    }

    fn push_swap<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        leaves.push(ScalarLeafSelector::Swap(SwapLeafSelector { ctx: *ctx }));
    }

    fn push_nearby_change<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        max_nearby: usize,
        value_candidate_limit: Option<usize>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        assert!(
            ctx.nearby_value_candidates.is_some(),
            "nearby_change_move selector requires nearby_value_candidates for {}::{}",
            ctx.entity_type_name,
            ctx.variable_name,
        );
        leaves.push(ScalarLeafSelector::NearbyChange(NearbyChangeLeafSelector {
            ctx: *ctx,
            max_nearby,
            value_candidate_limit,
        }));
    }

    fn push_nearby_swap<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        max_nearby: usize,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        assert!(
            ctx.nearby_entity_candidates.is_some(),
            "nearby_swap_move selector requires nearby_entity_candidates for {}::{}",
            ctx.entity_type_name,
            ctx.variable_name,
        );
        leaves.push(ScalarLeafSelector::NearbySwap(NearbySwapLeafSelector {
            ctx: *ctx,
            max_nearby,
        }));
    }

    fn push_pillar_change<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
        value_candidate_limit: Option<usize>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        leaves.push(ScalarLeafSelector::PillarChange(PillarChangeLeafSelector {
            ctx: *ctx,
            minimum_sub_pillar_size,
            maximum_sub_pillar_size,
            value_candidate_limit,
        }));
    }

    fn push_pillar_swap<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        leaves.push(ScalarLeafSelector::PillarSwap(PillarSwapLeafSelector {
            ctx: *ctx,
            minimum_sub_pillar_size,
            maximum_sub_pillar_size,
        }));
    }

    fn push_ruin_recreate<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        min_ruin_count: usize,
        max_ruin_count: usize,
        moves_per_step: Option<usize>,
        value_candidate_limit: Option<usize>,
        recreate_heuristic_type: RecreateHeuristicType,
        random_seed: Option<u64>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        if recreate_heuristic_type == RecreateHeuristicType::CheapestInsertion {
            assert!(
                ctx.candidate_values.is_some() || value_candidate_limit.is_some(),
                "cheapest_insertion scalar ruin_recreate requires candidate_values or value_candidate_limit for {}::{}",
                ctx.entity_type_name,
                ctx.variable_name,
            );
        }
        let access = RuinVariableAccess::new(
            ctx.entity_count,
            ctx.getter,
            ctx.setter,
            ctx.variable_index,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        let selector = RuinMoveSelector::new(min_ruin_count, max_ruin_count, access)
            .with_moves_per_step(moves_per_step.unwrap_or(10).max(1));
        let selector = match scoped_seed(
            random_seed,
            ctx.descriptor_index,
            ctx.variable_name,
            "scalar_ruin_recreate_move_selector",
        ) {
            Some(seed) => selector.with_seed(seed),
            None => selector,
        };
        leaves.push(ScalarLeafSelector::RuinRecreate(RuinRecreateLeafSelector {
            selector,
            getter: ctx.getter,
            setter: ctx.setter,
            descriptor_index: ctx.descriptor_index,
            variable_index: ctx.variable_index,
            variable_name: ctx.variable_name,
            value_source: scalar_recreate_candidate_source(*ctx, value_candidate_limit),
            recreate_heuristic_type,
            allows_unassigned: ctx.allows_unassigned,
        }));
    }

    fn require_matches<S: PlanningSolution + 'static>(
        label: &str,
        entity_class: Option<&str>,
        variable_name: Option<&str>,
        matched: &[ScalarVariableContext<S>],
    ) {
        assert!(
            !matched.is_empty(),
            "{label} selector matched no scalar planning variables for entity_class={:?} variable_name={:?}",
            entity_class,
            variable_name,
        );
    }

    fn collect<S: PlanningSolution + 'static>(
        cfg: &MoveSelectorConfig,
        scalar_variables: &[ScalarVariableContext<S>],
        random_seed: Option<u64>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        match cfg {
            MoveSelectorConfig::ChangeMoveSelector(change) => {
                let matched = matching_variables(
                    scalar_variables,
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                );
                require_matches(
                    "change_move",
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_change(&ctx, change.value_candidate_limit, leaves);
                }
            }
            MoveSelectorConfig::SwapMoveSelector(swap) => {
                let matched = matching_variables(
                    scalar_variables,
                    swap.target.entity_class.as_deref(),
                    swap.target.variable_name.as_deref(),
                );
                require_matches(
                    "swap_move",
                    swap.target.entity_class.as_deref(),
                    swap.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_swap(&ctx, leaves);
                }
            }
            MoveSelectorConfig::NearbyChangeMoveSelector(nearby_change) => {
                let matched = matching_variables(
                    scalar_variables,
                    nearby_change.target.entity_class.as_deref(),
                    nearby_change.target.variable_name.as_deref(),
                );
                require_matches(
                    "nearby_change_move",
                    nearby_change.target.entity_class.as_deref(),
                    nearby_change.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_nearby_change(
                        &ctx,
                        nearby_change.max_nearby,
                        nearby_change.value_candidate_limit,
                        leaves,
                    );
                }
            }
            MoveSelectorConfig::NearbySwapMoveSelector(nearby_swap) => {
                let matched = matching_variables(
                    scalar_variables,
                    nearby_swap.target.entity_class.as_deref(),
                    nearby_swap.target.variable_name.as_deref(),
                );
                require_matches(
                    "nearby_swap_move",
                    nearby_swap.target.entity_class.as_deref(),
                    nearby_swap.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_nearby_swap(&ctx, nearby_swap.max_nearby, leaves);
                }
            }
            MoveSelectorConfig::PillarChangeMoveSelector(pillar_change) => {
                let matched = matching_variables(
                    scalar_variables,
                    pillar_change.target.entity_class.as_deref(),
                    pillar_change.target.variable_name.as_deref(),
                );
                require_matches(
                    "pillar_change_move",
                    pillar_change.target.entity_class.as_deref(),
                    pillar_change.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_pillar_change(
                        &ctx,
                        pillar_change.minimum_sub_pillar_size,
                        pillar_change.maximum_sub_pillar_size,
                        pillar_change.value_candidate_limit,
                        leaves,
                    );
                }
            }
            MoveSelectorConfig::PillarSwapMoveSelector(pillar_swap) => {
                let matched = matching_variables(
                    scalar_variables,
                    pillar_swap.target.entity_class.as_deref(),
                    pillar_swap.target.variable_name.as_deref(),
                );
                require_matches(
                    "pillar_swap_move",
                    pillar_swap.target.entity_class.as_deref(),
                    pillar_swap.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_pillar_swap(
                        &ctx,
                        pillar_swap.minimum_sub_pillar_size,
                        pillar_swap.maximum_sub_pillar_size,
                        leaves,
                    );
                }
            }
            MoveSelectorConfig::RuinRecreateMoveSelector(ruin_recreate) => {
                let matched = matching_variables(
                    scalar_variables,
                    ruin_recreate.target.entity_class.as_deref(),
                    ruin_recreate.target.variable_name.as_deref(),
                );
                require_matches(
                    "ruin_recreate_move",
                    ruin_recreate.target.entity_class.as_deref(),
                    ruin_recreate.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_ruin_recreate(
                        &ctx,
                        ruin_recreate.min_ruin_count,
                        ruin_recreate.max_ruin_count,
                        ruin_recreate.moves_per_step,
                        ruin_recreate.value_candidate_limit,
                        ruin_recreate.recreate_heuristic_type,
                        random_seed,
                        leaves,
                    );
                }
            }
            MoveSelectorConfig::UnionMoveSelector(union) => {
                for child in &union.selectors {
                    collect(child, scalar_variables, random_seed, leaves);
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
                panic!("nested cartesian_product move selectors are not supported inside scalar cartesian children");
            }
        }
    }

    match config {
        Some(cfg) => collect(cfg, scalar_variables, random_seed, leaves),
        None => {
            for ctx in scalar_variables {
                push_change(ctx, None, leaves);
                push_swap(ctx, leaves);
            }
        }
    }
}

#[cfg(test)]
mod tests;
