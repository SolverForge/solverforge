#[derive(Clone, Copy)]
pub struct SwapLeafSelector<S> {
    ctx: ScalarVariableSlot<S>,
}

impl<S> Debug for SwapLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SwapLeafSelector")
            .field("descriptor_index", &self.ctx.descriptor_index)
            .field("variable_index", &self.ctx.variable_index)
            .field("variable_name", &self.ctx.variable_name)
            .finish()
    }
}

pub struct SwapLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    ctx: ScalarVariableSlot<S>,
    store: CandidateStore<S, ScalarMoveUnion<S, usize>>,
    current_values: Vec<Option<usize>>,
    legal_values: Vec<Vec<usize>>,
    context: MoveStreamContext,
    left_offset: usize,
    right_offset: usize,
}

impl<S> SwapLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn new(
        ctx: ScalarVariableSlot<S>,
        current_values: Vec<Option<usize>>,
        legal_values: Vec<Vec<usize>>,
        context: MoveStreamContext,
    ) -> Self {
        Self {
            ctx,
            store: CandidateStore::new(),
            current_values,
            legal_values,
            context,
            left_offset: 0,
            right_offset: 0,
        }
    }

    fn ordered_entity(&self, offset: usize, salt: u64) -> usize {
        let len = self.current_values.len();
        if len <= 1 {
            return 0;
        }
        let start = self.context.start_offset(len, salt);
        let stride = self.context.stride(len, salt ^ 0xD1B5_4A32_D192_ED03);
        (start + offset * stride) % len
    }
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for SwapLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        let entity_count = self.current_values.len();
        while self.left_offset < entity_count {
            let left_entity_index = self.ordered_entity(
                self.left_offset,
                0x5A09_5CA1_AA00_0001
                    ^ ((self.ctx.descriptor_index as u64) << 32)
                    ^ self.ctx.variable_index as u64,
            );
            while self.right_offset < entity_count {
                let right_entity_index = self.ordered_entity(
                    self.right_offset,
                    0x5A09_5CA1_AA00_0002
                        ^ left_entity_index as u64
                        ^ self.ctx.variable_index as u64,
                );
                self.right_offset += 1;

                if left_entity_index >= right_entity_index {
                    continue;
                }
                let left_value = self.current_values[left_entity_index];
                let right_value = self.current_values[right_entity_index];
                if left_value == right_value {
                    continue;
                }
                if !scalar_swap_is_legal(self.ctx, &self.legal_values[left_entity_index], right_value)
                    || !scalar_swap_is_legal(
                        self.ctx,
                        &self.legal_values[right_entity_index],
                        left_value,
                    )
                {
                    continue;
                }
                return Some(self.store.push(ScalarMoveUnion::Swap(
                    crate::heuristic::r#move::SwapMove::new(
                        left_entity_index,
                        right_entity_index,
                        self.ctx.getter,
                        self.ctx.setter,
                        self.ctx.variable_index,
                        self.ctx.variable_name,
                        self.ctx.descriptor_index,
                    ),
                )));
            }
            self.left_offset += 1;
            self.right_offset = 0;
        }
        None
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ScalarMoveUnion<S, usize>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ScalarMoveUnion<S, usize> {
        self.store.take_candidate(id)
    }
}

impl<S> Iterator for SwapLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    type Item = ScalarMoveUnion<S, usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for SwapLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = SwapLeafCursor<S>
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
        let entity_count = (self.ctx.entity_count)(solution);
        let current_values: Vec<_> = (0..entity_count)
            .map(|entity_index| self.ctx.current_value(solution, entity_index))
            .collect();
        let legal_values: Vec<_> = (0..entity_count)
            .map(|entity_index| self.ctx.values_for_entity(solution, entity_index))
            .collect();
        SwapLeafCursor::new(self.ctx, current_values, legal_values, context)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}

#[derive(Clone, Copy)]
pub struct NearbyChangeLeafSelector<S> {
    ctx: ScalarVariableSlot<S>,
    max_nearby: usize,
    value_candidate_limit: Option<usize>,
}

struct NearbyChangeSource {
    entity_index: usize,
    values: Vec<Option<usize>>,
}

pub struct NearbyChangeLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    ctx: ScalarVariableSlot<S>,
    store: CandidateStore<S, ScalarMoveUnion<S, usize>>,
    sources: Vec<NearbyChangeSource>,
    source_offset: usize,
    value_offset: usize,
}

impl<S> NearbyChangeLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn new(ctx: ScalarVariableSlot<S>, sources: Vec<NearbyChangeSource>) -> Self {
        Self {
            ctx,
            store: CandidateStore::new(),
            sources,
            source_offset: 0,
            value_offset: 0,
        }
    }
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for NearbyChangeLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            let source = self.sources.get(self.source_offset)?;
            if self.value_offset >= source.values.len() {
                self.source_offset += 1;
                self.value_offset = 0;
                continue;
            }
            let value = source.values[self.value_offset];
            self.value_offset += 1;
            return Some(self.store.push(ScalarMoveUnion::Change(
                crate::heuristic::r#move::ChangeMove::new(
                    source.entity_index,
                    value,
                    self.ctx.getter,
                    self.ctx.setter,
                    self.ctx.variable_index,
                    self.ctx.variable_name,
                    self.ctx.descriptor_index,
                ),
            )));
        }
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ScalarMoveUnion<S, usize>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ScalarMoveUnion<S, usize> {
        self.store.take_candidate(id)
    }
}

impl<S> Iterator for NearbyChangeLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    type Item = ScalarMoveUnion<S, usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S> Debug for NearbyChangeLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NearbyChangeLeafSelector")
            .field("descriptor_index", &self.ctx.descriptor_index)
            .field("variable_name", &self.ctx.variable_name)
            .field("max_nearby", &self.max_nearby)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for NearbyChangeLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = NearbyChangeLeafCursor<S>
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
        let distance_meter = self
            .ctx
            .nearby_value_distance_meter;
        let candidate_values = self
            .ctx
            .nearby_value_candidates
            .expect("nearby change requires nearby_value_candidates");
        let entity_count = (self.ctx.entity_count)(solution);
        let mut sources = Vec::new();

        let entity_start = context.start_offset(
            entity_count,
            0xC4A4_6E00_AAAA_0001
                ^ ((self.ctx.descriptor_index as u64) << 32)
                ^ self.ctx.variable_index as u64,
        );
        let entity_stride = context.stride(
            entity_count,
            0xC4A4_6E00_AAAA_0002
                ^ ((self.ctx.descriptor_index as u64) << 32)
                ^ self.ctx.variable_index as u64,
        );
        for entity_offset in 0..entity_count {
            let entity_index = if entity_count <= 1 {
                0
            } else {
                (entity_start + entity_offset * entity_stride) % entity_count
            };
            let current_value = self.ctx.current_value(solution, entity_index);
            let current_assigned = current_value.is_some();
            let values = candidate_values(
                solution,
                entity_index,
                self.ctx.variable_index,
            );
            let limit = self.value_candidate_limit.unwrap_or(values.len());
            let mut candidates: Vec<(usize, f64, usize)> = values
                .iter()
                .copied()
                .take(limit)
                .enumerate()
                .filter_map(|(order, value)| {
                    if current_value == Some(value) {
                        return None;
                    }
                    let distance = distance_meter
                        .and_then(|meter| {
                            meter(solution, entity_index, self.ctx.variable_index, value)
                        })
                        .unwrap_or(order as f64);
                    distance.is_finite().then_some((value, distance, order))
                })
                .collect();

            truncate_nearby_candidates(&mut candidates, self.max_nearby);

            let mut source_values: Vec<Option<usize>> = candidates
                .into_iter()
                .map(|(value, _, _)| Some(value))
                .collect();

            if self.ctx.allows_unassigned && current_assigned {
                source_values.push(None);
            }
            sources.push(NearbyChangeSource {
                entity_index,
                values: source_values,
            });
        }

        NearbyChangeLeafCursor::new(self.ctx, sources)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}

#[derive(Clone, Copy)]
pub struct NearbySwapLeafSelector<S> {
    ctx: ScalarVariableSlot<S>,
    max_nearby: usize,
}

struct NearbySwapSource {
    left_entity_index: usize,
    right_entities: Vec<usize>,
}

pub struct NearbySwapLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    ctx: ScalarVariableSlot<S>,
    store: CandidateStore<S, ScalarMoveUnion<S, usize>>,
    sources: Vec<NearbySwapSource>,
    source_offset: usize,
    right_offset: usize,
}

impl<S> NearbySwapLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn new(ctx: ScalarVariableSlot<S>, sources: Vec<NearbySwapSource>) -> Self {
        Self {
            ctx,
            store: CandidateStore::new(),
            sources,
            source_offset: 0,
            right_offset: 0,
        }
    }
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for NearbySwapLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            let source = self.sources.get(self.source_offset)?;
            if self.right_offset >= source.right_entities.len() {
                self.source_offset += 1;
                self.right_offset = 0;
                continue;
            }
            let right_entity_index = source.right_entities[self.right_offset];
            self.right_offset += 1;
            return Some(self.store.push(ScalarMoveUnion::Swap(
                crate::heuristic::r#move::SwapMove::new(
                    source.left_entity_index,
                    right_entity_index,
                    self.ctx.getter,
                    self.ctx.setter,
                    self.ctx.variable_index,
                    self.ctx.variable_name,
                    self.ctx.descriptor_index,
                ),
            )));
        }
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ScalarMoveUnion<S, usize>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ScalarMoveUnion<S, usize> {
        self.store.take_candidate(id)
    }
}

impl<S> Iterator for NearbySwapLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    type Item = ScalarMoveUnion<S, usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S> Debug for NearbySwapLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NearbySwapLeafSelector")
            .field("descriptor_index", &self.ctx.descriptor_index)
            .field("variable_name", &self.ctx.variable_name)
            .field("max_nearby", &self.max_nearby)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for NearbySwapLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = NearbySwapLeafCursor<S>
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
        let distance_meter = self
            .ctx
            .nearby_entity_distance_meter;
        let entity_candidates = self
            .ctx
            .nearby_entity_candidates
            .expect("nearby swap requires nearby_entity_candidates");
        let entity_count = (self.ctx.entity_count)(solution);
        let current_values: Vec<_> = (0..entity_count)
            .map(|entity_index| self.ctx.current_value(solution, entity_index))
            .collect();
        let mut sources = Vec::new();

        let entity_start = context.start_offset(
            entity_count,
            0x5A09_5CA1_AAAA_0001
                ^ ((self.ctx.descriptor_index as u64) << 32)
                ^ self.ctx.variable_index as u64,
        );
        let entity_stride = context.stride(
            entity_count,
            0x5A09_5CA1_AAAA_0002
                ^ ((self.ctx.descriptor_index as u64) << 32)
                ^ self.ctx.variable_index as u64,
        );
        for left_offset in 0..entity_count {
            let left_entity_index = if entity_count <= 1 {
                0
            } else {
                (entity_start + left_offset * entity_stride) % entity_count
            };
            let left_value = current_values[left_entity_index];
            let mut candidates: Vec<(usize, f64, usize)> = entity_candidates(
                solution,
                left_entity_index,
                self.ctx.variable_index,
            )
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(order, right_entity_index)| {
                    if right_entity_index <= left_entity_index || right_entity_index >= entity_count
                    {
                        return None;
                    }
                    if left_value == current_values[right_entity_index] {
                        return None;
                    }
                    if !self.ctx.value_is_legal(
                        solution,
                        left_entity_index,
                        current_values[right_entity_index],
                    ) || !self.ctx.value_is_legal(solution, right_entity_index, left_value)
                    {
                        return None;
                    }
                    let distance = distance_meter
                        .and_then(|meter| {
                            meter(
                                solution,
                                left_entity_index,
                                right_entity_index,
                                self.ctx.variable_index,
                            )
                        })
                        .unwrap_or(order as f64);
                    distance
                        .is_finite()
                        .then_some((right_entity_index, distance, order))
                })
                .collect();

            truncate_nearby_candidates(&mut candidates, self.max_nearby);

            sources.push(NearbySwapSource {
                left_entity_index,
                right_entities: candidates
                    .into_iter()
                    .map(|(right_entity_index, _, _)| right_entity_index)
                    .collect(),
            });
        }

        NearbySwapLeafCursor::new(self.ctx, sources)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}
