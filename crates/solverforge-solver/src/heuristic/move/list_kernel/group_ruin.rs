//! Ruin and recreate of whole element groups.
//!
//! A ruin removes every member of each selected group, wherever it sits.
//! Recreate then inserts the units in the plan's order, each at its best
//! bounded joint placement on the owners all its members allow, given the
//! units placed before it. Units are whole groups or single ungrouped
//! elements; members of a unit always land on one owner, in declared order.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{encode_usize, hash_str, MoveTabuScope};
use crate::heuristic::r#move::MoveTabuSignature;
use crate::list_groups::{search_joint_placement, JointBounds, JointSearch};
use crate::list_placement::OwnerRestriction;
use crate::stats::CandidateTraceIdentity;

use super::ruin::{final_positions_after_ordered_insertions, restore_values};
use super::{ListMoveAccess, ListRuinAccess, RuinUndo};

/// The elements one group ruin removes, grouped into recreate units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GroupRuinPlan {
    /// `(entity, position)` of every removed element: units one after
    /// another, each unit's members in declared order.
    pub(crate) removed: SmallVec<[(usize, usize); 8]>,
    /// End offset in `removed` of each unit.
    pub(crate) unit_ends: SmallVec<[u16; 8]>,
    pub(crate) bounds: JointBounds,
    pub(crate) skip_empty_destinations: bool,
}

impl GroupRuinPlan {
    pub(crate) fn entities(&self) -> SmallVec<[usize; 8]> {
        let mut entities: SmallVec<[usize; 8]> =
            self.removed.iter().map(|&(entity, _)| entity).collect();
        entities.sort_unstable();
        entities.dedup();
        entities
    }

    fn units(&self) -> impl Iterator<Item = std::ops::Range<usize>> + '_ {
        let mut start = 0;
        self.unit_ends.iter().map(move |&end| {
            let range = start..usize::from(end);
            start = usize::from(end);
            range
        })
    }
}

pub(crate) fn group_ruin_is_doable<S, A, D>(
    access: &A,
    plan: &GroupRuinPlan,
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    if plan.removed.is_empty() {
        return false;
    }
    let solution = score_director.working_solution();
    let descriptor_index = access.descriptor_index();
    let mut removed = plan.removed.clone();
    removed.sort_unstable();
    if removed.windows(2).any(|pair| pair[0] == pair[1])
        || removed
            .iter()
            .any(|&(entity, position)| position >= access.list_len(solution, entity))
        || plan.entities().iter().any(|&entity| {
            crate::pinning::entity_is_pinned(score_director, descriptor_index, entity)
        })
    {
        return false;
    }
    let entity_count = access.entity_count(solution);
    plan.units().all(|unit| {
        let restriction = unit_restriction(access, solution, entity_count, &plan.removed[unit]);
        (0..entity_count).any(|destination| {
            restriction.allows(destination)
                && !crate::pinning::entity_is_pinned(score_director, descriptor_index, destination)
        })
    })
}

fn unit_restriction<S, A>(
    access: &A,
    solution: &S,
    entity_count: usize,
    members: &[(usize, usize)],
) -> OwnerRestriction
where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
{
    if !access.has_owner_binding() {
        return OwnerRestriction::Unrestricted;
    }
    members.iter().fold(
        OwnerRestriction::Unrestricted,
        |restriction, &(entity, position)| {
            let element = access
                .list_get(solution, entity, position)
                .expect("a ruined member position holds the member");
            restriction.and(access.owner_restriction(solution, entity_count, &element))
        },
    )
}

pub(crate) fn group_ruin_do_move<S, A, D>(
    access: &A,
    plan: &GroupRuinPlan,
    score_director: &mut D,
) -> RuinUndo
where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    let entity_count = access.entity_count(score_director.working_solution());
    let restrictions: SmallVec<[OwnerRestriction; 8]> = plan
        .units()
        .map(|unit| {
            unit_restriction(
                access,
                score_director.working_solution(),
                entity_count,
                &plan.removed[unit],
            )
        })
        .collect();

    // Remove every member, later positions of an entity first.
    let mut order: SmallVec<[usize; 8]> = (0..plan.removed.len()).collect();
    order.sort_unstable_by(|&left, &right| {
        let (left_entity, left_position) = plan.removed[left];
        let (right_entity, right_position) = plan.removed[right];
        left_entity
            .cmp(&right_entity)
            .then(right_position.cmp(&left_position))
    });
    let mut values: SmallVec<[Option<A::Element>; 8]> =
        (0..plan.removed.len()).map(|_| None).collect();
    let mut current_entity = None;
    for index in order {
        let (entity, position) = plan.removed[index];
        if current_entity != Some(entity) {
            if let Some(previous) = current_entity {
                score_director.after_variable_changed(descriptor_index, previous);
            }
            score_director.before_variable_changed(descriptor_index, entity);
            current_entity = Some(entity);
        }
        values[index] =
            Some(access.list_remove(score_director.working_solution_mut(), entity, position));
    }
    if let Some(entity) = current_entity {
        score_director.after_variable_changed(descriptor_index, entity);
    }

    // One pass in the plan's unit order: each unit takes its best bounded
    // joint placement given the units placed before it.
    let mut placements = RuinUndo::new();
    for (unit_index, unit) in plan.units().enumerate() {
        let owners: SmallVec<[usize; 16]> = (0..entity_count)
            .filter(|&destination| {
                restrictions[unit_index].allows(destination)
                    && !crate::pinning::entity_is_pinned(
                        score_director,
                        descriptor_index,
                        destination,
                    )
                    && !(plan.skip_empty_destinations
                        && !access.has_owner_binding()
                        && access.list_len(score_director.working_solution(), destination) == 0)
            })
            .collect();
        let lens: SmallVec<[usize; 16]> = (0..entity_count)
            .map(|entity| access.list_len(score_director.working_solution(), entity))
            .collect();
        let mut trial = JointTrial::new(
            values[unit.clone()]
                .iter_mut()
                .map(|value| value.take().expect("an unplaced member is held"))
                .collect(),
        );
        let search = search_joint_placement(
            &owners,
            unit.len(),
            |entity| lens[entity],
            plan.bounds,
            |entity, positions| Some(trial.score(access, entity, positions, score_director)),
        );
        for (slot, member) in values[unit.clone()]
            .iter_mut()
            .zip(trial.into_members(access, score_director))
        {
            *slot = Some(member);
        }
        let JointSearch::Done {
            best: Some(choice), ..
        } = search
        else {
            restore_unplaced(access, plan, &placements, &mut values, score_director);
            return RuinUndo::new();
        };
        score_director.before_variable_changed(descriptor_index, choice.entity);
        for (removed_index, &position) in unit.zip(choice.positions.iter()) {
            access.list_insert(
                score_director.working_solution_mut(),
                choice.entity,
                position,
                values[removed_index]
                    .take()
                    .expect("the placed unit's members are held"),
            );
            placements.push((choice.entity, position, removed_index));
        }
        score_director.after_variable_changed(descriptor_index, choice.entity);
    }
    placements
}

/// The members of one unit while its joint placements are scored.
///
/// Members stay in the list between trials: the search enumerates an
/// owner's placements in lexicographic order of positions, so consecutive
/// placements usually share the positions of their first members, and only
/// the members after the shared prefix are taken out and put back, in one
/// owner change per trial.
struct JointTrial<E> {
    // Each member, or `None` while it sits in `entity`'s list.
    members: SmallVec<[Option<E>; 4]>,
    entity: Option<usize>,
    // Positions of the inserted members, a prefix of the unit.
    positions: SmallVec<[usize; 4]>,
}

impl<E> JointTrial<E> {
    fn new(members: SmallVec<[E; 4]>) -> Self {
        Self {
            members: members.into_iter().map(Some).collect(),
            entity: None,
            positions: SmallVec::new(),
        }
    }

    /// Places the members at `positions` of `entity` and scores the result.
    fn score<S, A, D>(
        &mut self,
        access: &A,
        entity: usize,
        positions: &[usize],
        score_director: &mut D,
    ) -> S::Score
    where
        S: PlanningSolution,
        A: ListRuinAccess<S, Element = E>,
        D: Director<S>,
    {
        if self.entity != Some(entity) {
            self.take_out(access, score_director);
        }
        let shared = self
            .positions
            .iter()
            .zip(positions)
            .take_while(|(inserted, wanted)| inserted == wanted)
            .count();
        let descriptor_index = access.descriptor_index();
        score_director.before_variable_changed(descriptor_index, entity);
        // Later members sit at higher positions, so they leave first.
        for member in (shared..self.positions.len()).rev() {
            self.members[member] = Some(access.list_remove(
                score_director.working_solution_mut(),
                entity,
                self.positions[member],
            ));
        }
        self.positions.truncate(shared);
        for (member, &position) in positions.iter().enumerate().skip(shared) {
            access.list_insert(
                score_director.working_solution_mut(),
                entity,
                position,
                self.members[member]
                    .take()
                    .expect("a member outside the list is held"),
            );
            self.positions.push(position);
        }
        score_director.after_variable_changed(descriptor_index, entity);
        self.entity = Some(entity);
        score_director.calculate_score()
    }

    /// Takes every inserted member out of the list.
    fn take_out<S, A, D>(&mut self, access: &A, score_director: &mut D)
    where
        S: PlanningSolution,
        A: ListRuinAccess<S, Element = E>,
        D: Director<S>,
    {
        let Some(entity) = self.entity.take() else {
            return;
        };
        if self.positions.is_empty() {
            return;
        }
        let descriptor_index = access.descriptor_index();
        score_director.before_variable_changed(descriptor_index, entity);
        for member in (0..self.positions.len()).rev() {
            self.members[member] = Some(access.list_remove(
                score_director.working_solution_mut(),
                entity,
                self.positions[member],
            ));
        }
        score_director.after_variable_changed(descriptor_index, entity);
        self.positions.clear();
    }

    /// Takes every member out of the list and hands them back in order.
    fn into_members<S, A, D>(mut self, access: &A, score_director: &mut D) -> SmallVec<[E; 4]>
    where
        S: PlanningSolution,
        A: ListRuinAccess<S, Element = E>,
        D: Director<S>,
    {
        self.take_out(access, score_director);
        self.members
            .into_iter()
            .map(|member| member.expect("every member is out of the list"))
            .collect()
    }
}

/// Takes back the placements already made, then restores every removed
/// element to its original position.
fn restore_unplaced<S, A, D>(
    access: &A,
    plan: &GroupRuinPlan,
    placements: &RuinUndo,
    values: &mut SmallVec<[Option<A::Element>; 8]>,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    take_back_placements(access, placements, values, score_director);
    let restored = values
        .iter_mut()
        .enumerate()
        .map(|(index, value)| {
            let (entity, position) = plan.removed[index];
            (
                entity,
                position,
                value.take().expect("every removed element is held"),
            )
        })
        .collect();
    restore_values(access, restored, score_director);
}

fn take_back_placements<S, A, D>(
    access: &A,
    placements: &RuinUndo,
    values: &mut SmallVec<[Option<A::Element>; 8]>,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    let mut current_positions = final_positions_after_ordered_insertions(placements);
    for i in (0..placements.len()).rev() {
        let (entity, _, removed_index) = placements[i];
        let actual_position = current_positions[i];
        score_director.before_variable_changed(descriptor_index, entity);
        values[removed_index] = Some(access.list_remove(
            score_director.working_solution_mut(),
            entity,
            actual_position,
        ));
        score_director.after_variable_changed(descriptor_index, entity);
        for j in 0..i {
            let (other_entity, _, _) = placements[j];
            if other_entity == entity && current_positions[j] > actual_position {
                current_positions[j] -= 1;
            }
        }
    }
}

pub(crate) fn group_ruin_undo_move<S, A, D>(
    access: &A,
    plan: &GroupRuinPlan,
    placements: RuinUndo,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    let mut values: SmallVec<[Option<A::Element>; 8]> =
        (0..plan.removed.len()).map(|_| None).collect();
    restore_unplaced(access, plan, &placements, &mut values, score_director);
}

pub(crate) fn group_ruin_tabu_signature<S, A, D>(
    access: &A,
    plan: &GroupRuinPlan,
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListMoveAccess<S>,
    D: Director<S>,
{
    let solution = score_director.working_solution();
    let value_ids: SmallVec<[u64; 8]> = plan
        .removed
        .iter()
        .map(|&(entity, position)| {
            access.tabu_value_id(
                solution,
                access.list_get(solution, entity, position).as_ref(),
            )
        })
        .collect();
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let mut move_id: SmallVec<[u64; 8]> = smallvec![
        encode_usize(access.descriptor_index()),
        hash_str(access.variable_name()),
        encode_usize(plan.removed.len())
    ];
    for &(entity, position) in &plan.removed {
        move_id.push(encode_usize(entity));
        move_id.push(encode_usize(position));
    }
    move_id.extend(value_ids.iter().copied());
    MoveTabuSignature::from_self_undoing_identity(scope, move_id)
        .with_entity_tokens(
            plan.entities()
                .into_iter()
                .map(|entity| scope.entity_token(encode_usize(entity))),
        )
        .with_destination_value_tokens(value_ids.iter().map(|&id| scope.value_token(id)))
}

pub(crate) fn group_ruin_candidate_trace_identity<S, A>(
    access: &A,
    plan: &GroupRuinPlan,
) -> CandidateTraceIdentity
where
    A: ListMoveAccess<S>,
{
    CandidateTraceIdentity::logical_move(
        access.descriptor_index(),
        access.variable_name(),
        "list_group_ruin",
        plan.removed
            .iter()
            .flat_map(|&(entity, position)| [entity, position]),
    )
}
