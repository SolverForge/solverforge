//! Per-step group snapshots for the group cursors and grouped ruin.

use std::fmt;

use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{list_access::ListAccess, RuntimeListSlot};
use crate::heuristic::r#move::list_kernel::MemberCoordinates;
use crate::heuristic::selector::list_kernel::GroupSnapshot;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::list_groups::{JointBounds, ListElementGroups, ListGroupPlacement};
use crate::list_placement::OwnerRestriction;

use super::super::emission::RuntimeListEmitter;
use super::super::group_emission::GroupedRuinEmitter;

/// Every fully assigned group with its members' coordinates and the owners
/// every member allows.
pub(super) fn runtime_group_snapshots<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    solution: &S,
) -> Vec<GroupSnapshot>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let Some(groups) = slot.element_groups(solution) else {
        return Vec::new();
    };
    let placement = slot.locate_group_members(&groups, solution);
    let entity_count = ListAccess::entity_count(slot, solution);
    let explicit = slot.ownership_policy().is_explicit();
    (0..groups.group_count())
        .filter(|&group| placement.group_is_assigned(&groups, group))
        .map(|group| {
            let members: MemberCoordinates = (0..groups.members(group).len())
                .map(|member| {
                    placement
                        .member_position(&groups, group, member)
                        .expect("an assigned group has every member placed")
                })
                .collect();
            let restriction = if explicit {
                members.iter().fold(
                    OwnerRestriction::Unrestricted,
                    |restriction, &(entity, position)| {
                        let element = ListAccess::list_get(slot, solution, entity, position)
                            .expect("a located member is in its list");
                        let owner = ListAccess::element_owner(slot, solution, &element).expect(
                            "compiled owner-aware list leaf requires validated owner metadata",
                        );
                        restriction.and(OwnerRestriction::from_hook_owner(owner, entity_count))
                    },
                )
            } else {
                OwnerRestriction::Unrestricted
            };
            GroupSnapshot {
                members,
                restriction,
            }
        })
        .collect()
}

/// The ruin emitter of a slot with element groups, or `None` without them.
pub(super) fn runtime_grouped_ruin_emitter<'a, S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    move_access: &'a super::super::move_access::RuntimeListMoveAccess<S, V>,
    solution: &S,
    skip_empty_destinations: bool,
    bounds: JointBounds,
) -> Option<GroupedRuinEmitter<'a, S, V, DM, IDM>>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let groups: ListElementGroups = slot.element_groups(solution)?;
    let placement: ListGroupPlacement = slot.locate_group_members(&groups, solution);
    let list_lens: Vec<usize> = (0..ListAccess::entity_count(slot, solution))
        .map(|entity| ListAccess::list_len(slot, solution, entity))
        .collect();
    Some(GroupedRuinEmitter::new(
        RuntimeListEmitter::new(slot, move_access, skip_empty_destinations),
        groups,
        placement,
        &list_lens,
        bounds,
    ))
}
