//! Streamed group change and group swap cursors.
//!
//! Both cursors read one frozen per-step snapshot of the assigned groups:
//! each group's member coordinates in declared order and the owners every
//! member allows. They choose coordinates only; emitters build the moves.

use std::fmt::Debug;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::list_kernel::{
    GroupChangeCoordinates, GroupSwapCoordinates, MemberCoordinates,
};
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::list_support::ordered_index;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::list_groups::JointPositions;
use crate::list_placement::OwnerRestriction;

const GROUP_SALT: u64 = 0x6A0F_C4A4_6E00_0001;
const DESTINATION_SALT: u64 = 0x6A0F_C4A4_6E00_0002;
const PLACEMENT_SALT: u64 = 0x6A0F_C4A4_6E00_0003;
const PARTNER_SALT: u64 = 0x6A0F_C4A4_6E00_0004;
const MIX: u64 = 0x9E37_79B9_7F4A_7C15;

/// Emits one group change for already-approved coordinates.
pub(crate) trait GroupChangeEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_group_change(&self, coordinates: GroupChangeCoordinates) -> Self::Move;
}

/// Emits one group swap for already-approved coordinates.
pub(crate) trait GroupSwapEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_group_swap(&self, coordinates: GroupSwapCoordinates) -> Self::Move;
}

/// One fully assigned group as the group cursors see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GroupSnapshot {
    /// `(entity, position)` of every member, in declared order.
    pub(crate) members: MemberCoordinates,
    /// The owners every member allows.
    pub(crate) restriction: OwnerRestriction,
}

impl GroupSnapshot {
    /// The one owner holding every member, if there is one.
    fn owner(&self) -> Option<usize> {
        let (owner, _) = *self.members.first()?;
        self.members
            .iter()
            .all(|&(entity, _)| entity == owner)
            .then_some(owner)
    }

    fn positions(&self) -> JointPositions {
        self.members.iter().map(|&(_, position)| position).collect()
    }
}

/// Joint placements of `member_count` members into a list of `list_len`
/// other elements, chosen level by level: the first member's position, then
/// each gap to the next member. Canonical order is lexicographic; random
/// orders permute every level, keyed by the choices before it.
#[derive(Debug)]
struct PlacementStream {
    member_count: usize,
    list_len: usize,
    max_gap: usize,
    salt: u64,
    offsets: SmallVec<[usize; 4]>,
    values: SmallVec<[usize; 4]>,
    started: bool,
}

impl PlacementStream {
    fn new(member_count: usize, list_len: usize, max_gap: Option<usize>, salt: u64) -> Self {
        Self {
            member_count,
            list_len,
            max_gap: max_gap.unwrap_or(usize::MAX),
            salt,
            offsets: SmallVec::from_elem(0, member_count),
            values: SmallVec::from_elem(0, member_count),
            started: false,
        }
    }

    fn range(&self, level: usize) -> usize {
        if level == 0 {
            return self.list_len + 1;
        }
        let used: usize = self.values[..level].iter().sum();
        self.max_gap.min(self.list_len - used) + 1
    }

    fn value_at(&self, level: usize, offset: usize, context: MoveStreamContext) -> usize {
        let prefix = self.values[..level]
            .iter()
            .fold(self.salt ^ level as u64, |salt, &value| {
                (salt ^ value as u64).wrapping_mul(MIX)
            });
        ordered_index(offset, self.range(level), context, prefix)
    }

    fn fill_from(&mut self, level: usize, context: MoveStreamContext) {
        for deeper in level..self.member_count {
            self.offsets[deeper] = 0;
            self.values[deeper] = self.value_at(deeper, 0, context);
        }
    }

    fn positions(&self) -> JointPositions {
        let mut positions = JointPositions::with_capacity(self.member_count);
        let mut position = self.values[0];
        positions.push(position);
        for &gap in &self.values[1..] {
            position += gap + 1;
            positions.push(position);
        }
        positions
    }

    fn next(&mut self, context: MoveStreamContext) -> Option<JointPositions> {
        if self.member_count == 0 {
            return None;
        }
        if !self.started {
            self.started = true;
            self.fill_from(0, context);
            return Some(self.positions());
        }
        let mut level = self.member_count;
        while level > 0 {
            level -= 1;
            if self.offsets[level] + 1 < self.range(level) {
                self.offsets[level] += 1;
                self.values[level] = self.value_at(level, self.offsets[level], context);
                self.fill_from(level + 1, context);
                return Some(self.positions());
            }
        }
        None
    }
}

/// Streams every bounded joint relocation of every assigned group.
pub(crate) struct GroupChangeCursor<S, E>
where
    S: PlanningSolution,
    E: GroupChangeEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    groups: Vec<GroupSnapshot>,
    entity_lens: Vec<usize>,
    context: MoveStreamContext,
    max_member_gap: Option<usize>,
    salt: u64,
    group_offset: usize,
    destination_offset: usize,
    current: Option<(usize, usize, PlacementStream)>,
}

impl<S, E> GroupChangeCursor<S, E>
where
    S: PlanningSolution,
    E: GroupChangeEmitter<S>,
{
    pub(crate) fn new(
        emitter: E,
        groups: Vec<GroupSnapshot>,
        entity_lens: Vec<usize>,
        context: MoveStreamContext,
        max_member_gap: Option<usize>,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            emitter,
            groups,
            entity_lens,
            context,
            max_member_gap,
            salt: GROUP_SALT ^ descriptor_index as u64,
            group_offset: 0,
            destination_offset: 0,
            current: None,
        }
    }

    fn next_move(&mut self) -> Option<E::Move> {
        loop {
            if let Some((group, destination, stream)) = &mut self.current {
                let snapshot = &self.groups[*group];
                while let Some(positions) = stream.next(self.context) {
                    let identity = snapshot.members.iter().zip(&positions).all(
                        |(&(entity, position), &target)| {
                            entity == *destination && position == target
                        },
                    );
                    if identity {
                        continue;
                    }
                    return Some(self.emitter.emit_group_change(GroupChangeCoordinates {
                        sources: snapshot.members.clone(),
                        destination_entity: *destination,
                        destination_positions: positions,
                    }));
                }
                self.current = None;
                self.destination_offset += 1;
            }
            if self.group_offset >= self.groups.len() {
                return None;
            }
            if self.destination_offset >= self.entity_lens.len() {
                self.group_offset += 1;
                self.destination_offset = 0;
                continue;
            }
            let group = ordered_index(
                self.group_offset,
                self.groups.len(),
                self.context,
                self.salt,
            );
            let destination = ordered_index(
                self.destination_offset,
                self.entity_lens.len(),
                self.context,
                (self.salt ^ DESTINATION_SALT ^ group as u64).wrapping_mul(MIX),
            );
            let snapshot = &self.groups[group];
            if !snapshot.restriction.allows(destination) {
                self.destination_offset += 1;
                continue;
            }
            let staying = snapshot
                .members
                .iter()
                .filter(|&&(entity, _)| entity == destination)
                .count();
            let stream = PlacementStream::new(
                snapshot.members.len(),
                self.entity_lens[destination] - staying,
                self.max_member_gap,
                (self.salt ^ PLACEMENT_SALT ^ group as u64)
                    .wrapping_mul(MIX)
                    .wrapping_add(destination as u64),
            );
            self.current = Some((group, destination, stream));
        }
    }
}

/// Streams every exchange of two equally sized groups on different owners.
pub(crate) struct GroupSwapCursor<S, E>
where
    S: PlanningSolution,
    E: GroupSwapEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    groups: Vec<GroupSnapshot>,
    owners: Vec<Option<usize>>,
    context: MoveStreamContext,
    salt: u64,
    first_offset: usize,
    second_offset: usize,
}

impl<S, E> GroupSwapCursor<S, E>
where
    S: PlanningSolution,
    E: GroupSwapEmitter<S>,
{
    pub(crate) fn new(
        emitter: E,
        groups: Vec<GroupSnapshot>,
        context: MoveStreamContext,
        descriptor_index: usize,
    ) -> Self {
        let owners = groups.iter().map(GroupSnapshot::owner).collect();
        Self {
            store: CandidateStore::new(),
            emitter,
            groups,
            owners,
            context,
            salt: GROUP_SALT ^ PARTNER_SALT ^ descriptor_index as u64,
            first_offset: 0,
            second_offset: 0,
        }
    }

    fn next_move(&mut self) -> Option<E::Move> {
        let count = self.groups.len();
        while self.first_offset < count {
            let first = ordered_index(self.first_offset, count, self.context, self.salt);
            while self.second_offset < count {
                let second = ordered_index(
                    self.second_offset,
                    count,
                    self.context,
                    (self.salt ^ first as u64).wrapping_mul(MIX),
                );
                self.second_offset += 1;
                // Each unordered pair once, from its lower group index.
                if second <= first {
                    continue;
                }
                let (Some(first_owner), Some(second_owner)) =
                    (self.owners[first], self.owners[second])
                else {
                    continue;
                };
                let (first_group, second_group) = (&self.groups[first], &self.groups[second]);
                if first_owner == second_owner
                    || first_group.members.len() != second_group.members.len()
                    || !first_group.restriction.allows(second_owner)
                    || !second_group.restriction.allows(first_owner)
                {
                    continue;
                }
                return Some(self.emitter.emit_group_swap(GroupSwapCoordinates {
                    first_entity: first_owner,
                    first_positions: first_group.positions(),
                    second_entity: second_owner,
                    second_positions: second_group.positions(),
                }));
            }
            self.first_offset += 1;
            self.second_offset = 0;
        }
        None
    }
}

macro_rules! group_move_cursor {
    ($cursor:ident, $emitter:ident) => {
        impl<S, E> MoveCursor<S, E::Move> for $cursor<S, E>
        where
            S: PlanningSolution,
            E: $emitter<S>,
        {
            fn next_candidate(&mut self, _solution: &S) -> Option<CandidateId> {
                self.next_move().map(|mov| self.store.push(mov))
            }

            fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, E::Move>> {
                self.store.candidate(id)
            }

            fn take_candidate(&mut self, id: CandidateId) -> E::Move {
                self.store.take_candidate(id)
            }

            fn next_owned_candidate(&mut self, _solution: &S) -> Option<E::Move> {
                self.next_move()
            }

            fn release_candidate(&mut self, id: CandidateId) -> bool {
                self.store.release_candidate(id)
            }
        }
    };
}

group_move_cursor!(GroupChangeCursor, GroupChangeEmitter);
group_move_cursor!(GroupSwapCursor, GroupSwapEmitter);
