fn build_sub_pillar_config(minimum_size: usize, maximum_size: usize) -> SubPillarConfig {
    if minimum_size == 0 || maximum_size == 0 {
        SubPillarConfig::none()
    } else {
        SubPillarConfig {
            enabled: true,
            minimum_size: minimum_size.max(2),
            maximum_size: maximum_size.max(minimum_size.max(2)),
        }
    }
}

#[derive(Clone)]
pub struct DescriptorPillarChangeMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    minimum_sub_pillar_size: usize,
    maximum_sub_pillar_size: usize,
    value_candidate_limit: Option<usize>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorPillarChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorPillarChangeMoveSelector")
            .field("binding", &self.binding)
            .field("minimum_sub_pillar_size", &self.minimum_sub_pillar_size)
            .field("maximum_sub_pillar_size", &self.maximum_sub_pillar_size)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .finish()
    }
}

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorPillarChangeMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let solution = score_director.working_solution() as &dyn Any;
        let binding = self.binding.clone();
        let descriptor = self.solution_descriptor.clone();
        let count = score_director
            .entity_count(binding.descriptor_index)
            .unwrap_or(0);
        let moves: Vec<_> = collect_pillar_groups(
            (0..count).map(|entity_index| {
                (
                    EntityReference::new(binding.descriptor_index, entity_index),
                    (binding.getter)(binding.entity_for_index(&descriptor, solution, entity_index)),
                )
            }),
            &build_sub_pillar_config(self.minimum_sub_pillar_size, self.maximum_sub_pillar_size),
        )
        .into_iter()
        .flat_map(move |group| {
            let entity_indices: Vec<usize> = group
                .pillar
                .iter()
                .map(|entity| entity.entity_index)
                .collect();
            intersect_legal_values_for_pillar(&group.pillar, |entity_index| {
                binding.candidate_values_for_entity_index(
                    &descriptor,
                    solution,
                    entity_index,
                    self.value_candidate_limit,
                )
            })
            .into_iter()
            .filter(move |&value| value != group.shared_value)
            .map({
                let binding = binding.clone();
                let descriptor = descriptor.clone();
                let entity_indices = entity_indices.clone();
                move |value| {
                    DescriptorScalarMoveUnion::PillarChange(DescriptorPillarChangeMove::new(
                        binding.clone(),
                        entity_indices.clone(),
                        Some(value),
                        descriptor.clone(),
                    ))
                }
            })
        })
        .collect();
        ArenaMoveCursor::from_moves(moves)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }
}

#[derive(Clone)]
pub struct DescriptorPillarSwapMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    minimum_sub_pillar_size: usize,
    maximum_sub_pillar_size: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorPillarSwapMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorPillarSwapMoveSelector")
            .field("binding", &self.binding)
            .field("minimum_sub_pillar_size", &self.minimum_sub_pillar_size)
            .field("maximum_sub_pillar_size", &self.maximum_sub_pillar_size)
            .finish()
    }
}

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorPillarSwapMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let binding = self.binding.clone();
        let descriptor = self.solution_descriptor.clone();
        let solution = score_director.working_solution() as &dyn Any;
        let count = score_director
            .entity_count(binding.descriptor_index)
            .unwrap_or(0);
        let pillars = collect_pillar_groups(
            (0..count).map(|entity_index| {
                (
                    EntityReference::new(binding.descriptor_index, entity_index),
                    (binding.getter)(binding.entity_for_index(&descriptor, solution, entity_index)),
                )
            }),
            &build_sub_pillar_config(self.minimum_sub_pillar_size, self.maximum_sub_pillar_size),
        );
        let mut moves = Vec::new();
        for left_index in 0..pillars.len() {
            for right_index in (left_index + 1)..pillars.len() {
                if !binding.has_unspecified_value_range()
                    && !pillars_are_swap_compatible(
                        &pillars[left_index],
                        &pillars[right_index],
                        |entity_index| {
                            binding.values_for_entity_index(&descriptor, solution, entity_index)
                        },
                    )
                {
                    continue;
                }
                moves.push(DescriptorScalarMoveUnion::PillarSwap(
                    DescriptorPillarSwapMove::new(
                        binding.clone(),
                        pillars[left_index]
                            .pillar
                            .iter()
                            .map(|entity| entity.entity_index)
                            .collect(),
                        pillars[right_index]
                            .pillar
                            .iter()
                            .map(|entity| entity.entity_index)
                            .collect(),
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

#[derive(Clone)]
pub struct DescriptorRuinRecreateMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    min_ruin_count: usize,
    max_ruin_count: usize,
    moves_per_step: usize,
    value_candidate_limit: Option<usize>,
    recreate_heuristic_type: solverforge_config::RecreateHeuristicType,
    rng: RefCell<SmallRng>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorRuinRecreateMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorRuinRecreateMoveSelector")
            .field("binding", &self.binding)
            .field("min_ruin_count", &self.min_ruin_count)
            .field("max_ruin_count", &self.max_ruin_count)
            .field("moves_per_step", &self.moves_per_step)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .field("recreate_heuristic_type", &self.recreate_heuristic_type)
            .finish()
    }
}

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorRuinRecreateMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let solution = score_director.working_solution() as &dyn Any;
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        let assigned_indices: Vec<usize> = (0..count)
            .filter(|&entity_index| {
                let entity = descriptor
                    .get_entity(solution, binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for descriptor ruin_recreate selector");
                (binding.getter)(entity).is_some()
            })
            .collect();
        let total = assigned_indices.len();
        let min = self.min_ruin_count.min(total);
        let max = self.max_ruin_count.min(total);
        let moves_per_step = self.moves_per_step;
        let recreate_heuristic_type = self.recreate_heuristic_type;
        let value_candidate_limit = self.value_candidate_limit;
        let mut rng = self.rng.borrow_mut();
        let subsets: Vec<SmallVec<[usize; 8]>> = (0..moves_per_step)
            .map(|_| {
                if total == 0 || min == 0 {
                    return SmallVec::new();
                }
                let ruin_count = if min == max {
                    min
                } else {
                    rng.random_range(min..=max)
                };
                let mut indices = assigned_indices.clone();
                for swap_index in 0..ruin_count {
                    let other = rng.random_range(swap_index..total);
                    indices.swap(swap_index, other);
                }
                indices.truncate(ruin_count);
                SmallVec::from_vec(indices)
            })
            .collect();

        let moves: Vec<_> = subsets
            .into_iter()
            .filter_map({
                let binding = binding.clone();
                let descriptor = descriptor.clone();
                move |indices| {
                    if indices.is_empty() {
                        return None;
                    }
                    let mov = DescriptorRuinRecreateMove::new(
                        binding.clone(),
                        &indices,
                        descriptor.clone(),
                        recreate_heuristic_type,
                        value_candidate_limit,
                    );
                    mov.is_doable(score_director)
                        .then_some(DescriptorScalarMoveUnion::RuinRecreate(mov))
                }
            })
            .collect();
        ArenaMoveCursor::from_moves(moves)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        if count == 0 {
            0
        } else {
            self.moves_per_step
        }
    }
}
