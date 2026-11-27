//! Group move emission over the shared runtime list emitter.

use std::fmt;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::heuristic::r#move::list_kernel::{
    GroupChangeCoordinates, GroupRuinPlan, GroupSwapCoordinates, RuinSources,
};
use crate::heuristic::selector::list_kernel::{
    GroupChangeEmitter, GroupSwapEmitter, MultiRuinEmitter, RuinEmitter,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::list_groups::{JointBounds, ListElementGroups, ListGroupPlacement};

use super::emission::RuntimeListEmitter;
use super::{RuntimeListMove, RuntimeListRecipe};

impl<'a, S, V, DM, IDM> GroupChangeEmitter<S> for RuntimeListEmitter<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<'a, S, V, DM, IDM>;

    fn emit_group_change(&self, coordinates: GroupChangeCoordinates) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::GroupChange {
            access: self.move_access(),
            coordinates,
        })
    }
}

impl<'a, S, V, DM, IDM> GroupSwapEmitter<S> for RuntimeListEmitter<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<'a, S, V, DM, IDM>;

    fn emit_group_swap(&self, coordinates: GroupSwapCoordinates) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::GroupSwap {
            access: self.move_access(),
            coordinates,
        })
    }
}

/// Ruin emission for a list with element groups: every ruined element takes
/// its whole group along, and recreate places each group jointly.
pub(super) struct GroupedRuinEmitter<'a, S, V, DM, IDM> {
    inner: RuntimeListEmitter<'a, S, V, DM, IDM>,
    groups: ListElementGroups,
    placement: ListGroupPlacement,
    /// Group of the element at `[entity][position]`, `u32::MAX` if none.
    group_at: Vec<Vec<u32>>,
    bounds: JointBounds,
}

impl<'a, S, V, DM, IDM> GroupedRuinEmitter<'a, S, V, DM, IDM> {
    pub(super) fn new(
        inner: RuntimeListEmitter<'a, S, V, DM, IDM>,
        groups: ListElementGroups,
        placement: ListGroupPlacement,
        list_lens: &[usize],
        bounds: JointBounds,
    ) -> Self {
        let mut group_at: Vec<Vec<u32>> =
            list_lens.iter().map(|&len| vec![u32::MAX; len]).collect();
        for group in 0..groups.group_count() {
            for member in 0..groups.members(group).len() {
                if let Some((entity, position)) = placement.member_position(&groups, group, member)
                {
                    group_at[entity][position] =
                        u32::try_from(group).expect("group count fits in u32");
                }
            }
        }
        Self {
            inner,
            groups,
            placement,
            group_at,
            bounds,
        }
    }
}

impl<'a, S, V, DM, IDM> fmt::Debug for GroupedRuinEmitter<'a, S, V, DM, IDM> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GroupedRuinEmitter")
            .field("group_count", &self.groups.group_count())
            .field("bounds", &self.bounds)
            .finish_non_exhaustive()
    }
}

impl<'a, S, V, DM, IDM> GroupedRuinEmitter<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    /// One ruin unit per ungrouped position and per distinct group, in source
    /// order with positions ascending per list.
    fn group_ruin_plan(
        &self,
        sources: impl Iterator<Item = (usize, SmallVec<[usize; 8]>)>,
    ) -> GroupRuinPlan {
        let mut removed = SmallVec::new();
        let mut unit_ends = SmallVec::new();
        let mut seen: SmallVec<[u32; 8]> = SmallVec::new();
        for (entity, mut indices) in sources {
            indices.sort_unstable();
            for index in indices {
                let group = self.group_at[entity][index];
                if group == u32::MAX {
                    removed.push((entity, index));
                } else if seen.contains(&group) {
                    continue;
                } else {
                    seen.push(group);
                    let group = group as usize;
                    removed.extend((0..self.groups.members(group).len()).filter_map(|member| {
                        self.placement.member_position(&self.groups, group, member)
                    }));
                }
                unit_ends.push(u16::try_from(removed.len()).expect("a ruin removes few elements"));
            }
        }
        GroupRuinPlan {
            removed,
            unit_ends,
            bounds: self.bounds,
            skip_empty_destinations: self.inner.skip_empty_destinations(),
        }
    }

    fn group_ruin(&self, plan: GroupRuinPlan) -> RuntimeListMove<'a, S, V, DM, IDM> {
        self.inner.move_from_recipe(RuntimeListRecipe::GroupRuin {
            access: self.inner.move_access(),
            plan,
        })
    }
}

impl<'a, S, V, DM, IDM> RuinEmitter<S> for GroupedRuinEmitter<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<'a, S, V, DM, IDM>;

    fn emit_ruin(&self, entity: usize, indices: SmallVec<[usize; 8]>) -> Self::Move {
        self.group_ruin(self.group_ruin_plan(std::iter::once((entity, indices))))
    }
}

impl<'a, S, V, DM, IDM> MultiRuinEmitter<S> for GroupedRuinEmitter<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn emit_ruin_sources(
        &self,
        sources: RuinSources,
        max_count: usize,
        max_routes: usize,
        eligible: impl Fn(usize, usize) -> bool,
    ) -> Option<Self::Move> {
        let plan = self.group_ruin_plan(sources.into_iter());
        if plan.removed.len() > max_count || plan.entities().len() > max_routes {
            return None;
        }
        // Validate the expanded segments without reordering the recreate units.
        for &(entity, position) in &plan.removed {
            if !eligible(entity, position) {
                return None;
            }
            let first = plan
                .removed
                .iter()
                .all(|&(other, offset)| other != entity || offset >= position);
            if !first && !plan.removed.contains(&(entity, position - 1)) {
                return None;
            }
        }
        Some(self.group_ruin(plan))
    }
}
