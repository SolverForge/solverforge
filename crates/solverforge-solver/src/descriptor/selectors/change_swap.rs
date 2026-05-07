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
        = ArenaMoveCursor<S, DescriptorMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        let allows_unassigned = self.allows_unassigned;
        let solution = score_director.working_solution() as &dyn Any;
        let moves: Vec<_> = (0..count)
            .flat_map(move |entity_index| {
                let entity = descriptor
                    .get_entity(solution, binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for change selector");
                let current_value = (binding.getter)(entity);
                let unassign_move = (allows_unassigned && current_value.is_some()).then({
                    let binding = binding.clone();
                    let descriptor = descriptor.clone();
                    move || {
                        DescriptorMoveUnion::Change(DescriptorChangeMove::new(
                            binding.clone(),
                            entity_index,
                            None,
                            descriptor.clone(),
                        ))
                    }
                });
                binding
                    .candidate_values_for_entity_index(
                        &descriptor,
                        solution,
                        entity_index,
                        self.value_candidate_limit,
                    )
                    .into_iter()
                    .map({
                        let binding = binding.clone();
                        let descriptor = descriptor.clone();
                        move |value| {
                            DescriptorMoveUnion::Change(DescriptorChangeMove::new(
                                binding.clone(),
                                entity_index,
                                Some(value),
                                descriptor.clone(),
                            ))
                        }
                    })
                    .chain(unassign_move)
            })
            .collect();
        ArenaMoveCursor::from_moves(moves)
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
        = ArenaMoveCursor<S, DescriptorMoveUnion<S>>
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

        let mut moves = Vec::new();
        for left_entity_index in 0..count {
            for right_entity_index in (left_entity_index + 1)..count {
                if let Some((left_value, right_value)) =
                    legality_index.values_for_swap(left_entity_index, right_entity_index)
                {
                    moves.push(DescriptorMoveUnion::Swap(
                        DescriptorSwapMove::new_validated(
                            binding.clone(),
                            left_entity_index,
                            left_value,
                            right_entity_index,
                            right_value,
                            descriptor.clone(),
                        ),
                    ));
                }
            }
        }
        ArenaMoveCursor::from_moves(moves)
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
        = ArenaMoveCursor<S, DescriptorMoveUnion<S>>
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
        let binding = self.binding.clone();
        let descriptor = self.solution_descriptor.clone();
        let max_nearby = self.max_nearby;
        let value_candidate_limit = self.value_candidate_limit;
        let moves: Vec<_> = (0..count)
            .flat_map(move |entity_index| {
                let entity = descriptor
                    .get_entity(solution, binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for nearby change selector");
                let current_value = (binding.getter)(entity);
                let current_assigned = current_value.is_some();
                let values = candidate_values(
                    solution,
                    entity_index,
                    binding.variable_index,
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

                let candidate_moves = candidates.into_iter().map({
                    let binding = binding.clone();
                    let descriptor = descriptor.clone();
                    move |(value, _, _)| {
                        DescriptorMoveUnion::Change(DescriptorChangeMove::new(
                            binding.clone(),
                            entity_index,
                            Some(value),
                            descriptor.clone(),
                        ))
                    }
                });
                let unassign = (binding.allows_unassigned && current_assigned).then({
                    let binding = binding.clone();
                    let descriptor = descriptor.clone();
                    move || {
                        DescriptorMoveUnion::Change(DescriptorChangeMove::new(
                            binding.clone(),
                            entity_index,
                            None,
                            descriptor.clone(),
                        ))
                    }
                });
                candidate_moves.chain(unassign)
            })
            .collect();
        ArenaMoveCursor::from_moves(moves)
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
        = ArenaMoveCursor<S, DescriptorMoveUnion<S>>
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
        let mut moves = Vec::new();
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
                moves.push(DescriptorMoveUnion::Swap(
                    DescriptorSwapMove::new_validated(
                        binding.clone(),
                        left_entity_index,
                        left_value,
                        right_entity_index,
                        right_value,
                        descriptor.clone(),
                    ),
                ));
            }
        }
        ArenaMoveCursor::from_moves(moves)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }
}
