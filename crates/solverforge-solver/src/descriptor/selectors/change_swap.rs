struct DescriptorEntityValues {
    entity_index: usize,
    values: Vec<usize>,
    current_assigned: bool,
}

pub struct DescriptorChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    store: CandidateStore<S, DescriptorMoveUnion<S>>,
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    entity_values: Vec<DescriptorEntityValues>,
    entity_offset: usize,
    value_offset: usize,
}

impl<S> DescriptorChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    fn new(
        binding: VariableBinding,
        solution_descriptor: SolutionDescriptor,
        entity_values: Vec<DescriptorEntityValues>,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            binding,
            solution_descriptor,
            entity_values,
            entity_offset: 0,
            value_offset: 0,
        }
    }
}

impl<S> MoveCursor<S, DescriptorMoveUnion<S>> for DescriptorChangeMoveCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        while let Some(entity_values) = self.entity_values.get(self.entity_offset) {
            if let Some(value) = entity_values.values.get(self.value_offset).copied() {
                self.value_offset += 1;
                return Some(self.store.push(DescriptorMoveUnion::Change(
                    DescriptorChangeMove::new(
                        self.binding.clone(),
                        entity_values.entity_index,
                        Some(value),
                        self.solution_descriptor.clone(),
                    ),
                )));
            }

            if self.binding.allows_unassigned
                && entity_values.current_assigned
                && self.value_offset == entity_values.values.len()
            {
                self.value_offset += 1;
                return Some(self.store.push(DescriptorMoveUnion::Change(
                    DescriptorChangeMove::new(
                        self.binding.clone(),
                        entity_values.entity_index,
                        None,
                        self.solution_descriptor.clone(),
                    ),
                )));
            }

            self.entity_offset += 1;
            self.value_offset = 0;
        }
        None
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, DescriptorMoveUnion<S>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> DescriptorMoveUnion<S> {
        self.store.take_candidate(id)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl<S> Iterator for DescriptorChangeMoveCursor<S>
where
    S: PlanningSolution + 'static,
{
    type Item = DescriptorMoveUnion<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

enum DescriptorSwapOrder {
    All {
        count: usize,
        left_entity_index: usize,
        right_entity_index: usize,
    },
    Explicit(std::vec::IntoIter<(usize, usize)>),
}

pub struct DescriptorSwapMoveCursor<S>
where
    S: PlanningSolution,
{
    store: CandidateStore<S, DescriptorMoveUnion<S>>,
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    legality_index: SwapLegalityIndex,
    order: DescriptorSwapOrder,
}

impl<S> DescriptorSwapMoveCursor<S>
where
    S: PlanningSolution,
{
    fn all(
        binding: VariableBinding,
        solution_descriptor: SolutionDescriptor,
        legality_index: SwapLegalityIndex,
        count: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            binding,
            solution_descriptor,
            legality_index,
            order: DescriptorSwapOrder::All {
                count,
                left_entity_index: 0,
                right_entity_index: 1,
            },
        }
    }

    fn explicit(
        binding: VariableBinding,
        solution_descriptor: SolutionDescriptor,
        legality_index: SwapLegalityIndex,
        pairs: Vec<(usize, usize)>,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            binding,
            solution_descriptor,
            legality_index,
            order: DescriptorSwapOrder::Explicit(pairs.into_iter()),
        }
    }

    fn next_pair(&mut self) -> Option<(usize, usize)> {
        match &mut self.order {
            DescriptorSwapOrder::All {
                count,
                left_entity_index,
                right_entity_index,
            } => {
                while *left_entity_index < *count {
                    if *right_entity_index < *count {
                        let pair = (*left_entity_index, *right_entity_index);
                        *right_entity_index += 1;
                        return Some(pair);
                    }
                    *left_entity_index += 1;
                    *right_entity_index = left_entity_index.saturating_add(1);
                }
                None
            }
            DescriptorSwapOrder::Explicit(pairs) => pairs.next(),
        }
    }
}

impl<S> MoveCursor<S, DescriptorMoveUnion<S>> for DescriptorSwapMoveCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        while let Some((left_entity_index, right_entity_index)) = self.next_pair() {
            let Some((left_value, right_value)) = self
                .legality_index
                .values_for_swap(left_entity_index, right_entity_index)
            else {
                continue;
            };
            return Some(self.store.push(DescriptorMoveUnion::Swap(
                DescriptorSwapMove::new_validated(
                    self.binding.clone(),
                    left_entity_index,
                    left_value,
                    right_entity_index,
                    right_value,
                    self.solution_descriptor.clone(),
                ),
            )));
        }
        None
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, DescriptorMoveUnion<S>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> DescriptorMoveUnion<S> {
        self.store.take_candidate(id)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl<S> Iterator for DescriptorSwapMoveCursor<S>
where
    S: PlanningSolution + 'static,
{
    type Item = DescriptorMoveUnion<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

#[derive(Clone)]
pub struct DescriptorChangeMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    allows_unassigned: bool,
    value_candidate_limit: Option<usize>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorChangeMoveSelector")
            .field("binding", &self.binding)
            .finish()
    }
}

impl<S> DescriptorChangeMoveSelector<S> {
    fn new(
        binding: VariableBinding,
        solution_descriptor: SolutionDescriptor,
        value_candidate_limit: Option<usize>,
    ) -> Self {
        let allows_unassigned = binding.allows_unassigned;
        Self {
            binding,
            solution_descriptor,
            allows_unassigned,
            value_candidate_limit,
            _phantom: PhantomData,
        }
    }
}

impl<S> MoveSelector<S, DescriptorMoveUnion<S>> for DescriptorChangeMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorChangeMoveCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let solution = score_director.working_solution() as &dyn Any;
        let entity_values = (0..count)
            .map(|entity_index| {
                let entity = self
                    .solution_descriptor
                    .get_entity(solution, self.binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for change selector");
                DescriptorEntityValues {
                    entity_index,
                    values: self.binding.candidate_values_for_entity_index(
                        &self.solution_descriptor,
                        solution,
                        entity_index,
                        self.value_candidate_limit,
                    ),
                    current_assigned: (self.binding.getter)(entity).is_some(),
                }
            })
            .collect();
        DescriptorChangeMoveCursor::new(
            self.binding.clone(),
            self.solution_descriptor.clone(),
            entity_values,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let mut total = 0;
        for entity_index in 0..count {
            let entity = self
                .solution_descriptor
                .get_entity(
                    score_director.working_solution() as &dyn Any,
                    self.binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for change selector");
            total += self
                .binding
                .candidate_values_for_entity_index(
                    &self.solution_descriptor,
                    score_director.working_solution() as &dyn Any,
                    entity_index,
                    self.value_candidate_limit,
                )
                .len()
                + usize::from(self.allows_unassigned && (self.binding.getter)(entity).is_some());
        }
        total
    }
}

#[derive(Clone)]
pub struct DescriptorSwapMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorSwapMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorSwapMoveSelector")
            .field("binding", &self.binding)
            .finish()
    }
}

impl<S> DescriptorSwapMoveSelector<S> {
    fn new(binding: VariableBinding, solution_descriptor: SolutionDescriptor) -> Self {
        Self {
            binding,
            solution_descriptor,
            _phantom: PhantomData,
        }
    }
}

impl<S> MoveSelector<S, DescriptorMoveUnion<S>> for DescriptorSwapMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorSwapMoveCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let binding = self.binding.clone();
        let descriptor = self.solution_descriptor.clone();
        let solution = score_director.working_solution() as &dyn Any;
        let legality_index = SwapLegalityIndex::new(
            &binding,
            &descriptor,
            solution,
            count,
            "entity lookup failed for swap selector",
        );

        DescriptorSwapMoveCursor::all(binding, descriptor, legality_index, count)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let solution = score_director.working_solution() as &dyn Any;
        let legality_index = SwapLegalityIndex::new(
            &self.binding,
            &self.solution_descriptor,
            solution,
            count,
            "entity lookup failed for swap selector",
        );
        legality_index.count_legal_pairs()
    }
}

#[derive(Clone)]
pub struct DescriptorNearbyChangeMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    max_nearby: usize,
    value_candidate_limit: Option<usize>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorNearbyChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorNearbyChangeMoveSelector")
            .field("binding", &self.binding)
            .field("max_nearby", &self.max_nearby)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .finish()
    }
}

impl<S> MoveSelector<S, DescriptorMoveUnion<S>> for DescriptorNearbyChangeMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorChangeMoveCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let distance_meter = self.binding.nearby_value_distance_meter;
        let candidate_values = self
            .binding
            .nearby_value_candidates
            .expect("nearby change requires nearby_value_candidates");
        let solution = score_director.working_solution() as &dyn Any;
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let max_nearby = self.max_nearby;
        let value_candidate_limit = self.value_candidate_limit;
        let entity_values = (0..count)
            .map(|entity_index| {
                let entity = self
                    .solution_descriptor
                    .get_entity(solution, self.binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for nearby change selector");
                let current_value = (self.binding.getter)(entity);
                let current_assigned = current_value.is_some();
                let values = candidate_values(
                    solution,
                    entity_index,
                    self.binding.variable_index,
                );
                let limit = value_candidate_limit.unwrap_or(values.len());
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
                            .map(|meter| meter(solution, entity_index, value))
                            .unwrap_or(order as f64);
                        distance.is_finite().then_some((value, distance, order))
                    })
                    .collect();
                truncate_nearby_candidates(&mut candidates, max_nearby);

                DescriptorEntityValues {
                    entity_index,
                    values: candidates.into_iter().map(|(value, _, _)| value).collect(),
                    current_assigned,
                }
            })
            .collect();
        DescriptorChangeMoveCursor::new(
            self.binding.clone(),
            self.solution_descriptor.clone(),
            entity_values,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }
}

#[derive(Clone)]
pub struct DescriptorNearbySwapMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    max_nearby: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorNearbySwapMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorNearbySwapMoveSelector")
            .field("binding", &self.binding)
            .field("max_nearby", &self.max_nearby)
            .finish()
    }
}

impl<S> MoveSelector<S, DescriptorMoveUnion<S>> for DescriptorNearbySwapMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorSwapMoveCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let distance_meter = self.binding.nearby_entity_distance_meter;
        let entity_candidates = self
            .binding
            .nearby_entity_candidates
            .expect("nearby swap requires nearby_entity_candidates");
        let solution = score_director.working_solution() as &dyn Any;
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let binding = self.binding.clone();
        let descriptor = self.solution_descriptor.clone();
        let max_nearby = self.max_nearby;
        let legality_index = SwapLegalityIndex::new(
            &binding,
            &descriptor,
            solution,
            count,
            "entity lookup failed for nearby swap selector",
        );
        let mut pairs = Vec::new();
        for left_entity_index in 0..count {
            let mut candidates: Vec<(usize, f64, usize)> = entity_candidates(
                solution,
                left_entity_index,
                binding.variable_index,
            )
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(order, right_entity_index)| {
                    if right_entity_index <= left_entity_index || right_entity_index >= count {
                        return None;
                    }
                    if !legality_index.can_swap(left_entity_index, right_entity_index) {
                        return None;
                    }
                    let distance = distance_meter
                        .map(|meter| meter(solution, left_entity_index, right_entity_index))
                        .unwrap_or(order as f64);
                    distance
                        .is_finite()
                        .then_some((right_entity_index, distance, order))
                })
                .collect();
            truncate_nearby_candidates(&mut candidates, max_nearby);
            for (right_entity_index, _, _) in candidates {
                let Some((left_value, right_value)) =
                    legality_index.values_for_swap(left_entity_index, right_entity_index)
                else {
                    continue;
                };
                let _ = (left_value, right_value);
                pairs.push((left_entity_index, right_entity_index));
            }
        }
        DescriptorSwapMoveCursor::explicit(binding, descriptor, legality_index, pairs)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }
}
