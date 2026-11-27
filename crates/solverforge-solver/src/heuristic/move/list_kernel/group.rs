//! Shared group change and group swap mechanics.
//!
//! A group's members move together and keep their declared order. Group
//! change takes every member out, wherever it is, and inserts the members at
//! increasing final positions of one destination list; other elements may
//! sit between them. Group swap exchanges two equally sized groups on two
//! owners, each taking the other's member slots.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{
    encode_usize, hash_str, MoveTabuScope, ScopedEntityTabuToken, ScopedValueTabuToken,
};
use crate::heuristic::r#move::MoveTabuSignature;
use crate::list_groups::JointPositions;
use crate::stats::CandidateTraceIdentity;

use super::{ListChangeAccess, ListMoveAccess};

mod swap;

pub(crate) use swap::{
    group_swap_candidate_trace_identity, group_swap_do_move, group_swap_is_doable,
    group_swap_tabu_signature, group_swap_undo_move,
};

/// `(entity, position)` of every member of a group, in declared order.
pub(crate) type MemberCoordinates = SmallVec<[(usize, usize); 4]>;

/// One group relocated to joint positions of a destination list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GroupChangeCoordinates {
    /// Where every member sits before the move, in declared order.
    pub(crate) sources: MemberCoordinates,
    pub(crate) destination_entity: usize,
    /// The members' final positions in the destination list, increasing in
    /// declared order.
    pub(crate) destination_positions: JointPositions,
}

/// Two equally sized groups exchanging their member slots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GroupSwapCoordinates {
    pub(crate) first_entity: usize,
    /// Positions of the first group's members, in declared order.
    pub(crate) first_positions: JointPositions,
    pub(crate) second_entity: usize,
    /// Positions of the second group's members, in declared order.
    pub(crate) second_positions: JointPositions,
}

impl GroupChangeCoordinates {
    /// Every entity the move touches, ascending.
    pub(crate) fn entities(&self) -> SmallVec<[usize; 8]> {
        let mut entities: SmallVec<[usize; 8]> =
            self.sources.iter().map(|&(entity, _)| entity).collect();
        entities.push(self.destination_entity);
        entities.sort_unstable();
        entities.dedup();
        entities
    }

    /// Whether the move leaves every member where it is.
    pub(crate) fn is_identity(&self) -> bool {
        self.sources.iter().zip(&self.destination_positions).all(
            |(&(entity, position), &destination)| {
                entity == self.destination_entity && position == destination
            },
        )
    }
}

impl GroupSwapCoordinates {
    pub(crate) fn entities(&self) -> SmallVec<[usize; 8]> {
        let mut entities: SmallVec<[usize; 8]> = smallvec![self.first_entity, self.second_entity];
        entities.sort_unstable();
        entities
    }
}

fn sorted(positions: &[usize]) -> JointPositions {
    let mut sorted = JointPositions::from_slice(positions);
    sorted.sort_unstable();
    sorted
}

fn entities_are_free<S: PlanningSolution, D: Director<S>>(
    score_director: &D,
    descriptor_index: usize,
    entities: &[usize],
) -> bool {
    entities
        .iter()
        .all(|&entity| !crate::pinning::entity_is_pinned(score_director, descriptor_index, entity))
}

fn positions_are_distinct_and_in_bounds(positions: &[usize], len: usize) -> bool {
    let sorted = sorted(positions);
    sorted.last().is_some_and(|&last| last < len) && sorted.windows(2).all(|w| w[0] < w[1])
}

pub(crate) fn group_change_is_doable<S, A, D>(
    access: &A,
    coordinates: &GroupChangeCoordinates,
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let member_count = coordinates.sources.len();
    if member_count == 0
        || coordinates.destination_positions.len() != member_count
        || coordinates.is_identity()
    {
        return false;
    }
    let solution = score_director.working_solution();
    let sources = &coordinates.sources;
    if has_duplicate(sources)
        || sources
            .iter()
            .any(|&(entity, position)| position >= access.list_len(solution, entity))
    {
        return false;
    }
    let staying = sources
        .iter()
        .filter(|&&(entity, _)| entity == coordinates.destination_entity)
        .count();
    let final_len =
        access.list_len(solution, coordinates.destination_entity) - staying + member_count;
    coordinates
        .destination_positions
        .windows(2)
        .all(|pair| pair[0] < pair[1])
        && coordinates
            .destination_positions
            .last()
            .is_some_and(|&last| last < final_len)
        && entities_are_free(
            score_director,
            access.descriptor_index(),
            &coordinates.entities(),
        )
}

fn notify_before<S: PlanningSolution, D: Director<S>>(
    score_director: &mut D,
    descriptor_index: usize,
    entities: &[usize],
) {
    for &entity in entities {
        score_director.before_variable_changed(descriptor_index, entity);
    }
}

fn notify_after<S: PlanningSolution, D: Director<S>>(
    score_director: &mut D,
    descriptor_index: usize,
    entities: &[usize],
) {
    for &entity in entities {
        score_director.after_variable_changed(descriptor_index, entity);
    }
}

/// Removes the members at `sources` and returns them in declared order.
fn take_members<S, A, D>(
    access: &A,
    sources: &[(usize, usize)],
    score_director: &mut D,
) -> SmallVec<[A::Element; 4]>
where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let mut order: SmallVec<[usize; 4]> = (0..sources.len()).collect();
    // Later positions first, so earlier positions stay valid.
    order.sort_unstable_by(|&left, &right| {
        let (left_entity, left_position) = sources[left];
        let (right_entity, right_position) = sources[right];
        left_entity
            .cmp(&right_entity)
            .then(right_position.cmp(&left_position))
    });
    let mut values: SmallVec<[Option<A::Element>; 4]> = (0..sources.len()).map(|_| None).collect();
    for member in order {
        let (entity, position) = sources[member];
        values[member] = Some(
            access
                .list_remove(score_director.working_solution_mut(), entity, position)
                .expect("group member position must hold the member"),
        );
    }
    values
        .into_iter()
        .map(|value| value.expect("every member was taken"))
        .collect()
}

/// Inserts `values` (declared order) at `targets`, earlier positions first
/// within each entity, so each value lands exactly at its target.
fn place_members<S, A, D>(
    access: &A,
    values: SmallVec<[A::Element; 4]>,
    targets: &[(usize, usize)],
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let mut slots: SmallVec<[Option<A::Element>; 4]> = values.into_iter().map(Some).collect();
    let mut order: SmallVec<[usize; 4]> = (0..targets.len()).collect();
    order.sort_unstable_by_key(|&member| targets[member]);
    for member in order {
        let (entity, position) = targets[member];
        access.list_insert(
            score_director.working_solution_mut(),
            entity,
            position,
            slots[member].take().expect("each member is placed once"),
        );
    }
}

fn destination_targets(coordinates: &GroupChangeCoordinates) -> MemberCoordinates {
    coordinates
        .destination_positions
        .iter()
        .map(|&position| (coordinates.destination_entity, position))
        .collect()
}

pub(crate) fn group_change_do_move<S, A, D>(
    access: &A,
    coordinates: &GroupChangeCoordinates,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    let entities = coordinates.entities();
    notify_before(score_director, descriptor_index, &entities);
    let values = take_members(access, &coordinates.sources, score_director);
    place_members(
        access,
        values,
        &destination_targets(coordinates),
        score_director,
    );
    notify_after(score_director, descriptor_index, &entities);
}

pub(crate) fn group_change_undo_move<S, A, D>(
    access: &A,
    coordinates: &GroupChangeCoordinates,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    let entities = coordinates.entities();
    notify_before(score_director, descriptor_index, &entities);
    let values = take_members(access, &destination_targets(coordinates), score_director);
    place_members(access, values, &coordinates.sources, score_director);
    notify_after(score_director, descriptor_index, &entities);
}

fn member_value_ids<S, A, D>(
    access: &A,
    targets: &[(usize, usize)],
    score_director: &D,
) -> SmallVec<[u64; 4]>
where
    S: PlanningSolution,
    A: ListMoveAccess<S>,
    D: Director<S>,
{
    let solution = score_director.working_solution();
    targets
        .iter()
        .map(|&(entity, position)| {
            access.tabu_value_id(
                solution,
                access.list_get(solution, entity, position).as_ref(),
            )
        })
        .collect()
}

fn entity_tokens(scope: MoveTabuScope, entities: &[usize]) -> SmallVec<[ScopedEntityTabuToken; 2]> {
    entities
        .iter()
        .map(|&entity| scope.entity_token(encode_usize(entity)))
        .collect()
}

fn value_tokens(scope: MoveTabuScope, ids: &[u64]) -> SmallVec<[ScopedValueTabuToken; 2]> {
    ids.iter().map(|&id| scope.value_token(id)).collect()
}

fn push_coordinates(id: &mut SmallVec<[u64; 8]>, coordinates: &[(usize, usize)]) {
    for &(entity, position) in coordinates {
        id.push(encode_usize(entity));
        id.push(encode_usize(position));
    }
}

pub(crate) fn group_change_tabu_signature<S, A, D>(
    access: &A,
    coordinates: &GroupChangeCoordinates,
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListMoveAccess<S>,
    D: Director<S>,
{
    let value_ids = member_value_ids(access, &coordinates.sources, score_director);
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let header = || -> SmallVec<[u64; 8]> {
        smallvec![
            encode_usize(access.descriptor_index()),
            hash_str(access.variable_name()),
            encode_usize(coordinates.sources.len())
        ]
    };
    let targets = destination_targets(coordinates);
    let mut forward = header();
    push_coordinates(&mut forward, &coordinates.sources);
    push_coordinates(&mut forward, &targets);
    forward.extend(value_ids.iter().copied());
    let mut backward = header();
    push_coordinates(&mut backward, &targets);
    push_coordinates(&mut backward, &coordinates.sources);
    backward.extend(value_ids.iter().copied());
    MoveTabuSignature::from_identity(scope, forward, backward)
        .with_entity_tokens(entity_tokens(scope, &coordinates.entities()))
        .with_destination_value_tokens(value_tokens(scope, &value_ids))
}

pub(crate) fn group_change_candidate_trace_identity<S, A>(
    access: &A,
    coordinates: &GroupChangeCoordinates,
) -> CandidateTraceIdentity
where
    A: ListMoveAccess<S>,
{
    CandidateTraceIdentity::logical_move(
        access.descriptor_index(),
        access.variable_name(),
        "list_group_change",
        coordinates
            .sources
            .iter()
            .flat_map(|&(entity, position)| [entity, position])
            .chain(std::iter::once(coordinates.destination_entity))
            .chain(coordinates.destination_positions.iter().copied()),
    )
}

/// Whether two of the (few) member coordinates coincide.
pub(super) fn has_duplicate(coordinates: &[(usize, usize)]) -> bool {
    coordinates
        .iter()
        .enumerate()
        .any(|(index, coordinate)| coordinates[index + 1..].contains(coordinate))
}
