use std::any::Any;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use smallvec::{smallvec, SmallVec};
use solverforge_config::RecreateHeuristicType;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;
use solverforge_scoring::{Director, RecordingDirector};

use crate::heuristic::r#move::metadata::{
    encode_option_usize, encode_usize, hash_str, MoveTabuScope, ScopedEntityTabuToken,
};
use crate::heuristic::r#move::{Move, MoveTabuSignature, SequentialCompositeMove};

use super::bindings::VariableBinding;

#[derive(Clone)]
pub struct DescriptorChangeMove<S> {
    binding: VariableBinding,
    entity_index: usize,
    to_value: Option<usize>,
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
            solution_descriptor,
            _phantom: PhantomData,
        }
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
}

impl<S> Move<S> for DescriptorChangeMove<S>
where
    S: PlanningSolution + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        self.current_value(score_director.working_solution()) != self.to_value
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
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

        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        let entity_index = self.entity_index;
        score_director.register_undo(Box::new(move |solution: &mut S| {
            let entity = descriptor
                .get_entity_mut(
                    solution as &mut dyn Any,
                    binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for descriptor change undo");
            (binding.setter)(entity, old_value);
        }));
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

#[derive(Clone)]
pub struct DescriptorSwapMove<S> {
    binding: VariableBinding,
    left_entity_index: usize,
    right_entity_index: usize,
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
    pub(crate) fn new(
        binding: VariableBinding,
        left_entity_index: usize,
        right_entity_index: usize,
        solution_descriptor: SolutionDescriptor,
    ) -> Self {
        Self {
            binding,
            left_entity_index,
            right_entity_index,
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
        self.left_entity_index != self.right_entity_index
            && self.current_value(score_director.working_solution(), self.left_entity_index)
                != self.current_value(score_director.working_solution(), self.right_entity_index)
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
        let variable_id = hash_str(self.binding.variable_name);
        let scope = MoveTabuScope::new(self.binding.descriptor_index, self.binding.variable_name);

        MoveTabuSignature::new(
            scope,
            smallvec![
                encode_usize(self.binding.descriptor_index),
                variable_id,
                left_entity_id,
                right_entity_id,
                left_id,
                right_id
            ],
            smallvec![
                encode_usize(self.binding.descriptor_index),
                variable_id,
                left_entity_id,
                right_entity_id,
                left_id,
                right_id
            ],
        )
        .with_entity_tokens([
            scope.entity_token(left_entity_id),
            scope.entity_token(right_entity_id),
        ])
        .with_destination_value_tokens([scope.value_token(right_id), scope.value_token(left_id)])
    }
}

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

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
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

        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        score_director.register_undo(Box::new(move |solution: &mut S| {
            for (entity_index, old_value) in old_values {
                let entity = descriptor
                    .get_entity_mut(
                        solution as &mut dyn Any,
                        binding.descriptor_index,
                        entity_index,
                    )
                    .expect("entity lookup failed for descriptor pillar change undo");
                (binding.setter)(entity, old_value);
            }
        }));
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

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
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

        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        score_director.register_undo(Box::new(move |solution: &mut S| {
            for (entity_index, old_value) in left_old.iter().chain(&right_old) {
                let entity = descriptor
                    .get_entity_mut(
                        solution as &mut dyn Any,
                        binding.descriptor_index,
                        *entity_index,
                    )
                    .expect("entity lookup failed for descriptor pillar swap undo");
                (binding.setter)(entity, *old_value);
            }
        }));
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
        let variable_id = hash_str(self.binding.variable_name);
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

        let mut move_id = smallvec![
            encode_usize(self.binding.descriptor_index),
            variable_id,
            encode_usize(self.left_indices.len()),
            encode_usize(self.right_indices.len()),
            left_id,
            right_id
        ];
        move_id.extend(
            self.left_indices
                .iter()
                .map(|&entity_index| encode_usize(entity_index)),
        );
        move_id.push(u64::MAX);
        move_id.extend(
            self.right_indices
                .iter()
                .map(|&entity_index| encode_usize(entity_index)),
        );

        MoveTabuSignature::new(scope, move_id.clone(), move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens([
                scope.value_token(right_id),
                scope.value_token(left_id),
            ])
    }
}

#[derive(Clone)]
pub struct DescriptorRuinRecreateMove<S> {
    binding: VariableBinding,
    entity_indices: SmallVec<[usize; 8]>,
    solution_descriptor: SolutionDescriptor,
    recreate_heuristic_type: RecreateHeuristicType,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorRuinRecreateMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorRuinRecreateMove")
            .field("descriptor_index", &self.binding.descriptor_index)
            .field("entity_indices", &self.entity_indices)
            .field("variable_name", &self.binding.variable_name)
            .field("recreate_heuristic_type", &self.recreate_heuristic_type)
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
    ) -> Self {
        Self {
            binding,
            entity_indices: SmallVec::from_slice(entity_indices),
            solution_descriptor,
            recreate_heuristic_type,
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
        let entity = self
            .solution_descriptor
            .get_entity(
                solution as &dyn Any,
                self.binding.descriptor_index,
                entity_index,
            )
            .expect("entity lookup failed for descriptor ruin_recreate move");
        self.binding
            .values_for_entity(&self.solution_descriptor, solution as &dyn Any, entity)
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
        let mut recording = RecordingDirector::new(score_director);
        DescriptorChangeMove::new(
            self.binding.clone(),
            entity_index,
            Some(value),
            self.solution_descriptor.clone(),
        )
        .do_move(&mut recording);
        let score = recording.calculate_score();
        recording.undo_changes();
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
}

impl<S> Move<S> for DescriptorRuinRecreateMove<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        self.entity_indices.iter().any(|&entity_index| {
            self.current_value(score_director.working_solution(), entity_index)
                .is_some()
        })
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
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

        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        score_director.register_undo(Box::new(move |solution: &mut S| {
            for (entity_index, old_value) in old_values {
                let entity = descriptor
                    .get_entity_mut(
                        solution as &mut dyn Any,
                        binding.descriptor_index,
                        entity_index,
                    )
                    .expect("entity lookup failed for descriptor ruin_recreate undo");
                (binding.setter)(entity, old_value);
            }
        }));
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

pub enum DescriptorScalarMoveUnion<S> {
    Change(DescriptorChangeMove<S>),
    Swap(DescriptorSwapMove<S>),
    PillarChange(DescriptorPillarChangeMove<S>),
    PillarSwap(DescriptorPillarSwapMove<S>),
    RuinRecreate(DescriptorRuinRecreateMove<S>),
    Composite(SequentialCompositeMove<S, DescriptorScalarMoveUnion<S>>),
}

impl<S> Clone for DescriptorScalarMoveUnion<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn clone(&self) -> Self {
        match self {
            Self::Change(m) => Self::Change(m.clone()),
            Self::Swap(m) => Self::Swap(m.clone()),
            Self::PillarChange(m) => Self::PillarChange(m.clone()),
            Self::PillarSwap(m) => Self::PillarSwap(m.clone()),
            Self::RuinRecreate(m) => Self::RuinRecreate(m.clone()),
            Self::Composite(m) => Self::Composite(m.clone()),
        }
    }
}

impl<S> Debug for DescriptorScalarMoveUnion<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(m) => m.fmt(f),
            Self::Swap(m) => m.fmt(f),
            Self::PillarChange(m) => m.fmt(f),
            Self::PillarSwap(m) => m.fmt(f),
            Self::RuinRecreate(m) => m.fmt(f),
            Self::Composite(m) => m.fmt(f),
        }
    }
}

impl<S> Move<S> for DescriptorScalarMoveUnion<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Change(m) => m.is_doable(score_director),
            Self::Swap(m) => m.is_doable(score_director),
            Self::PillarChange(m) => m.is_doable(score_director),
            Self::PillarSwap(m) => m.is_doable(score_director),
            Self::RuinRecreate(m) => m.is_doable(score_director),
            Self::Composite(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        match self {
            Self::Change(m) => m.do_move(score_director),
            Self::Swap(m) => m.do_move(score_director),
            Self::PillarChange(m) => m.do_move(score_director),
            Self::PillarSwap(m) => m.do_move(score_director),
            Self::RuinRecreate(m) => m.do_move(score_director),
            Self::Composite(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Change(m) => m.descriptor_index(),
            Self::Swap(m) => m.descriptor_index(),
            Self::PillarChange(m) => m.descriptor_index(),
            Self::PillarSwap(m) => m.descriptor_index(),
            Self::RuinRecreate(m) => m.descriptor_index(),
            Self::Composite(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Change(m) => m.entity_indices(),
            Self::Swap(m) => m.entity_indices(),
            Self::PillarChange(m) => m.entity_indices(),
            Self::PillarSwap(m) => m.entity_indices(),
            Self::RuinRecreate(m) => m.entity_indices(),
            Self::Composite(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Change(m) => m.variable_name(),
            Self::Swap(m) => m.variable_name(),
            Self::PillarChange(m) => m.variable_name(),
            Self::PillarSwap(m) => m.variable_name(),
            Self::RuinRecreate(m) => m.variable_name(),
            Self::Composite(m) => m.variable_name(),
        }
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        match self {
            Self::Change(m) => m.tabu_signature(score_director),
            Self::Swap(m) => m.tabu_signature(score_director),
            Self::PillarChange(m) => m.tabu_signature(score_director),
            Self::PillarSwap(m) => m.tabu_signature(score_director),
            Self::RuinRecreate(m) => m.tabu_signature(score_director),
            Self::Composite(m) => m.tabu_signature(score_director),
        }
    }
}
