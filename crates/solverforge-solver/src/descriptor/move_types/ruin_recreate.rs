#[derive(Clone)]
pub struct DescriptorRuinRecreateMove<S> {
    binding: VariableBinding,
    entity_indices: SmallVec<[usize; 8]>,
    solution_descriptor: SolutionDescriptor,
    recreate_heuristic_type: RecreateHeuristicType,
    value_candidate_limit: Option<usize>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorRuinRecreateMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorRuinRecreateMove")
            .field("descriptor_index", &self.binding.descriptor_index)
            .field("entity_indices", &self.entity_indices)
            .field("variable_name", &self.binding.variable_name)
            .field("recreate_heuristic_type", &self.recreate_heuristic_type)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .finish()
    }
}

impl<S> DescriptorRuinRecreateMove<S>
where
    S: PlanningSolution + 'static,
{
    pub(crate) fn new(
        binding: VariableBinding,
        entity_indices: &[usize],
        solution_descriptor: SolutionDescriptor,
        recreate_heuristic_type: RecreateHeuristicType,
        value_candidate_limit: Option<usize>,
    ) -> Self {
        Self {
            binding,
            entity_indices: SmallVec::from_slice(entity_indices),
            solution_descriptor,
            recreate_heuristic_type,
            value_candidate_limit,
            _phantom: PhantomData,
        }
    }

    fn current_value(&self, solution: &S, entity_index: usize) -> Option<usize> {
        let entity = self
            .solution_descriptor
            .get_entity(
                solution as &dyn Any,
                self.binding.descriptor_index,
                entity_index,
            )
            .expect("entity lookup failed for descriptor ruin_recreate move");
        (self.binding.getter)(entity)
    }

    fn values_for_entity(&self, solution: &S, entity_index: usize) -> Vec<usize> {
        self.binding
            .candidate_values_for_entity_index(
                &self.solution_descriptor,
                solution as &dyn Any,
                entity_index,
                self.value_candidate_limit,
            )
    }

    fn apply_value<D: Director<S>>(
        &self,
        score_director: &mut D,
        entity_index: usize,
        value: Option<usize>,
    ) {
        score_director.before_variable_changed(self.binding.descriptor_index, entity_index);
        let entity = self
            .solution_descriptor
            .get_entity_mut(
                score_director.working_solution_mut() as &mut dyn Any,
                self.binding.descriptor_index,
                entity_index,
            )
            .expect("entity lookup failed for descriptor ruin_recreate move");
        (self.binding.setter)(entity, value);
        score_director.after_variable_changed(self.binding.descriptor_index, entity_index);
    }

    fn evaluate_candidate<D: Director<S>>(
        &self,
        score_director: &mut D,
        entity_index: usize,
        value: usize,
    ) -> S::Score
    where
        S::Score: Score,
    {
        let old_value = self.current_value(score_director.working_solution(), entity_index);
        let score_state = score_director.snapshot_score_state();
        self.apply_value(score_director, entity_index, Some(value));
        let score = score_director.calculate_score();
        self.apply_value(score_director, entity_index, old_value);
        score_director.restore_score_state(score_state);
        score
    }

    fn choose_value<D: Director<S>>(
        &self,
        score_director: &mut D,
        entity_index: usize,
    ) -> Option<usize>
    where
        S::Score: Score,
    {
        let values = self.values_for_entity(score_director.working_solution(), entity_index);
        if values.is_empty() {
            return None;
        }

        match self.recreate_heuristic_type {
            RecreateHeuristicType::FirstFit => {
                let baseline_score = self
                    .binding
                    .allows_unassigned
                    .then(|| score_director.calculate_score());
                for value in values {
                    let score = self.evaluate_candidate(score_director, entity_index, value);
                    if baseline_score.is_none_or(|baseline| score > baseline) {
                        return Some(value);
                    }
                }
                None
            }
            RecreateHeuristicType::CheapestInsertion => {
                let baseline_score = self
                    .binding
                    .allows_unassigned
                    .then(|| score_director.calculate_score());
                let mut best: Option<(usize, usize, S::Score)> = None;
                for (value_index, value) in values.into_iter().enumerate() {
                    let score = self.evaluate_candidate(score_director, entity_index, value);
                    let should_replace = match best {
                        None => true,
                        Some((best_value_index, _, best_score)) => {
                            score > best_score
                                || (score == best_score && value_index < best_value_index)
                        }
                    };
                    if should_replace {
                        best = Some((value_index, value, score));
                    }
                }
                best.and_then(|(_, value, best_score)| {
                    baseline_score
                        .is_none_or(|baseline| best_score >= baseline)
                        .then_some(value)
                })
            }
        }
    }

    fn required_assignments_can_be_recreated(&self, solution: &S) -> bool {
        self.binding.allows_unassigned
            || self.entity_indices.iter().all(|&entity_index| {
                self.current_value(solution, entity_index).is_none()
                    || self.binding.has_candidate_values_for_entity_index(
                        &self.solution_descriptor,
                        solution as &dyn Any,
                        entity_index,
                        self.value_candidate_limit,
                    )
            })
    }
}

impl<S> Move<S> for DescriptorRuinRecreateMove<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Undo = SmallVec<[(usize, Option<usize>); 8]>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        self.required_assignments_can_be_recreated(solution)
            && self
                .entity_indices
                .iter()
                .any(|&entity_index| self.current_value(solution, entity_index).is_some())
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        if !self.is_doable(score_director) {
            return SmallVec::new();
        }

        let old_values: SmallVec<[(usize, Option<usize>); 8]> = self
            .entity_indices
            .iter()
            .map(|&entity_index| {
                (
                    entity_index,
                    self.current_value(score_director.working_solution(), entity_index),
                )
            })
            .collect();

        for &entity_index in &self.entity_indices {
            self.apply_value(score_director, entity_index, None);
        }
        for &entity_index in &self.entity_indices {
            if let Some(value) = self.choose_value(score_director, entity_index) {
                self.apply_value(score_director, entity_index, Some(value));
            }
        }

        old_values
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        for (entity_index, _) in &undo {
            score_director.before_variable_changed(self.binding.descriptor_index, *entity_index);
        }
        for (entity_index, old_value) in undo {
            let entity = self
                .solution_descriptor
                .get_entity_mut(
                    score_director.working_solution_mut() as &mut dyn Any,
                    self.binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for descriptor ruin_recreate undo");
            (self.binding.setter)(entity, old_value);
        }
        for &entity_index in &self.entity_indices {
            score_director.after_variable_changed(self.binding.descriptor_index, entity_index);
        }
    }

    fn descriptor_index(&self) -> usize {
        self.binding.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.binding.variable_name
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let variable_id = hash_str(self.binding.variable_name);
        let heuristic_id = match self.recreate_heuristic_type {
            RecreateHeuristicType::FirstFit => hash_str("first_fit"),
            RecreateHeuristicType::CheapestInsertion => hash_str("cheapest_insertion"),
        };
        let scope = MoveTabuScope::new(self.binding.descriptor_index, self.binding.variable_name);
        let entity_ids: SmallVec<[u64; 2]> = self
            .entity_indices
            .iter()
            .map(|&entity_index| encode_usize(entity_index))
            .collect();
        let entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> = entity_ids
            .iter()
            .copied()
            .map(|entity_id| scope.entity_token(entity_id))
            .collect();
        let mut move_id = smallvec![
            hash_str("descriptor_ruin_recreate"),
            encode_usize(self.binding.descriptor_index),
            variable_id,
            heuristic_id,
            encode_usize(self.entity_indices.len()),
        ];
        let mut undo_move_id = move_id.clone();
        for &entity_index in &self.entity_indices {
            let current = self.current_value(score_director.working_solution(), entity_index);
            move_id.push(encode_usize(entity_index));
            move_id.push(encode_option_usize(current));
            undo_move_id.push(encode_usize(entity_index));
            undo_move_id.push(encode_option_usize(current));
        }

        MoveTabuSignature::new(scope, move_id, undo_move_id).with_entity_tokens(entity_tokens)
    }
}
