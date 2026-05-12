#[derive(Clone)]
pub struct DescriptorPillarChangeMove<S> {
    binding: VariableBinding,
    entity_indices: Vec<usize>,
    to_value: Option<usize>,
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorPillarChangeMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorPillarChangeMove")
            .field("descriptor_index", &self.binding.descriptor_index)
            .field("entity_indices", &self.entity_indices)
            .field("variable_name", &self.binding.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S: 'static> DescriptorPillarChangeMove<S> {
    pub(crate) fn new(
        binding: VariableBinding,
        entity_indices: Vec<usize>,
        to_value: Option<usize>,
        solution_descriptor: SolutionDescriptor,
    ) -> Self {
        Self {
            binding,
            entity_indices,
            to_value,
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
            .expect("entity lookup failed for descriptor pillar change move");
        (self.binding.getter)(entity)
    }
}

impl<S> Move<S> for DescriptorPillarChangeMove<S>
where
    S: PlanningSolution + 'static,
{
    type Undo = Vec<(usize, Option<usize>)>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        if self.entity_indices.is_empty() {
            return false;
        }

        let solution = score_director.working_solution() as &dyn Any;
        self.entity_indices.iter().any(|&entity_index| {
            self.current_value(score_director.working_solution(), entity_index) != self.to_value
        }) && self.entity_indices.iter().all(|&entity_index| {
            self.binding.value_is_legal_for_entity_index(
                &self.solution_descriptor,
                solution,
                entity_index,
                self.to_value,
            )
        })
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let old_values: Vec<(usize, Option<usize>)> = self
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
            score_director.before_variable_changed(self.binding.descriptor_index, entity_index);
        }
        for &entity_index in &self.entity_indices {
            let entity = self
                .solution_descriptor
                .get_entity_mut(
                    score_director.working_solution_mut() as &mut dyn Any,
                    self.binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for descriptor pillar change move");
            (self.binding.setter)(entity, self.to_value);
        }
        for &entity_index in &self.entity_indices {
            score_director.after_variable_changed(self.binding.descriptor_index, entity_index);
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
                .expect("entity lookup failed for descriptor pillar change undo");
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
        let from_value = self.entity_indices.first().and_then(|&entity_index| {
            self.current_value(score_director.working_solution(), entity_index)
        });
        let from_id = encode_option_usize(from_value);
        let to_id = encode_option_usize(self.to_value);
        let variable_id = hash_str(self.binding.variable_name);
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
            encode_usize(self.binding.descriptor_index),
            variable_id,
            encode_usize(self.entity_indices.len()),
            from_id,
            to_id
        ];
        move_id.extend(entity_ids.iter().copied());

        let mut undo_move_id = smallvec![
            encode_usize(self.binding.descriptor_index),
            variable_id,
            encode_usize(self.entity_indices.len()),
            to_id,
            from_id
        ];
        undo_move_id.extend(entity_ids.iter().copied());

        MoveTabuSignature::new(scope, move_id, undo_move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens([scope.value_token(to_id)])
    }
}
