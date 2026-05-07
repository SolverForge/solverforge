fn build_leaf_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> LeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    let mut leaves = Vec::new();
    match config {
        None => unreachable!("default neighborhoods must be resolved before leaf selection"),
        Some(MoveSelectorConfig::ChangeMoveSelector(_))
        | Some(MoveSelectorConfig::SwapMoveSelector(_))
        | Some(MoveSelectorConfig::NearbyChangeMoveSelector(_))
        | Some(MoveSelectorConfig::NearbySwapMoveSelector(_))
        | Some(MoveSelectorConfig::PillarChangeMoveSelector(_))
        | Some(MoveSelectorConfig::PillarSwapMoveSelector(_))
        | Some(MoveSelectorConfig::RuinRecreateMoveSelector(_)) => {
            push_scalar_selector(config, model, random_seed, &mut leaves);
        }
        Some(MoveSelectorConfig::GroupedScalarMoveSelector(config)) => {
            push_grouped_scalar_selector(config, model, &mut leaves);
        }
        Some(MoveSelectorConfig::ListChangeMoveSelector(_))
        | Some(MoveSelectorConfig::NearbyListChangeMoveSelector(_))
        | Some(MoveSelectorConfig::ListSwapMoveSelector(_))
        | Some(MoveSelectorConfig::NearbyListSwapMoveSelector(_))
        | Some(MoveSelectorConfig::SublistChangeMoveSelector(_))
        | Some(MoveSelectorConfig::SublistSwapMoveSelector(_))
        | Some(MoveSelectorConfig::ListReverseMoveSelector(_))
        | Some(MoveSelectorConfig::KOptMoveSelector(_))
        | Some(MoveSelectorConfig::ListRuinMoveSelector(_)) => {
            push_list_selector(config, model, random_seed, &mut leaves);
        }
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
            for child in &union.selectors {
                match child {
                    MoveSelectorConfig::GroupedScalarMoveSelector(config) => {
                        push_grouped_scalar_selector(config, model, &mut leaves);
                        continue;
                    }
                    MoveSelectorConfig::ConflictRepairMoveSelector(config) => {
                        push_conflict_repair_selector(config, model, &mut leaves);
                        continue;
                    }
                    MoveSelectorConfig::CompoundConflictRepairMoveSelector(config) => {
                        push_compound_conflict_repair_selector(config, model, &mut leaves);
                        continue;
                    }
                    _ => {}
                }
                match selector_family(child) {
                    SelectorFamily::Scalar => {
                        push_scalar_selector(Some(child), model, random_seed, &mut leaves);
                    }
                    SelectorFamily::List => {
                        push_list_selector(Some(child), model, random_seed, &mut leaves);
                    }
                    SelectorFamily::Mixed => {
                        let nested = build_leaf_selector(Some(child), model, random_seed);
                        leaves.extend(nested.into_selectors());
                    }
                    SelectorFamily::Unsupported => {
                        panic!(
                            "cartesian_product move selectors are not supported in the runtime selector graph"
                        );
                    }
                }
            }
        }
        Some(MoveSelectorConfig::LimitedNeighborhood(_)) => {
            panic!("limited_neighborhood must be wrapped at the neighborhood level");
        }
        Some(MoveSelectorConfig::CartesianProductMoveSelector(_)) => {
            panic!(
                "cartesian_product move selectors are not supported in the runtime selector graph"
            );
        }
        Some(MoveSelectorConfig::ConflictRepairMoveSelector(config)) => {
            push_conflict_repair_selector(config, model, &mut leaves);
        }
        Some(MoveSelectorConfig::CompoundConflictRepairMoveSelector(config)) => {
            push_compound_conflict_repair_selector(config, model, &mut leaves);
        }
    }
    assert!(
        !leaves.is_empty(),
        "move selector configuration produced no neighborhoods \
         (scalar_slots_present={}, list_slots_present={}, requested_selector_family={})",
        model.scalar_variables().next().is_some(),
        model.has_list_variables(),
        selector_family_name(config),
    );
    let selection_order = match config {
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
        _ => solverforge_config::UnionSelectionOrder::Sequential,
    };
    VecUnionSelector::with_selection_order(leaves, selection_order)
}

fn build_cartesian_child_selector<S, V, DM, IDM>(
    config: &MoveSelectorConfig,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    match config {
        MoveSelectorConfig::LimitedNeighborhood(limit) => CartesianChildSelector::Limited {
            selector: build_leaf_selector(Some(limit.selector.as_ref()), model, random_seed),
            selected_count_limit: limit.selected_count_limit,
        },
        MoveSelectorConfig::CartesianProductMoveSelector(_) => {
            panic!("nested cartesian_product move selectors are not supported")
        }
        other => CartesianChildSelector::Flat(build_leaf_selector(Some(other), model, random_seed)),
    }
}

fn wrap_neighborhood_composite<S, V>(
    mov: SequentialCompositeMove<S, NeighborhoodMove<S, V>>,
) -> NeighborhoodMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    NeighborhoodMove::Composite(mov)
}

fn default_scalar_change_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
        value_candidate_limit: None,
        target: VariableTargetConfig::default(),
    })
}

fn default_scalar_swap_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::SwapMoveSelector(solverforge_config::SwapMoveConfig {
        target: VariableTargetConfig::default(),
    })
}

fn default_nearby_list_change_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::NearbyListChangeMoveSelector(NearbyListChangeMoveConfig {
        max_nearby: 20,
        target: VariableTargetConfig::default(),
    })
}

fn default_nearby_list_swap_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::NearbyListSwapMoveSelector(NearbyListSwapMoveConfig {
        max_nearby: 20,
        target: VariableTargetConfig::default(),
    })
}

fn default_list_reverse_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
        target: VariableTargetConfig::default(),
    })
}

fn collect_default_neighborhoods<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
    out: &mut Vec<Neighborhood<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    if model.has_list_variables() {
        let list_change = default_nearby_list_change_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&list_change),
            model,
            random_seed,
        )));

        let list_swap = default_nearby_list_swap_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&list_swap),
            model,
            random_seed,
        )));

        let list_reverse = default_list_reverse_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&list_reverse),
            model,
            random_seed,
        )));
    }

    if model.scalar_variables().next().is_some() {
        let scalar_change = default_scalar_change_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&scalar_change),
            model,
            random_seed,
        )));

        let scalar_swap = default_scalar_swap_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&scalar_swap),
            model,
            random_seed,
        )));
    }
}

fn collect_neighborhoods<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
    out: &mut Vec<Neighborhood<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    match config {
        None => collect_default_neighborhoods(model, random_seed, out),
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
            for child in &union.selectors {
                collect_neighborhoods(Some(child), model, random_seed, out);
            }
        }
        Some(MoveSelectorConfig::LimitedNeighborhood(limit)) => {
            let selector = build_leaf_selector(Some(limit.selector.as_ref()), model, random_seed);
            out.push(Neighborhood::Limited {
                selector,
                selected_count_limit: limit.selected_count_limit,
            });
        }
        Some(MoveSelectorConfig::ConflictRepairMoveSelector(config)) => {
            let mut leaves = Vec::new();
            push_conflict_repair_selector(config, model, &mut leaves);
            out.push(Neighborhood::Flat(VecUnionSelector::new(leaves)));
        }
        Some(MoveSelectorConfig::CompoundConflictRepairMoveSelector(config)) => {
            let mut leaves = Vec::new();
            push_compound_conflict_repair_selector(config, model, &mut leaves);
            out.push(Neighborhood::Flat(VecUnionSelector::new(leaves)));
        }
        Some(MoveSelectorConfig::GroupedScalarMoveSelector(config)) => {
            let mut leaves = Vec::new();
            push_grouped_scalar_selector(config, model, &mut leaves);
            out.push(Neighborhood::Flat(VecUnionSelector::new(leaves)));
        }
        Some(MoveSelectorConfig::CartesianProductMoveSelector(cartesian)) => {
            assert_eq!(
                cartesian.selectors.len(),
                2,
                "cartesian_product move selector requires exactly two child selectors"
            );
            assert_cartesian_left_preview_safe(&cartesian.selectors[0]);
            let left = build_cartesian_child_selector(&cartesian.selectors[0], model, random_seed);
            let right = build_cartesian_child_selector(&cartesian.selectors[1], model, random_seed);
            out.push(Neighborhood::Cartesian(
                CartesianProductSelector::new(left, right, wrap_neighborhood_composite::<S, V>)
                    .with_require_hard_improvement(cartesian.require_hard_improvement),
            ));
        }
        Some(other) => out.push(Neighborhood::Flat(build_leaf_selector(
            Some(other),
            model,
            random_seed,
        ))),
    }
}

pub fn build_move_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> Selector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let mut neighborhoods = Vec::new();
    collect_neighborhoods(config, model, random_seed, &mut neighborhoods);
    assert!(
        !neighborhoods.is_empty(),
        "move selector configuration produced no neighborhoods \
         (scalar_slots_present={}, list_slots_present={}, requested_selector_family={})",
        model.scalar_variables().next().is_some(),
        model.has_list_variables(),
        selector_family_name(config),
    );
    let selection_order = match config {
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
        _ => solverforge_config::UnionSelectionOrder::Sequential,
    };
    VecUnionSelector::with_selection_order(neighborhoods, selection_order)
}

pub fn build_local_search<S, V, DM, IDM>(
    config: Option<&LocalSearchConfig>,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> LocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let acceptor = config
        .and_then(|ls| ls.acceptor.as_ref())
        .map(|cfg| AcceptorBuilder::build_with_seed::<S>(cfg, random_seed))
        .unwrap_or_else(|| {
            if model.has_list_variables() {
                AnyAcceptor::LateAcceptance(
                    crate::phase::localsearch::LateAcceptanceAcceptor::<S>::new(400),
                )
            } else {
                match random_seed {
                    Some(seed) => AnyAcceptor::SimulatedAnnealing(
                        SimulatedAnnealingAcceptor::auto_calibrate_with_seed(0.999985, seed),
                    ),
                    None => AnyAcceptor::SimulatedAnnealing(SimulatedAnnealingAcceptor::default()),
                }
            }
        });
    let forager = config
        .and_then(|ls| ls.forager.as_ref())
        .map(|cfg| ForagerBuilder::build::<S>(Some(cfg)))
        .unwrap_or_else(|| {
            let is_tabu = config
                .and_then(|ls| ls.acceptor.as_ref())
                .is_some_and(|acceptor| matches!(acceptor, AcceptorConfig::TabuSearch(_)));
            if is_tabu {
                AnyForager::BestScore(crate::phase::localsearch::BestScoreForager::new())
            } else {
                let accepted = if model.has_list_variables() { 4 } else { 1 };
                AnyForager::AcceptedCount(AcceptedCountForager::new(accepted))
            }
        });
    let move_selector = build_move_selector(
        config.and_then(|ls| ls.move_selector.as_ref()),
        model,
        random_seed,
    );
    let step_limit = config
        .and_then(|ls| ls.termination.as_ref())
        .and_then(|termination| termination.step_count_limit);

    LocalSearchPhase::new(move_selector, acceptor, forager, step_limit)
}

pub fn build_vnd<S, V, DM, IDM>(
    config: &VndConfig,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> Vnd<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let neighborhoods = if config.neighborhoods.is_empty() {
        let mut neighborhoods = Vec::new();
        collect_neighborhoods(None, model, random_seed, &mut neighborhoods);
        neighborhoods
    } else {
        config
            .neighborhoods
            .iter()
            .flat_map(|selector| {
                let mut neighborhoods = Vec::new();
                collect_neighborhoods(Some(selector), model, random_seed, &mut neighborhoods);
                neighborhoods
            })
            .collect()
    };

    DynamicVndPhase::new(neighborhoods)
}

#[cfg(test)]
mod tests;
