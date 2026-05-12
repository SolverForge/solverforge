#[derive(Clone)]
pub struct DescriptorPillarSwapMove<S> {
    binding: VariableBinding,
    left_indices: Vec<usize>,
    right_indices: Vec<usize>,
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorPillarSwapMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorPillarSwapMove")
            .field("descriptor_index", &self.binding.descriptor_index)
            .field("left_indices", &self.left_indices)
            .field("right_indices", &self.right_indices)
            .field("variable_name", &self.binding.variable_name)
            .finish()
    }
}

impl<S: 'static> DescriptorPillarSwapMove<S> {
    pub(crate) fn new(
        binding: VariableBinding,
        left_indices: Vec<usize>,
        right_indices: Vec<usize>,
        solution_descriptor: SolutionDescriptor,
    ) -> Self {
        Self {
            binding,
            left_indices,
            right_indices,
            solution_descriptor,
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
            .expect("entity lookup failed for descriptor pillar swap move");
        (self.binding.getter)(entity)
    }
}

impl<S> Move<S> for DescriptorPillarSwapMove<S>
where
    S: PlanningSolution + 'static,
{
    type Undo = (Vec<(usize, Option<usize>)>, Vec<(usize, Option<usize>)>);

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let Some(&left_index) = self.left_indices.first() else {
            return false;
        };
        let Some(&right_index) = self.right_indices.first() else {
            return false;
        };

        let Some(left_value) = self.current_value(score_director.working_solution(), left_index)
        else {
            return false;
        };
        let Some(right_value) = self.current_value(score_director.working_solution(), right_index)
        else {
            return false;
        };
        if left_value == right_value {
            return false;
        }

        let solution = score_director.working_solution() as &dyn Any;
        self.left_indices.iter().all(|&entity_index| {
            self.current_value(score_director.working_solution(), entity_index) == Some(left_value)
                && self.binding.value_is_legal_for_entity_index(
                    &self.solution_descriptor,
                    solution,
                    entity_index,
                    Some(right_value),
                )
        }) && self.right_indices.iter().all(|&entity_index| {
            self.current_value(score_director.working_solution(), entity_index) == Some(right_value)
                && self.binding.value_is_legal_for_entity_index(
                    &self.solution_descriptor,
                    solution,
                    entity_index,
                    Some(left_value),
                )
        })
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let left_old: Vec<(usize, Option<usize>)> = self
            .left_indices
            .iter()
            .map(|&entity_index| {
                (
                    entity_index,
                    self.current_value(score_director.working_solution(), entity_index),
                )
            })
            .collect();
        let right_old: Vec<(usize, Option<usize>)> = self
            .right_indices
            .iter()
            .map(|&entity_index| {
                (
                    entity_index,
                    self.current_value(score_director.working_solution(), entity_index),
                )
            })
            .collect();
        let left_value = left_old.first().and_then(|(_, value)| *value);
        let right_value = right_old.first().and_then(|(_, value)| *value);

        for &entity_index in self.left_indices.iter().chain(&self.right_indices) {
            score_director.before_variable_changed(self.binding.descriptor_index, entity_index);
        }
        for &entity_index in &self.left_indices {
            let entity = self
                .solution_descriptor
                .get_entity_mut(
                    score_director.working_solution_mut() as &mut dyn Any,
                    self.binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for descriptor pillar swap move");
            (self.binding.setter)(entity, right_value);
        }
        for &entity_index in &self.right_indices {
            let entity = self
                .solution_descriptor
                .get_entity_mut(
                    score_director.working_solution_mut() as &mut dyn Any,
                    self.binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for descriptor pillar swap move");
            (self.binding.setter)(entity, left_value);
        }
        for &entity_index in self.left_indices.iter().chain(&self.right_indices) {
            score_director.after_variable_changed(self.binding.descriptor_index, entity_index);
        }

        (left_old, right_old)
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        for (entity_index, _) in undo.0.iter().chain(&undo.1) {
            score_director.before_variable_changed(self.binding.descriptor_index, *entity_index);
        }
        for (entity_index, old_value) in undo.0.into_iter().chain(undo.1) {
            let entity = self
                .solution_descriptor
                .get_entity_mut(
                    score_director.working_solution_mut() as &mut dyn Any,
                    self.binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for descriptor pillar swap undo");
            (self.binding.setter)(entity, old_value);
        }
        for &entity_index in self.left_indices.iter().chain(&self.right_indices) {
            score_director.after_variable_changed(self.binding.descriptor_index, entity_index);
        }
    }

    fn descriptor_index(&self) -> usize {
        self.binding.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.left_indices
    }

    fn variable_name(&self) -> &str {
        self.binding.variable_name
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let left_value = self.left_indices.first().and_then(|&entity_index| {
            self.current_value(score_director.working_solution(), entity_index)
        });
        let right_value = self.right_indices.first().and_then(|&entity_index| {
            self.current_value(score_director.working_solution(), entity_index)
        });
        let left_id = encode_option_usize(left_value);
        let right_id = encode_option_usize(right_value);
        let scope = MoveTabuScope::new(self.binding.descriptor_index, self.binding.variable_name);
        let mut entity_ids: SmallVec<[u64; 2]> = self
            .left_indices
            .iter()
            .chain(&self.right_indices)
            .map(|&entity_index| encode_usize(entity_index))
            .collect();
        entity_ids.sort_unstable();
        entity_ids.dedup();
        let entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> = entity_ids
            .iter()
            .copied()
            .map(|entity_id| scope.entity_token(entity_id))
            .collect();

        let mut move_id = scoped_move_identity(scope, TABU_OP_PILLAR_SWAP, std::iter::empty());
        append_canonical_usize_slice_pair(&mut move_id, &self.left_indices, &self.right_indices);

        MoveTabuSignature::new(scope, move_id.clone(), move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens([
                scope.value_token(right_id),
                scope.value_token(left_id),
            ])
    }
}
