#[derive(Clone)]
pub struct DescriptorSwapMove<S> {
    binding: VariableBinding,
    left_entity_index: usize,
    left_value: Option<usize>,
    right_entity_index: usize,
    right_value: Option<usize>,
    indices: [usize; 2],
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorSwapMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorSwapMove")
            .field("descriptor_index", &self.binding.descriptor_index)
            .field("left_entity_index", &self.left_entity_index)
            .field("right_entity_index", &self.right_entity_index)
            .field("variable_name", &self.binding.variable_name)
            .finish()
    }
}

impl<S: 'static> DescriptorSwapMove<S> {
    pub(crate) fn new_validated(
        binding: VariableBinding,
        left_entity_index: usize,
        left_value: Option<usize>,
        right_entity_index: usize,
        right_value: Option<usize>,
        solution_descriptor: SolutionDescriptor,
    ) -> Self {
        Self {
            binding,
            left_entity_index,
            left_value,
            right_entity_index,
            right_value,
            indices: [left_entity_index, right_entity_index],
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
            .expect("entity lookup failed for descriptor swap move");
        (self.binding.getter)(entity)
    }
}

impl<S> Move<S> for DescriptorSwapMove<S>
where
    S: PlanningSolution + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        self.left_entity_index != self.right_entity_index
            && self.left_value != self.right_value
            && self.current_value(solution, self.left_entity_index) == self.left_value
            && self.current_value(solution, self.right_entity_index) == self.right_value
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        let left_value =
            self.current_value(score_director.working_solution(), self.left_entity_index);
        let right_value =
            self.current_value(score_director.working_solution(), self.right_entity_index);

        score_director
            .before_variable_changed(self.binding.descriptor_index, self.left_entity_index);
        score_director
            .before_variable_changed(self.binding.descriptor_index, self.right_entity_index);

        let left_entity = self
            .solution_descriptor
            .get_entity_mut(
                score_director.working_solution_mut() as &mut dyn Any,
                self.binding.descriptor_index,
                self.left_entity_index,
            )
            .expect("entity lookup failed for descriptor swap move");
        (self.binding.setter)(left_entity, right_value);

        let right_entity = self
            .solution_descriptor
            .get_entity_mut(
                score_director.working_solution_mut() as &mut dyn Any,
                self.binding.descriptor_index,
                self.right_entity_index,
            )
            .expect("entity lookup failed for descriptor swap move");
        (self.binding.setter)(right_entity, left_value);

        score_director
            .after_variable_changed(self.binding.descriptor_index, self.left_entity_index);
        score_director
            .after_variable_changed(self.binding.descriptor_index, self.right_entity_index);

        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        let left_entity_index = self.left_entity_index;
        let right_entity_index = self.right_entity_index;
        score_director.register_undo(Box::new(move |solution: &mut S| {
            let left_entity = descriptor
                .get_entity_mut(
                    solution as &mut dyn Any,
                    binding.descriptor_index,
                    left_entity_index,
                )
                .expect("entity lookup failed for descriptor swap undo");
            (binding.setter)(left_entity, left_value);
            let right_entity = descriptor
                .get_entity_mut(
                    solution as &mut dyn Any,
                    binding.descriptor_index,
                    right_entity_index,
                )
                .expect("entity lookup failed for descriptor swap undo");
            (binding.setter)(right_entity, right_value);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.binding.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.indices
    }

    fn variable_name(&self) -> &str {
        self.binding.variable_name
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let left_val =
            self.current_value(score_director.working_solution(), self.left_entity_index);
        let right_val =
            self.current_value(score_director.working_solution(), self.right_entity_index);
        let left_id = encode_option_usize(left_val);
        let right_id = encode_option_usize(right_val);
        let left_entity_id = encode_usize(self.left_entity_index);
        let right_entity_id = encode_usize(self.right_entity_index);
        let scope = MoveTabuScope::new(self.binding.descriptor_index, self.binding.variable_name);
        let entity_pair = ordered_coordinate_pair((left_entity_id, 0), (right_entity_id, 0));
        let move_id = scoped_move_identity(
            scope,
            TABU_OP_SWAP,
            entity_pair.into_iter().map(|(entity_id, _)| entity_id),
        );

        MoveTabuSignature::new(scope, move_id.clone(), move_id)
            .with_entity_tokens([
                scope.entity_token(left_entity_id),
                scope.entity_token(right_entity_id),
            ])
            .with_destination_value_tokens([
                scope.value_token(right_id),
                scope.value_token(left_id),
            ])
    }
}

