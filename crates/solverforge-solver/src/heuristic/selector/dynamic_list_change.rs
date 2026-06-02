use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::{DynamicListVariableSlot, PlanningSolution};
use solverforge_scoring::Director;

use crate::heuristic::r#move::{DynamicListChangeMove, MoveArena};

use super::list_support::ordered_index;
use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

pub struct DynamicListChangeMoveSelector<S> {
    slot: DynamicListVariableSlot<S>,
}

enum DynamicListChangeStage {
    Intra,
    Inter,
}

pub struct DynamicListChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    store: CandidateStore<S, DynamicListChangeMove<S>>,
    slot: DynamicListVariableSlot<S>,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    src_idx: usize,
    src_pos_offset: usize,
    stage: DynamicListChangeStage,
    intra_dst_offset: usize,
    dst_idx: usize,
    inter_dst_pos_offset: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> DynamicListChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    fn new(
        slot: DynamicListVariableSlot<S>,
        mut entities: Vec<usize>,
        mut route_lens: Vec<usize>,
        context: MoveStreamContext,
    ) -> Self {
        let start = context.start_offset(
            entities.len(),
            0xD158_C4A4_6E00_0001 ^ slot.descriptor_index() as u64,
        );
        entities.rotate_left(start);
        route_lens.rotate_left(start);
        Self {
            store: CandidateStore::new(),
            slot,
            entities,
            route_lens,
            context,
            src_idx: 0,
            src_pos_offset: 0,
            stage: DynamicListChangeStage::Intra,
            intra_dst_offset: 0,
            dst_idx: 0,
            inter_dst_pos_offset: 0,
            _phantom: PhantomData,
        }
    }

    fn current_source(&self) -> Option<(usize, usize, usize)> {
        let src_entity = *self.entities.get(self.src_idx)?;
        let src_len = self.route_lens[self.src_idx];
        if src_len == 0 {
            return Some((src_entity, src_len, 0));
        }
        let src_pos = ordered_index(
            self.src_pos_offset,
            src_len,
            self.context,
            0xD158_C4A4_6E00_0002 ^ src_entity as u64 ^ self.slot.descriptor_index() as u64,
        );
        Some((src_entity, src_len, src_pos))
    }

    fn advance_source_position(&mut self) {
        self.src_pos_offset += 1;
        self.stage = DynamicListChangeStage::Intra;
        self.intra_dst_offset = 0;
        self.dst_idx = 0;
        self.inter_dst_pos_offset = 0;

        while self.src_idx < self.route_lens.len()
            && self.src_pos_offset >= self.route_lens[self.src_idx]
        {
            self.src_idx += 1;
            self.src_pos_offset = 0;
        }
    }

    fn push_move(
        &mut self,
        src_entity: usize,
        src_pos: usize,
        dst_entity: usize,
        dst_pos: usize,
    ) -> CandidateId {
        self.store.push(DynamicListChangeMove::new(
            self.slot.clone(),
            src_entity,
            src_pos,
            dst_entity,
            dst_pos,
        ))
    }
}

impl<S> MoveCursor<S, DynamicListChangeMove<S>> for DynamicListChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            let (src_entity, src_len, src_pos) = self.current_source()?;
            if src_len == 0 {
                self.src_idx += 1;
                continue;
            }

            match self.stage {
                DynamicListChangeStage::Intra => {
                    while self.intra_dst_offset <= src_len {
                        let dst_pos = ordered_index(
                            self.intra_dst_offset,
                            src_len + 1,
                            self.context,
                            0xD158_C4A4_6E00_0003 ^ src_entity as u64 ^ src_pos as u64,
                        );
                        self.intra_dst_offset += 1;
                        if src_pos == dst_pos || dst_pos == src_pos + 1 {
                            continue;
                        }
                        return Some(self.push_move(src_entity, src_pos, src_entity, dst_pos));
                    }
                    self.stage = DynamicListChangeStage::Inter;
                    self.dst_idx = 0;
                    self.inter_dst_pos_offset = 0;
                }
                DynamicListChangeStage::Inter => {
                    while self.dst_idx < self.entities.len() {
                        if self.dst_idx == self.src_idx {
                            self.dst_idx += 1;
                            self.inter_dst_pos_offset = 0;
                            continue;
                        }
                        let dst_entity = self.entities[self.dst_idx];
                        let dst_len = self.route_lens[self.dst_idx];
                        if self.inter_dst_pos_offset <= dst_len {
                            let dst_pos = ordered_index(
                                self.inter_dst_pos_offset,
                                dst_len + 1,
                                self.context,
                                0xD158_C4A4_6E00_0004
                                    ^ src_entity as u64
                                    ^ dst_entity as u64
                                    ^ src_pos as u64,
                            );
                            self.inter_dst_pos_offset += 1;
                            return Some(self.push_move(src_entity, src_pos, dst_entity, dst_pos));
                        }
                        self.dst_idx += 1;
                        self.inter_dst_pos_offset = 0;
                    }
                    self.advance_source_position();
                }
            }
        }
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, DynamicListChangeMove<S>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> DynamicListChangeMove<S> {
        self.store.take_candidate(id)
    }
}

impl<S> DynamicListChangeMoveSelector<S> {
    pub fn new(slot: DynamicListVariableSlot<S>) -> Self {
        Self { slot }
    }
}

impl<S> Debug for DynamicListChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicListChangeMoveSelector")
            .field("slot", &self.slot)
            .finish()
    }
}

impl<S> MoveSelector<S, DynamicListChangeMove<S>> for DynamicListChangeMoveSelector<S>
where
    S: PlanningSolution,
{
    type Cursor<'a>
        = DynamicListChangeMoveCursor<S>
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
        let entities = (0..self.slot.entity_count(solution)).collect::<Vec<_>>();
        let route_lens = entities
            .iter()
            .map(|&entity| self.slot.list_len(solution, entity))
            .collect::<Vec<_>>();
        DynamicListChangeMoveCursor::new(self.slot.clone(), entities, route_lens, context)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let solution = score_director.working_solution();
        let route_lens = (0..self.slot.entity_count(solution))
            .map(|entity| self.slot.list_len(solution, entity))
            .collect::<Vec<_>>();
        let entity_count = route_lens.len();
        let total_elements: usize = route_lens.iter().sum();
        route_lens
            .iter()
            .map(|&source_len| {
                let intra_moves = source_len * source_len.saturating_sub(1);
                let inter_destinations =
                    total_elements.saturating_sub(source_len) + entity_count.saturating_sub(1);
                intra_moves + source_len * inter_destinations
            })
            .sum()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<DynamicListChangeMove<S>>,
    ) {
        let mut cursor = self.open_cursor(score_director);
        while let Some(id) = cursor.next_candidate() {
            arena.push(cursor.take_candidate(id));
        }
    }
}
