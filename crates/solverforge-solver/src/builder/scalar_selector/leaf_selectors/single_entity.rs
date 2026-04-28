#[derive(Clone, Copy)]
pub struct SwapLeafSelector<S> {
    ctx: ScalarVariableContext<S>,
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

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for SwapLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, ScalarMoveUnion<S, usize>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let entity_count = (self.ctx.entity_count)(solution);
        let current_values: Vec<_> = (0..entity_count)
            .map(|entity_index| self.ctx.current_value(solution, entity_index))
            .collect();
        let legal_values: Vec<_> = (0..entity_count)
            .map(|entity_index| self.ctx.values_for_entity(solution, entity_index))
            .collect();
        let mut moves = Vec::new();

        for left_entity_index in 0..entity_count {
            let left_value = current_values[left_entity_index];
            for right_entity_index in (left_entity_index + 1)..entity_count {
                let right_value = current_values[right_entity_index];
                if left_value == right_value {
                    continue;
                }
                if !scalar_swap_is_legal(self.ctx, &legal_values[left_entity_index], right_value)
                    || !scalar_swap_is_legal(
                        self.ctx,
                        &legal_values[right_entity_index],
                        left_value,
                    )
                {
                    continue;
                }
                moves.push(ScalarMoveUnion::Swap(
                    crate::heuristic::r#move::SwapMove::new(
                        left_entity_index,
                        right_entity_index,
                        self.ctx.getter,
                        self.ctx.setter,
                        self.ctx.variable_index,
                        self.ctx.variable_name,
                        self.ctx.descriptor_index,
                    ),
                ));
            }
        }

        ArenaMoveCursor::from_moves(moves)
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
    ctx: ScalarVariableContext<S>,
    max_nearby: usize,
    value_candidate_limit: Option<usize>,
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
        = ArenaMoveCursor<S, ScalarMoveUnion<S, usize>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let distance_meter = self
            .ctx
            .nearby_value_distance_meter;
        let candidate_values = self
            .ctx
            .nearby_value_candidates
            .expect("nearby change requires nearby_value_candidates");
        let mut moves = Vec::new();

        for entity_index in 0..(self.ctx.entity_count)(solution) {
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

            moves.extend(candidates.into_iter().map(|(value, _, _)| {
                ScalarMoveUnion::Change(crate::heuristic::r#move::ChangeMove::new(
                    entity_index,
                    Some(value),
                    self.ctx.getter,
                    self.ctx.setter,
                    self.ctx.variable_index,
                    self.ctx.variable_name,
                    self.ctx.descriptor_index,
                ))
            }));

            if self.ctx.allows_unassigned && current_assigned {
                moves.push(ScalarMoveUnion::Change(
                    crate::heuristic::r#move::ChangeMove::new(
                        entity_index,
                        None,
                        self.ctx.getter,
                        self.ctx.setter,
                        self.ctx.variable_index,
                        self.ctx.variable_name,
                        self.ctx.descriptor_index,
                    ),
                ));
            }
        }

        ArenaMoveCursor::from_moves(moves)
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
    ctx: ScalarVariableContext<S>,
    max_nearby: usize,
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
        = ArenaMoveCursor<S, ScalarMoveUnion<S, usize>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
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
        let mut moves = Vec::new();

        for left_entity_index in 0..entity_count {
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

            moves.extend(candidates.into_iter().map(|(right_entity_index, _, _)| {
                ScalarMoveUnion::Swap(crate::heuristic::r#move::SwapMove::new(
                    left_entity_index,
                    right_entity_index,
                    self.ctx.getter,
                    self.ctx.setter,
                    self.ctx.variable_index,
                    self.ctx.variable_name,
                    self.ctx.descriptor_index,
                ))
            }));
        }

        ArenaMoveCursor::from_moves(moves)
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
