use std::any::Any;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_scoring::Director;

use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::move_selector::MoveSelector;

use super::bindings::{collect_bindings, find_binding, VariableBinding};
use super::move_types::{DescriptorChangeMove, DescriptorEitherMove, DescriptorSwapMove};

#[derive(Clone)]
pub struct DescriptorChangeMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    allows_unassigned: bool,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorChangeMoveSelector")
            .field("binding", &self.binding)
            .finish()
    }
}

impl<S> DescriptorChangeMoveSelector<S> {
    fn new(binding: VariableBinding, solution_descriptor: SolutionDescriptor) -> Self {
        let allows_unassigned = binding.allows_unassigned;
        Self {
            binding,
            solution_descriptor,
            allows_unassigned,
            _phantom: PhantomData,
        }
    }
}

impl<S> MoveSelector<S, DescriptorEitherMove<S>> for DescriptorChangeMoveSelector<S>
where
    S: PlanningSolution + 'static,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorEitherMove<S>> + 'a {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        let allows_unassigned = self.allows_unassigned;
        let solution = score_director.working_solution() as &dyn Any;
        let moves: Vec<_> = (0..count)
            .flat_map(move |entity_index| {
                let entity = descriptor
                    .get_entity(solution, binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for change selector");
                let current_value = (binding.getter)(entity);
                let unassign_move = (allows_unassigned && current_value.is_some()).then({
                    let binding = binding.clone();
                    let descriptor = descriptor.clone();
                    move || {
                        DescriptorEitherMove::Change(DescriptorChangeMove::new(
                            binding.clone(),
                            entity_index,
                            None,
                            descriptor.clone(),
                        ))
                    }
                });
                binding
                    .values_for_entity(&descriptor, solution, entity)
                    .into_iter()
                    .map({
                        let binding = binding.clone();
                        let descriptor = descriptor.clone();
                        move |value| {
                            DescriptorEitherMove::Change(DescriptorChangeMove::new(
                                binding.clone(),
                                entity_index,
                                Some(value),
                                descriptor.clone(),
                            ))
                        }
                    })
                    .chain(unassign_move)
            })
            .collect();
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let mut total = 0;
        for entity_index in 0..count {
            let entity = self
                .solution_descriptor
                .get_entity(
                    score_director.working_solution() as &dyn Any,
                    self.binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for change selector");
            total += self
                .binding
                .values_for_entity(
                    &self.solution_descriptor,
                    score_director.working_solution() as &dyn Any,
                    entity,
                )
                .len()
                + usize::from(self.allows_unassigned && (self.binding.getter)(entity).is_some());
        }
        total
    }
}

#[derive(Clone)]
pub struct DescriptorSwapMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorSwapMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorSwapMoveSelector")
            .field("binding", &self.binding)
            .finish()
    }
}

impl<S> DescriptorSwapMoveSelector<S> {
    fn new(binding: VariableBinding, solution_descriptor: SolutionDescriptor) -> Self {
        Self {
            binding,
            solution_descriptor,
            _phantom: PhantomData,
        }
    }
}

impl<S> MoveSelector<S, DescriptorEitherMove<S>> for DescriptorSwapMoveSelector<S>
where
    S: PlanningSolution + 'static,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorEitherMove<S>> + 'a {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let binding = self.binding.clone();
        let descriptor = self.solution_descriptor.clone();
        let moves: Vec<_> = (0..count)
            .flat_map(move |left_entity_index| {
                ((left_entity_index + 1)..count).map({
                    let binding = binding.clone();
                    let descriptor = descriptor.clone();
                    move |right_entity_index| {
                        DescriptorEitherMove::Swap(DescriptorSwapMove::new(
                            binding.clone(),
                            left_entity_index,
                            right_entity_index,
                            descriptor.clone(),
                        ))
                    }
                })
            })
            .collect();
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        count.saturating_mul(count.saturating_sub(1)) / 2
    }
}

#[derive(Clone)]
pub enum DescriptorLeafSelector<S> {
    Change(DescriptorChangeMoveSelector<S>),
    Swap(DescriptorSwapMoveSelector<S>),
}

impl<S> Debug for DescriptorLeafSelector<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(selector) => selector.fmt(f),
            Self::Swap(selector) => selector.fmt(f),
        }
    }
}

impl<S> MoveSelector<S, DescriptorEitherMove<S>> for DescriptorLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorEitherMove<S>> + 'a {
        enum DescriptorLeafIter<A, B> {
            Change(A),
            Swap(B),
        }

        impl<T, A, B> Iterator for DescriptorLeafIter<A, B>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Change(iter) => iter.next(),
                    Self::Swap(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Change(selector) => {
                DescriptorLeafIter::Change(selector.open_cursor(score_director))
            }
            Self::Swap(selector) => DescriptorLeafIter::Swap(selector.open_cursor(score_director)),
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Change(selector) => selector.size(score_director),
            Self::Swap(selector) => selector.size(score_director),
        }
    }
}

fn collect_descriptor_leaf_selectors<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
) -> Vec<DescriptorLeafSelector<S>>
where
    S: PlanningSolution + 'static,
{
    let bindings = collect_bindings(descriptor);
    let mut leaves = Vec::new();

    fn collect<S>(
        cfg: &MoveSelectorConfig,
        descriptor: &SolutionDescriptor,
        bindings: &[VariableBinding],
        leaves: &mut Vec<DescriptorLeafSelector<S>>,
    ) where
        S: PlanningSolution + 'static,
    {
        match cfg {
            MoveSelectorConfig::ChangeMoveSelector(change) => {
                let matched = find_binding(
                    bindings,
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                );
                assert!(
                    !matched.is_empty(),
                    "change_move selector matched no standard planning variables for entity_class={:?} variable_name={:?}",
                    change.target.entity_class,
                    change.target.variable_name
                );
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::Change(
                        DescriptorChangeMoveSelector::new(binding, descriptor.clone()),
                    ));
                }
            }
            MoveSelectorConfig::SwapMoveSelector(swap) => {
                let matched = find_binding(
                    bindings,
                    swap.target.entity_class.as_deref(),
                    swap.target.variable_name.as_deref(),
                );
                assert!(
                    !matched.is_empty(),
                    "swap_move selector matched no standard planning variables for entity_class={:?} variable_name={:?}",
                    swap.target.entity_class,
                    swap.target.variable_name
                );
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::Swap(
                        DescriptorSwapMoveSelector::new(binding, descriptor.clone()),
                    ));
                }
            }
            MoveSelectorConfig::UnionMoveSelector(union) => {
                for child in &union.selectors {
                    collect(child, descriptor, bindings, leaves);
                }
            }
            MoveSelectorConfig::LimitedNeighborhood(_) => {
                panic!("limited_neighborhood must be handled by the canonical runtime");
            }
            MoveSelectorConfig::ListChangeMoveSelector(_)
            | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
            | MoveSelectorConfig::ListSwapMoveSelector(_)
            | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
            | MoveSelectorConfig::SubListChangeMoveSelector(_)
            | MoveSelectorConfig::SubListSwapMoveSelector(_)
            | MoveSelectorConfig::ListReverseMoveSelector(_)
            | MoveSelectorConfig::KOptMoveSelector(_)
            | MoveSelectorConfig::ListRuinMoveSelector(_) => {
                panic!("list move selector configured against a standard-variable context");
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                panic!("cartesian_product move selectors are not supported in the canonical solver path");
            }
        }
    }

    match config {
        Some(cfg) => collect(cfg, descriptor, &bindings, &mut leaves),
        None => {
            for binding in bindings {
                leaves.push(DescriptorLeafSelector::Change(
                    DescriptorChangeMoveSelector::new(binding.clone(), descriptor.clone()),
                ));
                leaves.push(DescriptorLeafSelector::Swap(
                    DescriptorSwapMoveSelector::new(binding, descriptor.clone()),
                ));
            }
        }
    }

    assert!(
        !leaves.is_empty(),
        "move selector configuration produced no standard neighborhoods"
    );

    leaves
}

pub fn build_descriptor_move_selector<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
) -> VecUnionSelector<S, DescriptorEitherMove<S>, DescriptorLeafSelector<S>>
where
    S: PlanningSolution + 'static,
{
    VecUnionSelector::new(collect_descriptor_leaf_selectors(config, descriptor))
}
