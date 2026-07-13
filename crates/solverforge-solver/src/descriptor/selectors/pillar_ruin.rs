struct DescriptorPillarChangeInput {
    entity_indices: Vec<usize>,
    values: Vec<usize>,
}

pub struct DescriptorPillarChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    store: CandidateStore<S, DescriptorMoveUnion<S>>,
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    inputs: Vec<DescriptorPillarChangeInput>,
    input_offset: usize,
    value_offset: usize,
}

impl<S> MoveCursor<S, DescriptorMoveUnion<S>> for DescriptorPillarChangeMoveCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        while let Some(input) = self.inputs.get(self.input_offset) {
            if let Some(value) = input.values.get(self.value_offset).copied() {
                self.value_offset += 1;
                return Some(self.store.push(DescriptorMoveUnion::PillarChange(
                    DescriptorPillarChangeMove::new(
                        self.binding.clone(),
                        input.entity_indices.clone(),
                        Some(value),
                        self.solution_descriptor.clone(),
                    ),
                )));
            }
            self.input_offset += 1;
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

impl<S> Iterator for DescriptorPillarChangeMoveCursor<S>
where
    S: PlanningSolution + 'static,
{
    type Item = DescriptorMoveUnion<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

pub struct DescriptorPillarSwapMoveCursor<S>
where
    S: PlanningSolution,
{
    store: CandidateStore<S, DescriptorMoveUnion<S>>,
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    pillars: Vec<Vec<usize>>,
    pairs: std::vec::IntoIter<(usize, usize)>,
}

impl<S> MoveCursor<S, DescriptorMoveUnion<S>> for DescriptorPillarSwapMoveCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        let (left_index, right_index) = self.pairs.next()?;
        Some(self.store.push(DescriptorMoveUnion::PillarSwap(
            DescriptorPillarSwapMove::new(
                self.binding.clone(),
                self.pillars[left_index].clone(),
                self.pillars[right_index].clone(),
                self.solution_descriptor.clone(),
            ),
        )))
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

impl<S> Iterator for DescriptorPillarSwapMoveCursor<S>
where
    S: PlanningSolution + 'static,
{
    type Item = DescriptorMoveUnion<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

pub struct DescriptorRuinRecreateMoveCursor<S>
where
    S: PlanningSolution,
{
    store: CandidateStore<S, DescriptorMoveUnion<S>>,
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    subsets: std::vec::IntoIter<SmallVec<[usize; 8]>>,
    recreatable: Vec<bool>,
    recreate_heuristic_type: RecreateHeuristicType,
    value_candidate_limit: Option<usize>,
}

impl<S> MoveCursor<S, DescriptorMoveUnion<S>> for DescriptorRuinRecreateMoveCursor<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        for indices in self.subsets.by_ref() {
            if indices.is_empty()
                || (!self.binding.allows_unassigned
                    && !indices.iter().all(|&entity_index| {
                        self.recreatable.get(entity_index).copied().unwrap_or(false)
                    }))
            {
                continue;
            }
            return Some(self.store.push(DescriptorMoveUnion::RuinRecreate(
                DescriptorRuinRecreateMove::new(
                    self.binding.clone(),
                    &indices,
                    self.solution_descriptor.clone(),
                    self.recreate_heuristic_type,
                    self.value_candidate_limit,
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

impl<S> Iterator for DescriptorRuinRecreateMoveCursor<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Item = DescriptorMoveUnion<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

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

impl<S> MoveSelector<S, DescriptorMoveUnion<S>> for DescriptorPillarChangeMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorPillarChangeMoveCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let solution = score_director.working_solution() as &dyn Any;
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let inputs = collect_pillar_groups(
            (0..count).map(|entity_index| {
                (
                    EntityReference::new(self.binding.descriptor_index, entity_index),
                    (self.binding.getter)(self.binding.entity_for_index(
                        &self.solution_descriptor,
                        solution,
                        entity_index,
                    )),
                )
            }),
            &build_sub_pillar_config(self.minimum_sub_pillar_size, self.maximum_sub_pillar_size),
        )
        .into_iter()
        .map(|group| {
            let entity_indices: Vec<usize> = group
                .pillar
                .iter()
                .map(|entity| entity.entity_index)
                .collect();
            let values = intersect_legal_values_for_pillar(&group.pillar, |entity_index| {
                self.binding.candidate_values_for_entity_index(
                    &self.solution_descriptor,
                    solution,
                    entity_index,
                    self.value_candidate_limit,
                )
            })
            .into_iter()
            .filter(|&value| value != group.shared_value)
            .collect();
            DescriptorPillarChangeInput {
                entity_indices,
                values,
            }
        })
        .collect();
        DescriptorPillarChangeMoveCursor {
            store: CandidateStore::new(),
            binding: self.binding.clone(),
            solution_descriptor: self.solution_descriptor.clone(),
            inputs,
            input_offset: 0,
            value_offset: 0,
        }
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

impl<S> MoveSelector<S, DescriptorMoveUnion<S>> for DescriptorPillarSwapMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorPillarSwapMoveCursor<S>
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
        let mut pairs = Vec::new();
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
                pairs.push((left_index, right_index));
            }
        }
        DescriptorPillarSwapMoveCursor {
            store: CandidateStore::new(),
            binding,
            solution_descriptor: descriptor,
            pillars: pillars
                .into_iter()
                .map(|group| {
                    group
                        .pillar
                        .iter()
                        .map(|entity| entity.entity_index)
                        .collect()
                })
                .collect(),
            pairs: pairs.into_iter(),
        }
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

impl<S> MoveSelector<S, DescriptorMoveUnion<S>> for DescriptorRuinRecreateMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorRuinRecreateMoveCursor<S>
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
        let recreatable = (0..count)
            .map(|entity_index| {
                binding.has_candidate_values_for_entity_index(
                    &descriptor,
                    solution,
                    entity_index,
                    self.value_candidate_limit,
                )
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

        DescriptorRuinRecreateMoveCursor {
            store: CandidateStore::new(),
            binding,
            solution_descriptor: descriptor,
            subsets: subsets.into_iter(),
            recreatable,
            recreate_heuristic_type,
            value_candidate_limit,
        }
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
