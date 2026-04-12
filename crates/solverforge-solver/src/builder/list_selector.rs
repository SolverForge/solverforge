// List variable move selector enum and builder.

use std::fmt::Debug;

use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{ListMoveImpl, MoveArena};
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::k_opt::KOptConfig;
use crate::heuristic::selector::move_selector::{
    ListMoveKOptSelector, ListMoveListChangeSelector, ListMoveListRuinSelector,
    ListMoveNearbyKOptSelector, MoveSelector,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::context::IntraDistanceAdapter;
use crate::heuristic::selector::{
    FromSolutionEntitySelector, ListMoveListReverseSelector, ListMoveListSwapSelector,
    ListMoveNearbyListChangeSelector, ListMoveNearbyListSwapSelector,
    ListMoveSubListChangeSelector, ListMoveSubListSwapSelector,
};

use super::context::ListContext;

/// A monomorphized leaf selector for list planning variables.
///
/// Each variant wraps one of the available list move selector wrapper types.
/// Allows `VecUnionSelector<S, ListMoveImpl<S, V>, ListLeafSelector<S, V, DM, IDM>>` to have
/// a single concrete type regardless of configuration.
pub enum ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S>,
{
    // Nearby list change (distance-pruned relocation).
    NearbyListChange(ListMoveNearbyListChangeSelector<S, V, DM, FromSolutionEntitySelector>),
    // Nearby list swap (distance-pruned swap).
    NearbyListSwap(ListMoveNearbyListSwapSelector<S, V, DM, FromSolutionEntitySelector>),
    // List reverse (2-opt).
    ListReverse(ListMoveListReverseSelector<S, V, FromSolutionEntitySelector>),
    // Sublist change (Or-opt).
    SubListChange(ListMoveSubListChangeSelector<S, V, FromSolutionEntitySelector>),
    // K-opt.
    KOpt(ListMoveKOptSelector<S, V, FromSolutionEntitySelector>),
    // Nearby k-opt (distance-pruned).
    NearbyKOpt(
        ListMoveNearbyKOptSelector<S, V, IntraDistanceAdapter<IDM>, FromSolutionEntitySelector>,
    ),
    // List ruin (LNS).
    ListRuin(ListMoveListRuinSelector<S, V>),
    // Full list change (unrestricted relocation).
    ListChange(ListMoveListChangeSelector<S, V, FromSolutionEntitySelector>),
    // Full list swap (unrestricted swap).
    ListSwap(ListMoveListSwapSelector<S, V, FromSolutionEntitySelector>),
    // Sublist swap.
    SubListSwap(ListMoveSubListSwapSelector<S, V, FromSolutionEntitySelector>),
}

impl<S, V, DM, IDM> Debug for ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NearbyListChange(s) => write!(f, "ListLeafSelector::NearbyListChange({s:?})"),
            Self::NearbyListSwap(s) => write!(f, "ListLeafSelector::NearbyListSwap({s:?})"),
            Self::ListReverse(s) => write!(f, "ListLeafSelector::ListReverse({s:?})"),
            Self::SubListChange(s) => write!(f, "ListLeafSelector::SubListChange({s:?})"),
            Self::KOpt(s) => write!(f, "ListLeafSelector::KOpt({s:?})"),
            Self::NearbyKOpt(s) => write!(f, "ListLeafSelector::NearbyKOpt({s:?})"),
            Self::ListRuin(s) => write!(f, "ListLeafSelector::ListRuin({s:?})"),
            Self::ListChange(s) => write!(f, "ListLeafSelector::ListChange({s:?})"),
            Self::ListSwap(s) => write!(f, "ListLeafSelector::ListSwap({s:?})"),
            Self::SubListSwap(s) => write!(f, "ListLeafSelector::SubListSwap({s:?})"),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, ListMoveImpl<S, V>> for ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S> + 'static,
{
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = ListMoveImpl<S, V>> + 'a {
        enum ListLeafIter<A, B, C, DIter, E, F, G, H, I, J> {
            NearbyListChange(A),
            NearbyListSwap(B),
            ListReverse(C),
            SubListChange(DIter),
            KOpt(E),
            NearbyKOpt(F),
            ListRuin(G),
            ListChange(H),
            ListSwap(I),
            SubListSwap(J),
        }

        impl<T, A, B, C, DIter, E, F, G, H, I, J> Iterator
            for ListLeafIter<A, B, C, DIter, E, F, G, H, I, J>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
            C: Iterator<Item = T>,
            DIter: Iterator<Item = T>,
            E: Iterator<Item = T>,
            F: Iterator<Item = T>,
            G: Iterator<Item = T>,
            H: Iterator<Item = T>,
            I: Iterator<Item = T>,
            J: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::NearbyListChange(iter) => iter.next(),
                    Self::NearbyListSwap(iter) => iter.next(),
                    Self::ListReverse(iter) => iter.next(),
                    Self::SubListChange(iter) => iter.next(),
                    Self::KOpt(iter) => iter.next(),
                    Self::NearbyKOpt(iter) => iter.next(),
                    Self::ListRuin(iter) => iter.next(),
                    Self::ListChange(iter) => iter.next(),
                    Self::ListSwap(iter) => iter.next(),
                    Self::SubListSwap(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::NearbyListChange(s) => {
                ListLeafIter::NearbyListChange(s.iter_moves(score_director))
            }
            Self::NearbyListSwap(s) => ListLeafIter::NearbyListSwap(s.iter_moves(score_director)),
            Self::ListReverse(s) => ListLeafIter::ListReverse(s.iter_moves(score_director)),
            Self::SubListChange(s) => ListLeafIter::SubListChange(s.iter_moves(score_director)),
            Self::KOpt(s) => ListLeafIter::KOpt(s.iter_moves(score_director)),
            Self::NearbyKOpt(s) => ListLeafIter::NearbyKOpt(s.iter_moves(score_director)),
            Self::ListRuin(s) => ListLeafIter::ListRuin(s.iter_moves(score_director)),
            Self::ListChange(s) => ListLeafIter::ListChange(s.iter_moves(score_director)),
            Self::ListSwap(s) => ListLeafIter::ListSwap(s.iter_moves(score_director)),
            Self::SubListSwap(s) => ListLeafIter::SubListSwap(s.iter_moves(score_director)),
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::NearbyListChange(s) => s.size(score_director),
            Self::NearbyListSwap(s) => s.size(score_director),
            Self::ListReverse(s) => s.size(score_director),
            Self::SubListChange(s) => s.size(score_director),
            Self::KOpt(s) => s.size(score_director),
            Self::NearbyKOpt(s) => s.size(score_director),
            Self::ListRuin(s) => s.size(score_director),
            Self::ListChange(s) => s.size(score_director),
            Self::ListSwap(s) => s.size(score_director),
            Self::SubListSwap(s) => s.size(score_director),
        }
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ListMoveImpl<S, V>>,
    ) {
        match self {
            Self::NearbyListChange(s) => arena.extend(s.iter_moves(score_director)),
            Self::NearbyListSwap(s) => arena.extend(s.iter_moves(score_director)),
            Self::ListReverse(s) => arena.extend(s.iter_moves(score_director)),
            Self::SubListChange(s) => arena.extend(s.iter_moves(score_director)),
            Self::KOpt(s) => arena.extend(s.iter_moves(score_director)),
            Self::NearbyKOpt(s) => arena.extend(s.iter_moves(score_director)),
            Self::ListRuin(s) => arena.extend(s.iter_moves(score_director)),
            Self::ListChange(s) => arena.extend(s.iter_moves(score_director)),
            Self::ListSwap(s) => arena.extend(s.iter_moves(score_director)),
            Self::SubListSwap(s) => arena.extend(s.iter_moves(score_director)),
        }
    }
}

/// Builder that constructs a `VecUnionSelector` of `ListLeafSelector` from config.
pub struct ListMoveSelectorBuilder;

impl ListMoveSelectorBuilder {
    /// Builds a `VecUnionSelector` from move selector config and domain context.
    ///
    /// Default (no config): `Union(NearbyListChange(20), NearbyListSwap(20), ListReverse)`
    pub fn build<S, V, DM, IDM>(
        config: Option<&MoveSelectorConfig>,
        ctx: &ListContext<S, V, DM, IDM>,
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
            "stock move selector configuration produced no list neighborhoods"
        );
        VecUnionSelector::new(leaves)
    }

    fn collect_leaves<S, V, DM, IDM>(
        config: &MoveSelectorConfig,
        ctx: &ListContext<S, V, DM, IDM>,
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
                    "selected_count_limit_move_selector must be handled by the unified stock runtime"
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
                panic!("standard move selector configured against a list-variable stock context");
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                panic!("cartesian_product move selectors are not supported in stock solving");
            }
        }
    }

    fn push_nearby_change<S, V, DM, IDM>(
        out: &mut Vec<ListLeafSelector<S, V, DM, IDM>>,
        ctx: &ListContext<S, V, DM, IDM>,
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
        ctx: &ListContext<S, V, DM, IDM>,
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
        ctx: &ListContext<S, V, DM, IDM>,
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
        ctx: &ListContext<S, V, DM, IDM>,
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
        ctx: &ListContext<S, V, DM, IDM>,
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
        ctx: &ListContext<S, V, DM, IDM>,
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
        ctx: &ListContext<S, V, DM, IDM>,
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
        ctx: &ListContext<S, V, DM, IDM>,
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
        ctx: &ListContext<S, V, DM, IDM>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use solverforge_config::{NearbyListSwapMoveConfig, VariableTargetConfig};
    use solverforge_core::domain::{
        EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
    };
    use solverforge_core::score::SoftScore;
    use solverforge_scoring::ScoreDirector;

    #[derive(Clone, Debug)]
    struct Vehicle {
        visits: Vec<usize>,
    }

    #[derive(Clone, Debug)]
    struct Plan {
        vehicles: Vec<Vehicle>,
        score: Option<SoftScore>,
    }

    impl PlanningSolution for Plan {
        type Score = SoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[derive(Clone, Debug)]
    struct CountingMeter {
        calls: Arc<AtomicUsize>,
    }

    impl CountingMeter {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    calls: calls.clone(),
                },
                calls,
            )
        }
    }

    impl CrossEntityDistanceMeter<Plan> for CountingMeter {
        fn distance(
            &self,
            _solution: &Plan,
            _src_entity: usize,
            _src_pos: usize,
            _dst_entity: usize,
            _dst_pos: usize,
        ) -> f64 {
            self.calls.fetch_add(1, Ordering::SeqCst);
            1.0
        }
    }

    fn descriptor() -> SolutionDescriptor {
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
            EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Vehicle",
                    "vehicles",
                    |s: &Plan| &s.vehicles,
                    |s: &mut Plan| &mut s.vehicles,
                )),
            ),
        )
    }

    fn list_len(s: &Plan, entity_idx: usize) -> usize {
        s.vehicles
            .get(entity_idx)
            .map_or(0, |vehicle| vehicle.visits.len())
    }

    fn list_remove(s: &mut Plan, entity_idx: usize, pos: usize) -> Option<usize> {
        let visits = &mut s.vehicles.get_mut(entity_idx)?.visits;
        if pos < visits.len() {
            Some(visits.remove(pos))
        } else {
            None
        }
    }

    fn list_insert(s: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
        if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
            vehicle.visits.insert(pos, value);
        }
    }

    fn list_get(s: &Plan, entity_idx: usize, pos: usize) -> Option<usize> {
        s.vehicles
            .get(entity_idx)
            .and_then(|vehicle| vehicle.visits.get(pos))
            .copied()
    }

    fn list_set(s: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
        if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
            vehicle.visits[pos] = value;
        }
    }

    fn list_reverse(s: &mut Plan, entity_idx: usize, start: usize, end: usize) {
        if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
            vehicle.visits[start..end].reverse();
        }
    }

    fn sublist_remove(s: &mut Plan, entity_idx: usize, start: usize, end: usize) -> Vec<usize> {
        if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
            vehicle.visits.drain(start..end).collect()
        } else {
            Vec::new()
        }
    }

    fn sublist_insert(s: &mut Plan, entity_idx: usize, pos: usize, values: Vec<usize>) {
        if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
            vehicle.visits.splice(pos..pos, values);
        }
    }

    fn ruin_remove(s: &mut Plan, entity_idx: usize, pos: usize) -> usize {
        s.vehicles[entity_idx].visits.remove(pos)
    }

    fn ruin_insert(s: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
        s.vehicles[entity_idx].visits.insert(pos, value);
    }

    fn entity_count(s: &Plan) -> usize {
        s.vehicles.len()
    }

    #[test]
    fn nearby_list_swap_uses_cross_entity_meter() {
        let (cross_meter, cross_calls) = CountingMeter::new();
        let (intra_meter, intra_calls) = CountingMeter::new();
        let ctx = ListContext::new(
            list_len,
            list_remove,
            list_insert,
            list_get,
            list_set,
            list_reverse,
            sublist_remove,
            sublist_insert,
            ruin_remove,
            ruin_insert,
            entity_count,
            cross_meter,
            intra_meter,
            "visits",
            0,
        );
        let config = MoveSelectorConfig::NearbyListSwapMoveSelector(NearbyListSwapMoveConfig {
            max_nearby: 4,
            target: VariableTargetConfig::default(),
        });
        let selector = ListMoveSelectorBuilder::build(Some(&config), &ctx, None);
        let solution = Plan {
            vehicles: vec![Vehicle { visits: vec![10] }, Vehicle { visits: vec![20] }],
            score: None,
        };
        let director = ScoreDirector::simple(solution, descriptor(), |s, descriptor_index| {
            if descriptor_index == 0 {
                s.vehicles.len()
            } else {
                0
            }
        });

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        assert_eq!(moves.len(), 1, "expected a single inter-entity swap");
        assert!(
            cross_calls.load(Ordering::SeqCst) > 0,
            "nearby_list_swap must evaluate distances through the cross-entity meter"
        );
        assert_eq!(
            intra_calls.load(Ordering::SeqCst),
            0,
            "nearby_list_swap must not consult the intra-route meter"
        );
    }
}
