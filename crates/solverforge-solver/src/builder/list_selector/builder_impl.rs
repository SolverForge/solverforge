use std::fmt::Debug;

use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::ListMoveImpl;
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::k_opt::KOptConfig;
use crate::heuristic::selector::move_selector::{
    ListMoveKOptSelector, ListMoveListChangeSelector, ListMoveListRuinSelector,
    ListMoveNearbyKOptSelector,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::{
    FromSolutionEntitySelector, ListMoveListReverseSelector, ListMoveListSwapSelector,
    ListMoveNearbyListChangeSelector, ListMoveNearbyListSwapSelector,
    ListMoveSubListChangeSelector, ListMoveSubListSwapSelector,
};

use super::super::context::{IntraDistanceAdapter, ListVariableContext};
use super::leaf::ListLeafSelector;

/// Builder that constructs a `VecUnionSelector` of `ListLeafSelector` from config.
pub struct ListMoveSelectorBuilder;

impl ListMoveSelectorBuilder {
    /// Builds a `VecUnionSelector` from move selector config and domain context.
    ///
    /// Default (no config): `Union(NearbyListChange(20), NearbyListSwap(20), ListReverse)`
    pub fn build<S, V, DM, IDM>(
        config: Option<&MoveSelectorConfig>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        random_seed: Option<u64>,
    ) -> VecUnionSelector<S, ListMoveImpl<S, V>, ListLeafSelector<S, V, DM, IDM>>
    where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone,
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
        VecUnionSelector::new(leaves)
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
        IDM: CrossEntityDistanceMeter<S> + Clone,
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
            MoveSelectorConfig::SubListChangeMoveSelector(c) => {
                Self::push_sublist_change(out, ctx, c.min_sublist_size, c.max_sublist_size);
            }
            MoveSelectorConfig::SubListSwapMoveSelector(c) => {
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
            MoveSelectorConfig::SelectedCountLimitMoveSelector(_) => {
                panic!(
                    "selected_count_limit_move_selector must be handled by the canonical runtime"
                );
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
            MoveSelectorConfig::ChangeMoveSelector(_) | MoveSelectorConfig::SwapMoveSelector(_) => {
                panic!("standard move selector configured against a list-variable context");
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                panic!("cartesian_product move selectors are not supported in the canonical solver path");
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
        IDM: CrossEntityDistanceMeter<S> + Clone,
    {
        use crate::heuristic::selector::nearby_list_change::NearbyListChangeMoveSelector;

        let inner = NearbyListChangeMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.cross_distance_meter.clone(),
            max_nearby,
            ctx.list_len,
            ctx.list_remove,
            ctx.list_insert,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::NearbyListChange(
            ListMoveNearbyListChangeSelector::new(inner),
        ));
    }

    fn push_nearby_swap<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
        max_nearby: usize,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone,
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
        out.push(ListLeafSelector::NearbyListSwap(
            ListMoveNearbyListSwapSelector::new(inner),
        ));
    }

    fn push_list_reverse<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone,
    {
        use crate::heuristic::selector::list_reverse::ListReverseMoveSelector;

        let inner = ListReverseMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.list_len,
            ctx.list_reverse,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::ListReverse(
            ListMoveListReverseSelector::new(inner),
        ));
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
        IDM: CrossEntityDistanceMeter<S> + Clone,
    {
        use crate::heuristic::selector::sublist_change::SubListChangeMoveSelector;

        let inner = SubListChangeMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            min_sublist_size,
            max_sublist_size,
            ctx.list_len,
            ctx.sublist_remove,
            ctx.sublist_insert,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::SubListChange(
            ListMoveSubListChangeSelector::new(inner),
        ));
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
        IDM: CrossEntityDistanceMeter<S> + Clone,
    {
        use crate::heuristic::selector::sublist_swap::SubListSwapMoveSelector;

        let inner = SubListSwapMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            min_sublist_size,
            max_sublist_size,
            ctx.list_len,
            ctx.sublist_remove,
            ctx.sublist_insert,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::SubListSwap(
            ListMoveSubListSwapSelector::new(inner),
        ));
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
        IDM: CrossEntityDistanceMeter<S> + Clone,
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
                ctx.sublist_remove,
                ctx.sublist_insert,
                ctx.variable_name,
                ctx.descriptor_index,
            );
            out.push(ListLeafSelector::NearbyKOpt(
                ListMoveNearbyKOptSelector::new(inner),
            ));
        } else {
            let inner = KOptMoveSelector::new(
                FromSolutionEntitySelector::new(ctx.descriptor_index),
                config,
                ctx.list_len,
                ctx.sublist_remove,
                ctx.sublist_insert,
                ctx.variable_name,
                ctx.descriptor_index,
            );
            out.push(ListLeafSelector::KOpt(ListMoveKOptSelector::new(inner)));
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
        IDM: CrossEntityDistanceMeter<S> + Clone,
    {
        use crate::heuristic::selector::list_ruin::ListRuinMoveSelector;

        let inner = ListRuinMoveSelector::new(
            min_ruin_count.max(1),
            max_ruin_count.max(1),
            ctx.entity_count,
            ctx.list_len,
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
        out.push(ListLeafSelector::ListRuin(ListMoveListRuinSelector::new(
            inner,
        )));
    }

    fn push_list_change<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListVariableContext<S, V, DM, IDM>,
    ) where
        S: PlanningSolution,
        V: Clone + PartialEq + Send + Sync + Debug + 'static,
        DM: CrossEntityDistanceMeter<S> + Clone,
        IDM: CrossEntityDistanceMeter<S> + Clone,
    {
        use crate::heuristic::selector::list_change::ListChangeMoveSelector;

        let inner = ListChangeMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.list_len,
            ctx.list_remove,
            ctx.list_insert,
            ctx.variable_name,
            ctx.descriptor_index,
        );
        out.push(ListLeafSelector::ListChange(
            ListMoveListChangeSelector::new(inner),
        ));
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
        out.push(ListLeafSelector::ListSwap(ListMoveListSwapSelector::new(
            inner,
        )));
    }
}
