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
        None => {
            let Some(default_config) =
                crate::builder::search::defaults::default_move_selector_config(model)
            else {
                return;
            };
            collect_neighborhoods(Some(&default_config), model, random_seed, out);
        }
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
    let default_config;
    let effective_config = match config {
        Some(config) => Some(config),
        None => {
            default_config = crate::builder::search::defaults::default_move_selector_config(model);
            default_config.as_ref()
        }
    };
    collect_neighborhoods(effective_config, model, random_seed, &mut neighborhoods);
    assert!(
        !neighborhoods.is_empty(),
        "move selector configuration produced no neighborhoods \
         (scalar_slots_present={}, list_slots_present={}, requested_selector_family={})",
        model.scalar_variables().next().is_some(),
        model.has_list_variables(),
        selector_family_name(effective_config),
    );
    let selection_order = match effective_config {
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
        _ => solverforge_config::UnionSelectionOrder::Sequential,
    };
    VecUnionSelector::with_selection_order(neighborhoods, selection_order)
}

fn build_acceptor_forager_local_search<S, V, DM, IDM>(
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
    if let Some(config) = config {
        assert!(
            config.neighborhoods.is_empty(),
            "acceptor_forager local_search uses move_selector; neighborhoods are only valid with local_search_type = \"variable_neighborhood_descent\""
        );
    }

    let acceptor = config
        .and_then(|ls| ls.acceptor.as_ref())
        .map(|cfg| AcceptorBuilder::build_with_seed::<S>(cfg, random_seed))
        .unwrap_or_else(|| {
            crate::builder::search::defaults::default_local_search_acceptor(model, random_seed)
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
                crate::builder::search::defaults::default_local_search_forager(model)
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

fn build_variable_neighborhood_descent<S, V, DM, IDM>(
    config: &LocalSearchConfig,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> VndPhase<S, NeighborhoodMove<S, V>, Neighborhood<S, V, DM, IDM>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    assert!(
        config.acceptor.is_none() && config.forager.is_none() && config.move_selector.is_none(),
        "variable_neighborhood_descent local_search uses neighborhoods; acceptor, forager, and move_selector are only valid with local_search_type = \"acceptor_forager\""
    );
    assert!(
        !config.neighborhoods.is_empty(),
        "variable_neighborhood_descent local_search requires at least one [[phases.neighborhoods]] block"
    );

    let neighborhoods = config
        .neighborhoods
        .iter()
        .flat_map(|selector| {
            let mut neighborhoods = Vec::new();
            collect_neighborhoods(Some(selector), model, random_seed, &mut neighborhoods);
            neighborhoods
        })
        .collect::<Vec<_>>();
    assert!(
        !neighborhoods.is_empty(),
        "variable_neighborhood_descent local_search neighborhoods produced no move selectors"
    );

    let step_limit = config
        .termination
        .as_ref()
        .and_then(|termination| termination.step_count_limit);

    VndPhase::new(neighborhoods, step_limit)
}

pub fn build_local_search<S, V, DM, IDM>(
    config: Option<&LocalSearchConfig>,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> LocalSearchStrategy<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    match config.map(|ls| ls.local_search_type).unwrap_or_default() {
        LocalSearchType::AcceptorForager => LocalSearchStrategy::acceptor_forager(
            build_acceptor_forager_local_search(config, model, random_seed),
        ),
        LocalSearchType::VariableNeighborhoodDescent => {
            let config = config.expect(
                "variable_neighborhood_descent local_search requires an explicit local_search phase",
            );
            LocalSearchStrategy::variable_neighborhood_descent(build_variable_neighborhood_descent(
                config,
                model,
                random_seed,
            ))
        }
    }
}

#[cfg(test)]
mod tests;
