/// Builder that constructs a `VecUnionSelector` of `ListLeafSelector` from config.
pub struct ListMoveSelectorBuilder;

impl ListMoveSelectorBuilder {
    fn selector_requires_score_during_move(config: &MoveSelectorConfig) -> bool {
        match config {
            MoveSelectorConfig::ListRuinMoveSelector(_) => true,
            MoveSelectorConfig::LimitedNeighborhood(limit) => {
                Self::selector_requires_score_during_move(limit.selector.as_ref())
            }
            MoveSelectorConfig::UnionMoveSelector(union) => union
                .selectors
                .iter()
                .any(Self::selector_requires_score_during_move),
            MoveSelectorConfig::CartesianProductMoveSelector(_) => true,
            _ => false,
        }
    }

    fn assert_cartesian_left_preview_safe(config: &MoveSelectorConfig) {
        assert!(
            !Self::selector_requires_score_during_move(config),
            "cartesian_product left child cannot contain list_ruin_move_selector because preview directors do not calculate scores",
        );
    }

    /// Builds a top-level list move selector from move selector config and domain context.
    ///
    /// Default (no config): `Union(NearbyListChange(20), NearbyListSwap(20), ListReverse)`
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn build<S, V, DM, IDM>(
        config: Option<&MoveSelectorConfig>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        random_seed: Option<u64>,
    ) -> VecUnionSelector<S, ListMoveUnion<S, V>, ListSelectorNode<S, V, DM, IDM>>
    where
        S: PlanningSolution + 'static,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
        IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    {
        fn collect_nodes<S, V, DM, IDM>(
            config: Option<&MoveSelectorConfig>,
            ctx: &ListVariableContext<S, V, DM, IDM>,
            random_seed: Option<u64>,
            nodes: &mut Vec<ListSelectorNode<S, V, DM, IDM>>,
        ) where
            S: PlanningSolution + 'static,
            V: Clone + PartialEq + Send + Sync + Debug + 'static,
            DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
            IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
        {
            match config {
                Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
                    for child in &union.selectors {
                        collect_nodes(Some(child), ctx, random_seed, nodes);
                    }
                }
                Some(MoveSelectorConfig::CartesianProductMoveSelector(cartesian)) => {
                    assert_eq!(
                        cartesian.selectors.len(),
                        2,
                        "cartesian_product move selector requires exactly two child selectors"
                    );
                    ListMoveSelectorBuilder::assert_cartesian_left_preview_safe(
                        &cartesian.selectors[0],
                    );
                    let left = ListMoveSelectorBuilder::build_flat(
                        Some(&cartesian.selectors[0]),
                        ctx,
                        random_seed,
                    );
                    let right = ListMoveSelectorBuilder::build_flat(
                        Some(&cartesian.selectors[1]),
                        ctx,
                        random_seed,
                    );
                    nodes.push(ListSelectorNode::Cartesian(CartesianProductSelector::new(
                        left,
                        right,
                        wrap_list_composite::<S, V>,
                    )));
                }
                other => {
                    let flat = ListMoveSelectorBuilder::build_flat(other, ctx, random_seed);
                    nodes.extend(
                        flat.into_selectors()
                            .into_iter()
                            .map(ListSelectorNode::Leaf),
                    );
                }
            }
        }

        let mut nodes = Vec::new();
        collect_nodes(config, ctx, random_seed, &mut nodes);
        assert!(
            !nodes.is_empty(),
            "move selector configuration produced no list neighborhoods"
        );
        let selection_order = match config {
            Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
            _ => solverforge_config::UnionSelectionOrder::Sequential,
        };
        VecUnionSelector::with_selection_order(nodes, selection_order)
    }

    pub fn build_flat<S, V, DM, IDM>(
        config: Option<&MoveSelectorConfig>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        random_seed: Option<u64>,
    ) -> ListFlatSelector<S, V, DM, IDM>
    where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        let mut leaves: Vec<ListLeafSelector<S, V, DM, IDM>> = Vec::new();
        match config {
            None => {
                Self::push_nearby_change(&mut leaves, ctx, 20);
                Self::push_nearby_swap(&mut leaves, ctx, 20);
                Self::push_list_reverse(&mut leaves, ctx);
            }
            Some(cfg) => Self::collect_leaves(cfg, ctx, random_seed, &mut leaves),
        }
        assert!(
            !leaves.is_empty(),
            "move selector configuration produced no list neighborhoods"
        );
        let selection_order = match config {
            Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
            _ => solverforge_config::UnionSelectionOrder::Sequential,
        };
        VecUnionSelector::with_selection_order(leaves, selection_order)
    }

    fn collect_leaves<S, V, DM, IDM>(
        config: &MoveSelectorConfig,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        random_seed: Option<u64>,
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        match config {
            MoveSelectorConfig::NearbyListChangeMoveSelector(c) => {
                Self::push_nearby_change(out, ctx, c.max_nearby);
            }
            MoveSelectorConfig::NearbyListSwapMoveSelector(c) => {
                Self::push_nearby_swap(out, ctx, c.max_nearby);
            }
            MoveSelectorConfig::ListReverseMoveSelector(_) => {
                Self::push_list_reverse(out, ctx);
            }
            MoveSelectorConfig::SublistChangeMoveSelector(c) => {
                Self::push_sublist_change(out, ctx, c.min_sublist_size, c.max_sublist_size);
            }
            MoveSelectorConfig::SublistSwapMoveSelector(c) => {
                Self::push_sublist_swap(out, ctx, c.min_sublist_size, c.max_sublist_size);
            }
            MoveSelectorConfig::KOptMoveSelector(c) => {
                Self::push_kopt(out, ctx, c.k, c.min_segment_len, c.max_nearby);
            }
            MoveSelectorConfig::ListRuinMoveSelector(c) => {
                Self::push_list_ruin(
                    out,
                    ctx,
                    c.min_ruin_count,
                    c.max_ruin_count,
                    c.moves_per_step,
                    random_seed,
                );
            }
            MoveSelectorConfig::LimitedNeighborhood(_) => {
                panic!("limited_neighborhood must be handled by the canonical runtime");
            }
            MoveSelectorConfig::ListChangeMoveSelector(_) => {
                Self::push_list_change(out, ctx);
            }
            MoveSelectorConfig::ListSwapMoveSelector(_) => {
                Self::push_list_swap(out, ctx);
            }
            MoveSelectorConfig::UnionMoveSelector(u) => {
                for child in &u.selectors {
                    Self::collect_leaves(child, ctx, random_seed, out);
                }
            }
            MoveSelectorConfig::ChangeMoveSelector(_)
            | MoveSelectorConfig::SwapMoveSelector(_)
            | MoveSelectorConfig::NearbyChangeMoveSelector(_)
            | MoveSelectorConfig::NearbySwapMoveSelector(_)
            | MoveSelectorConfig::PillarChangeMoveSelector(_)
            | MoveSelectorConfig::PillarSwapMoveSelector(_)
            | MoveSelectorConfig::RuinRecreateMoveSelector(_) => {
                panic!("scalar move selector configured against a list-variable context");
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                panic!("nested cartesian_product move selectors are not supported inside list cartesian children");
            }
            MoveSelectorConfig::ConflictRepairMoveSelector(_) => {
                panic!("conflict_repair_move_selector must be handled by the canonical runtime");
            }
        }
    }

    fn push_nearby_change<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        max_nearby: usize,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        use crate::heuristic::selector::nearby_list_change::NearbyListChangeMoveSelector;

        let inner = NearbyListChangeMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.cross_distance_meter.clone(),
            max_nearby,
            ctx.list_len,
            ctx.list_get,
            ctx.list_remove,
            ctx.list_insert,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::NearbyListChange(inner));
    }

    fn push_nearby_swap<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        max_nearby: usize,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        use crate::heuristic::selector::nearby_list_swap::NearbyListSwapMoveSelector;

        let inner = NearbyListSwapMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.cross_distance_meter.clone(),
            max_nearby,
            ctx.list_len,
            ctx.list_get,
            ctx.list_set,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::NearbyListSwap(inner));
    }

    fn push_list_reverse<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        use crate::heuristic::selector::list_reverse::ListReverseMoveSelector;

        let inner = ListReverseMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.list_len,
            ctx.list_get,
            ctx.list_reverse,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::ListReverse(inner));
    }

    fn push_sublist_change<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        min_sublist_size: usize,
        max_sublist_size: usize,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        use crate::heuristic::selector::sublist_change::SublistChangeMoveSelector;

        let inner = SublistChangeMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            min_sublist_size,
            max_sublist_size,
            ctx.list_len,
            ctx.list_get,
            ctx.sublist_remove,
            ctx.sublist_insert,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::SublistChange(inner));
    }

    fn push_sublist_swap<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        min_sublist_size: usize,
        max_sublist_size: usize,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        use crate::heuristic::selector::sublist_swap::SublistSwapMoveSelector;

        let inner = SublistSwapMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            min_sublist_size,
            max_sublist_size,
            ctx.list_len,
            ctx.list_get,
            ctx.sublist_remove,
            ctx.sublist_insert,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::SublistSwap(inner));
    }

    fn push_kopt<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        k: usize,
        min_segment_len: usize,
        max_nearby: usize,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        use crate::heuristic::selector::k_opt::{KOptMoveSelector, NearbyKOptMoveSelector};

        let config = KOptConfig::new(k.clamp(2, 5)).with_min_segment_len(min_segment_len);
        if max_nearby > 0 {
            let adapter = IntraDistanceAdapter(ctx.intra_distance_meter.clone());
            let inner = NearbyKOptMoveSelector::new(
                FromSolutionEntitySelector::new(ctx.descriptor_index),
                adapter,
                max_nearby,
                config,
                ctx.list_len,
                ctx.list_get,
                ctx.sublist_remove,
                ctx.sublist_insert,
                ctx.variable_name,
                ctx.descriptor_index,
            );
            out.push(ListLeafSelector::NearbyKOpt(inner));
        } else {
            let inner = KOptMoveSelector::new(
                FromSolutionEntitySelector::new(ctx.descriptor_index),
                config,
                ctx.list_len,
                ctx.list_get,
                ctx.sublist_remove,
                ctx.sublist_insert,
                ctx.variable_name,
                ctx.descriptor_index,
            );
            out.push(ListLeafSelector::KOpt(inner));
        }
    }

    fn push_list_ruin<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        min_ruin_count: usize,
        max_ruin_count: usize,
        moves_per_step: Option<usize>,
        random_seed: Option<u64>,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        use crate::heuristic::selector::list_ruin::ListRuinMoveSelector;

        let inner = ListRuinMoveSelector::new(
            min_ruin_count,
            max_ruin_count,
            ctx.entity_count,
            ctx.list_len,
            ctx.list_get,
            ctx.ruin_remove,
            ctx.ruin_insert,
            ctx.variable_name,
            ctx.descriptor_index,
        )
        .with_moves_per_step(moves_per_step.unwrap_or(10).max(1));
        let inner = if let Some(seed) = random_seed {
            inner.with_seed(seed)
        } else {
            inner
        };
        out.push(ListLeafSelector::ListRuin(inner));
    }

    fn push_list_change<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
    {
        use crate::heuristic::selector::list_change::ListChangeMoveSelector;

        let inner = ListChangeMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.list_len,
            ctx.list_get,
            ctx.list_remove,
            ctx.list_insert,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::ListChange(inner));
    }

    fn push_list_swap<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone,
    {
        use crate::heuristic::selector::list_swap::ListSwapMoveSelector;

        let inner = ListSwapMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.list_len,
            ctx.list_get,
            ctx.list_set,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::ListSwap(inner));
    }
}
