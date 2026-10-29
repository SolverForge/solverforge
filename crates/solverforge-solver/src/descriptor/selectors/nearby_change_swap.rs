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
        let solution = score_director.working_solution();
        let mut cursor = self.open_cursor(score_director);
        let mut count = 0;
        while cursor.next_candidate(solution).is_some() {
            count += 1;
        }
        count
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
        let solution = score_director.working_solution();
        let mut cursor = self.open_cursor(score_director);
        let mut count = 0;
        while cursor.next_candidate(solution).is_some() {
            count += 1;
        }
        count
    }
}
