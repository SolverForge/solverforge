//! Group move emission over the shared runtime list emitter.

use std::fmt;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::heuristic::r#move::list_kernel::{
    GroupChangeCoordinates, GroupRuinPlan, GroupSwapCoordinates,
};
use crate::heuristic::selector::list_kernel::{GroupChangeEmitter, GroupSwapEmitter, RuinEmitter};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::list_groups::{JointBounds, ListElementGroups, ListGroupPlacement};

use super::emission::RuntimeListEmitter;
use super::{RuntimeListMove, RuntimeListRecipe};

impl<S, V, DM, IDM> GroupChangeEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_group_change(&self, coordinates: GroupChangeCoordinates) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::GroupChange {
            access: self.move_access().clone(),
            coordinates,
        })
    }
}

impl<S, V, DM, IDM> GroupSwapEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_group_swap(&self, coordinates: GroupSwapCoordinates) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::GroupSwap {
            access: self.move_access().clone(),
            coordinates,
        })
    }
}

/// Ruin emission for a list with element groups: every ruined element takes
/// its whole group along, and recreate places each group jointly.
pub(super) struct GroupedRuinEmitter<S, V, DM, IDM> {
    inner: RuntimeListEmitter<S, V, DM, IDM>,
    groups: ListElementGroups,
    placement: ListGroupPlacement,
    /// Group of the element at `[entity][position]`, `u32::MAX` if none.
    group_at: Vec<Vec<u32>>,
    bounds: JointBounds,
}

impl<S, V, DM, IDM> GroupedRuinEmitter<S, V, DM, IDM> {
    pub(super) fn new(
        inner: RuntimeListEmitter<S, V, DM, IDM>,
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

impl<S, V, DM, IDM> fmt::Debug for GroupedRuinEmitter<S, V, DM, IDM> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GroupedRuinEmitter")
            .field("group_count", &self.groups.group_count())
            .field("bounds", &self.bounds)
            .finish_non_exhaustive()
    }
}

impl<S, V, DM, IDM> RuinEmitter<S> for GroupedRuinEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_ruin(&self, entity: usize, indices: &[usize]) -> Self::Move {
        let mut removed = SmallVec::new();
        let mut unit_ends = SmallVec::new();
        let mut seen: SmallVec<[u32; 8]> = SmallVec::new();
        let mut sorted: SmallVec<[usize; 8]> = SmallVec::from_slice(indices);
        sorted.sort_unstable();
        for index in sorted {
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
        self.inner.move_from_recipe(RuntimeListRecipe::GroupRuin {
            access: self.inner.move_access().clone(),
            plan: GroupRuinPlan {
                removed,
                unit_ends,
                bounds: self.bounds,
                skip_empty_destinations: self.inner.skip_empty_destinations(),
            },
        })
    }
}
