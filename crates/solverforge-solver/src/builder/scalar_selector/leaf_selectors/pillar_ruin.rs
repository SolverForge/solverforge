#[derive(Clone, Copy)]
pub struct PillarChangeLeafSelector<S> {
    ctx: ScalarVariableSlot<S>,
    minimum_sub_pillar_size: usize,
    maximum_sub_pillar_size: usize,
    value_candidate_limit: Option<usize>,
}

impl<S> Debug for PillarChangeLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PillarChangeLeafSelector")
            .field("descriptor_index", &self.ctx.descriptor_index)
            .field("variable_name", &self.ctx.variable_name)
            .field("minimum_sub_pillar_size", &self.minimum_sub_pillar_size)
            .field("maximum_sub_pillar_size", &self.maximum_sub_pillar_size)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for PillarChangeLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, ScalarMoveUnion<S, usize>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let pillar_selector = DefaultPillarSelector::<S, usize, _, _>::new(
            FromSolutionEntitySelector::new(self.ctx.descriptor_index),
            self.ctx.descriptor_index,
            self.ctx.variable_name,
            |sd: &dyn Director<S>, _descriptor_index, entity_index| {
                self.ctx.current_value(sd.working_solution(), entity_index)
            },
        )
        .with_sub_pillar_config(build_sub_pillar_config(
            self.minimum_sub_pillar_size,
            self.maximum_sub_pillar_size,
        ));

        let value_selector = ScalarCandidateSelector::new(self.ctx, self.value_candidate_limit);
        let mut moves = Vec::new();
        for pillar in pillar_selector.iter(score_director) {
            let Some(first) = pillar.first() else {
                continue;
            };
            let Some(current_value) = self
                .ctx
                .current_value(score_director.working_solution(), first.entity_index)
            else {
                continue;
            };
            let entity_indices: Vec<usize> =
                pillar.iter().map(|entity| entity.entity_index).collect();
            let legal_values = intersect_legal_values_for_pillar(&pillar, |entity_index| {
                scalar_legal_values_for_entity(
                    &value_selector,
                    score_director,
                    self.ctx.descriptor_index,
                    entity_index,
                )
            });
            moves.extend(
                legal_values
                    .into_iter()
                    .filter(|&value| value != current_value)
                    .map(|value| {
                        ScalarMoveUnion::PillarChange(PillarChangeMove::new(
                            entity_indices.clone(),
                            Some(value),
                            self.ctx.getter,
                            self.ctx.setter,
                            self.ctx.variable_index,
                            self.ctx.variable_name,
                            self.ctx.descriptor_index,
                        ))
                    }),
            );
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
pub struct PillarSwapLeafSelector<S> {
    ctx: ScalarVariableSlot<S>,
    minimum_sub_pillar_size: usize,
    maximum_sub_pillar_size: usize,
}

impl<S> Debug for PillarSwapLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PillarSwapLeafSelector")
            .field("descriptor_index", &self.ctx.descriptor_index)
            .field("variable_name", &self.ctx.variable_name)
            .field("minimum_sub_pillar_size", &self.minimum_sub_pillar_size)
            .field("maximum_sub_pillar_size", &self.maximum_sub_pillar_size)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for PillarSwapLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, ScalarMoveUnion<S, usize>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let pillar_selector = DefaultPillarSelector::<S, usize, _, _>::new(
            FromSolutionEntitySelector::new(self.ctx.descriptor_index),
            self.ctx.descriptor_index,
            self.ctx.variable_name,
            |sd: &dyn Director<S>, _descriptor_index, entity_index| {
                self.ctx.current_value(sd.working_solution(), entity_index)
            },
        )
        .with_sub_pillar_config(build_sub_pillar_config(
            self.minimum_sub_pillar_size,
            self.maximum_sub_pillar_size,
        ));

        let value_selector = ScalarValueSelector::from_context(self.ctx);
        let pillars: Vec<_> = pillar_selector.iter(score_director).collect();
        let mut moves = Vec::new();
        for left_index in 0..pillars.len() {
            let Some(left_first) = pillars[left_index].first() else {
                continue;
            };
            let Some(left_value) = self
                .ctx
                .current_value(score_director.working_solution(), left_first.entity_index)
            else {
                continue;
            };
            let left_entities: Vec<usize> = pillars[left_index]
                .iter()
                .map(|entity| entity.entity_index)
                .collect();
            for right_pillar in pillars.iter().skip(left_index + 1) {
                let Some(right_first) = right_pillar.first() else {
                    continue;
                };
                let Some(right_value) = self
                    .ctx
                    .current_value(score_director.working_solution(), right_first.entity_index)
                else {
                    continue;
                };
                let left_group = PillarGroup::new(left_value, pillars[left_index].clone());
                let right_group = PillarGroup::new(right_value, right_pillar.clone());
                if !matches!(self.ctx.value_source, ValueSource::Empty)
                    && !pillars_are_swap_compatible(&left_group, &right_group, |entity_index| {
                        scalar_legal_values_for_entity(
                            &value_selector,
                            score_director,
                            self.ctx.descriptor_index,
                            entity_index,
                        )
                    })
                {
                    continue;
                }
                let right_entities: Vec<usize> = right_pillar
                    .iter()
                    .map(|entity| entity.entity_index)
                    .collect();
                moves.push(ScalarMoveUnion::PillarSwap(PillarSwapMove::new(
                    left_entities.clone(),
                    right_entities,
                    self.ctx.getter,
                    self.ctx.setter,
                    self.ctx.variable_index,
                    self.ctx.variable_name,
                    self.ctx.descriptor_index,
                )));
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

pub struct RuinRecreateLeafSelector<S> {
    selector: RuinMoveSelector<S, usize>,
    getter: fn(&S, usize, usize) -> Option<usize>,
    setter: fn(&mut S, usize, usize, Option<usize>),
    descriptor_index: usize,
    variable_index: usize,
    variable_name: &'static str,
    value_source: ScalarRecreateValueSource<S>,
    recreate_heuristic_type: RecreateHeuristicType,
    allows_unassigned: bool,
}

impl<S> Debug for RuinRecreateLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuinRecreateLeafSelector")
            .field("selector", &self.selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
            .field("variable_name", &self.variable_name)
            .field("recreate_heuristic_type", &self.recreate_heuristic_type)
            .field("allows_unassigned", &self.allows_unassigned)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for RuinRecreateLeafSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: solverforge_core::score::Score,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, ScalarMoveUnion<S, usize>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let value_source = self.value_source;
        let moves: Vec<_> = self
            .selector
            .iter_moves(score_director)
            .filter_map(move |ruin| {
                let mov = RuinRecreateMove::new(
                    ruin.entity_indices_slice(),
                    self.getter,
                    self.setter,
                    self.descriptor_index,
                    self.variable_index,
                    self.variable_name,
                    value_source,
                    self.recreate_heuristic_type,
                    self.allows_unassigned,
                );
                mov.is_doable(score_director)
                    .then_some(ScalarMoveUnion::RuinRecreate(mov))
            })
            .collect();
        ArenaMoveCursor::from_moves(moves)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.selector.size(score_director)
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}
