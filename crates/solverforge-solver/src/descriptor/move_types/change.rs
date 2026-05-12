#[derive(Clone)]
pub struct DescriptorChangeMove<S> {
    binding: VariableBinding,
    entity_index: usize,
    to_value: Option<usize>,
    construction_value_order_key: Option<ConstructionValueOrderKey<S>>,
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorChangeMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorChangeMove")
            .field("descriptor_index", &self.binding.descriptor_index)
            .field("entity_index", &self.entity_index)
            .field("variable_name", &self.binding.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S: 'static> DescriptorChangeMove<S> {
    pub(crate) fn new(
        binding: VariableBinding,
        entity_index: usize,
        to_value: Option<usize>,
        solution_descriptor: SolutionDescriptor,
    ) -> Self {
        Self {
            binding,
            entity_index,
            to_value,
            construction_value_order_key: None,
            solution_descriptor,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn with_construction_value_order_key(
        mut self,
        order_key: ConstructionValueOrderKey<S>,
    ) -> Self {
        self.construction_value_order_key = Some(order_key);
        self
    }

    fn current_value(&self, solution: &S) -> Option<usize> {
        let entity = self
            .solution_descriptor
            .get_entity(
                solution as &dyn Any,
                self.binding.descriptor_index,
                self.entity_index,
            )
            .expect("entity lookup failed for descriptor change move");
        (self.binding.getter)(entity)
    }

    pub(crate) fn live_value_order_key(&self, solution: &S) -> Option<i64> {
        self.to_value.map(|value| {
            self.construction_value_order_key
                .and_then(|order_key| {
                    order_key(
                        solution,
                        self.entity_index,
                        self.binding.variable_index,
                        value,
                    )
                })
                .or_else(|| {
                    self.binding
                        .value_order_key(solution as &dyn Any, self.entity_index, value)
                })
                .unwrap_or(0)
        })
    }
}

impl<S> Move<S> for DescriptorChangeMove<S>
where
    S: PlanningSolution + 'static,
{
    type Undo = Option<usize>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        self.current_value(score_director.working_solution()) != self.to_value
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let old_value = self.current_value(score_director.working_solution());
        score_director.before_variable_changed(self.binding.descriptor_index, self.entity_index);
        let entity = self
            .solution_descriptor
            .get_entity_mut(
                score_director.working_solution_mut() as &mut dyn Any,
                self.binding.descriptor_index,
                self.entity_index,
            )
            .expect("entity lookup failed for descriptor change move");
        (self.binding.setter)(entity, self.to_value);
        score_director.after_variable_changed(self.binding.descriptor_index, self.entity_index);

        old_value
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        score_director.before_variable_changed(self.binding.descriptor_index, self.entity_index);
        let entity = self
            .solution_descriptor
            .get_entity_mut(
                score_director.working_solution_mut() as &mut dyn Any,
                self.binding.descriptor_index,
                self.entity_index,
            )
            .expect("entity lookup failed for descriptor change undo");
        (self.binding.setter)(entity, undo);
        score_director.after_variable_changed(self.binding.descriptor_index, self.entity_index);
    }

    fn descriptor_index(&self) -> usize {
        self.binding.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.binding.variable_name
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let current = self.current_value(score_director.working_solution());
        let from_id = encode_option_usize(current);
        let to_id = encode_option_usize(self.to_value);
        let entity_id = encode_usize(self.entity_index);
        let variable_id = hash_str(self.binding.variable_name);
        let scope = MoveTabuScope::new(self.binding.descriptor_index, self.binding.variable_name);

        MoveTabuSignature::new(
            scope,
            smallvec![
                encode_usize(self.binding.descriptor_index),
                variable_id,
                entity_id,
                from_id,
                to_id
            ],
            smallvec![
                encode_usize(self.binding.descriptor_index),
                variable_id,
                entity_id,
                to_id,
                from_id
            ],
        )
        .with_entity_tokens([scope.entity_token(entity_id)])
        .with_destination_value_tokens([scope.value_token(to_id)])
    }
}
