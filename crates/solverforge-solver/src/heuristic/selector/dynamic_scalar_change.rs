use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::{DynamicScalarVariableSlot, PlanningSolution};
use solverforge_scoring::Director;

use crate::heuristic::r#move::{DynamicScalarChangeMove, MoveArena};

use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

pub struct DynamicScalarChangeMoveSelector<S> {
    slot: DynamicScalarVariableSlot<S>,
    value_candidate_limit: Option<usize>,
}

struct DynamicScalarEntityValues {
    entity_index: usize,
    values: Vec<usize>,
    current_assigned: bool,
}

pub struct DynamicScalarChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    store: CandidateStore<S, DynamicScalarChangeMove<S>>,
    entity_values: Vec<DynamicScalarEntityValues>,
    entity_offset: usize,
    value_offset: usize,
    slot: DynamicScalarVariableSlot<S>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> DynamicScalarChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    fn new(
        slot: DynamicScalarVariableSlot<S>,
        entity_values: Vec<DynamicScalarEntityValues>,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            entity_values,
            entity_offset: 0,
            value_offset: 0,
            slot,
            _phantom: PhantomData,
        }
    }
}

impl<S> MoveCursor<S, DynamicScalarChangeMove<S>> for DynamicScalarChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        while self.entity_offset < self.entity_values.len() {
            let entity_values = &self.entity_values[self.entity_offset];
            if self.value_offset < entity_values.values.len() {
                let value = entity_values.values[self.value_offset];
                self.value_offset += 1;
                return Some(self.store.push(DynamicScalarChangeMove::new(
                    self.slot.clone(),
                    entity_values.entity_index,
                    Some(value),
                )));
            }

            let to_none_offset = entity_values.values.len();
            if self.slot.allows_unassigned
                && entity_values.current_assigned
                && self.value_offset == to_none_offset
            {
                self.value_offset += 1;
                return Some(self.store.push(DynamicScalarChangeMove::new(
                    self.slot.clone(),
                    entity_values.entity_index,
                    None,
                )));
            }

            self.entity_offset += 1;
            self.value_offset = 0;
        }

        None
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, DynamicScalarChangeMove<S>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> DynamicScalarChangeMove<S> {
        self.store.take_candidate(id)
    }
}

impl<S> Iterator for DynamicScalarChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    type Item = DynamicScalarChangeMove<S>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S> DynamicScalarChangeMoveSelector<S> {
    pub fn new(slot: DynamicScalarVariableSlot<S>, value_candidate_limit: Option<usize>) -> Self {
        Self {
            slot,
            value_candidate_limit,
        }
    }
}

impl<S> Debug for DynamicScalarChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicScalarChangeMoveSelector")
            .field("slot", &self.slot)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .finish()
    }
}

impl<S> MoveSelector<S, DynamicScalarChangeMove<S>> for DynamicScalarChangeMoveSelector<S>
where
    S: PlanningSolution,
{
    type Cursor<'a>
        = DynamicScalarChangeMoveCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let value_limit = self.value_candidate_limit.unwrap_or(usize::MAX);
        let mut entity_values = (0..self.slot.entity_count(solution))
            .map(|entity_index| {
                let current_assigned = self.slot.current_value(solution, entity_index).is_some();
                let mut values = self
                    .slot
                    .candidate_values(solution, entity_index)
                    .iter()
                    .copied()
                    .take(value_limit)
                    .collect::<Vec<_>>();
                let start = context.start_offset(
                    values.len(),
                    0xD94E_5CA1_0000_0000
                        ^ entity_index as u64
                        ^ ((self.slot.descriptor_index() as u64) << 32)
                        ^ self.slot.variable.0 as u64,
                );
                values.rotate_left(start);
                DynamicScalarEntityValues {
                    entity_index,
                    values,
                    current_assigned,
                }
            })
            .collect::<Vec<_>>();
        let entity_start = context.start_offset(
            entity_values.len(),
            0xD94E_5CA1_0000_0001
                ^ ((self.slot.descriptor_index() as u64) << 32)
                ^ self.slot.variable.0 as u64,
        );
        entity_values.rotate_left(entity_start);
        DynamicScalarChangeMoveCursor::new(self.slot.clone(), entity_values)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let solution = score_director.working_solution();
        let value_limit = self.value_candidate_limit.unwrap_or(usize::MAX);
        (0..self.slot.entity_count(solution))
            .map(|entity_index| {
                let candidate_count = self
                    .slot
                    .candidate_values(solution, entity_index)
                    .iter()
                    .take(value_limit)
                    .count();
                let unassigned_count = usize::from(
                    self.slot.allows_unassigned
                        && self.slot.current_value(solution, entity_index).is_some(),
                );
                candidate_count + unassigned_count
            })
            .sum()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<DynamicScalarChangeMove<S>>,
    ) {
        let mut cursor = self.open_cursor(score_director);
        while let Some(id) = cursor.next_candidate() {
            arena.push(cursor.take_candidate(id));
        }
    }
}
