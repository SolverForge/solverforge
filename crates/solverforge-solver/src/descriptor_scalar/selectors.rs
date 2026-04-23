use std::any::Any;
use std::cell::RefCell;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use smallvec::SmallVec;
use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::{CartesianProductSelector, VecUnionSelector};
use crate::heuristic::selector::entity::EntityReference;
use crate::heuristic::selector::move_selector::MoveSelector;
use crate::heuristic::selector::nearby_support::truncate_nearby_candidates;
use crate::heuristic::selector::pillar::SubPillarConfig;
use crate::heuristic::selector::pillar_support::{
    collect_pillar_groups, intersect_legal_values_for_pillar, pillars_are_swap_compatible,
};
use crate::heuristic::selector::seed::scoped_seed;

use super::bindings::{collect_bindings, find_binding, VariableBinding};
use super::move_types::{
    DescriptorChangeMove, DescriptorPillarChangeMove, DescriptorPillarSwapMove,
    DescriptorRuinRecreateMove, DescriptorScalarMoveUnion, DescriptorSwapMove,
};

pub type DescriptorFlatSelector<S> =
    VecUnionSelector<S, DescriptorScalarMoveUnion<S>, DescriptorLeafSelector<S>>;
type DescriptorCartesianSelector<S> = CartesianProductSelector<
    S,
    DescriptorScalarMoveUnion<S>,
    DescriptorFlatSelector<S>,
    DescriptorFlatSelector<S>,
>;
pub type DescriptorSelector<S> =
    VecUnionSelector<S, DescriptorScalarMoveUnion<S>, DescriptorSelectorNode<S>>;

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

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorChangeMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorScalarMoveUnion<S>> + 'a {
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
                        DescriptorScalarMoveUnion::Change(DescriptorChangeMove::new(
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
                            DescriptorScalarMoveUnion::Change(DescriptorChangeMove::new(
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

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorSwapMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorScalarMoveUnion<S>> + 'a {
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
                        DescriptorScalarMoveUnion::Swap(DescriptorSwapMove::new(
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
pub struct DescriptorNearbyChangeMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    max_nearby: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorNearbyChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorNearbyChangeMoveSelector")
            .field("binding", &self.binding)
            .field("max_nearby", &self.max_nearby)
            .finish()
    }
}

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorNearbyChangeMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorScalarMoveUnion<S>> + 'a {
        let distance_meter = self
            .binding
            .nearby_value_distance_meter
            .expect("nearby change requires a nearby value distance meter");
        let solution = score_director.working_solution() as &dyn Any;
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let binding = self.binding.clone();
        let descriptor = self.solution_descriptor.clone();
        let max_nearby = self.max_nearby;
        let moves: Vec<_> = (0..count)
            .flat_map(move |entity_index| {
                let entity = descriptor
                    .get_entity(solution, binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for nearby change selector");
                let current_value = (binding.getter)(entity);
                let current_assigned = current_value.is_some();
                let mut candidates: Vec<(usize, f64, usize)> = binding
                    .values_for_entity(&descriptor, solution, entity)
                    .into_iter()
                    .enumerate()
                    .filter_map(|(order, value)| {
                        if current_value == Some(value) {
                            return None;
                        }
                        let distance = distance_meter(solution, entity_index, value);
                        distance.is_finite().then_some((value, distance, order))
                    })
                    .collect();
                truncate_nearby_candidates(&mut candidates, max_nearby);

                let candidate_moves = candidates.into_iter().map({
                    let binding = binding.clone();
                    let descriptor = descriptor.clone();
                    move |(value, _, _)| {
                        DescriptorScalarMoveUnion::Change(DescriptorChangeMove::new(
                            binding.clone(),
                            entity_index,
                            Some(value),
                            descriptor.clone(),
                        ))
                    }
                });
                let unassign = (binding.allows_unassigned && current_assigned).then({
                    let binding = binding.clone();
                    let descriptor = descriptor.clone();
                    move || {
                        DescriptorScalarMoveUnion::Change(DescriptorChangeMove::new(
                            binding.clone(),
                            entity_index,
                            None,
                            descriptor.clone(),
                        ))
                    }
                });
                candidate_moves.chain(unassign)
            })
            .collect();
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }
}

#[derive(Clone)]
pub struct DescriptorNearbySwapMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    max_nearby: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorNearbySwapMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorNearbySwapMoveSelector")
            .field("binding", &self.binding)
            .field("max_nearby", &self.max_nearby)
            .finish()
    }
}

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorNearbySwapMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorScalarMoveUnion<S>> + 'a {
        let distance_meter = self
            .binding
            .nearby_entity_distance_meter
            .expect("nearby swap requires a nearby entity distance meter");
        let solution = score_director.working_solution() as &dyn Any;
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let binding = self.binding.clone();
        let descriptor = self.solution_descriptor.clone();
        let max_nearby = self.max_nearby;
        let current_values: Vec<_> = (0..count)
            .map(|entity_index| {
                let entity = descriptor
                    .get_entity(solution, binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for nearby swap selector");
                (binding.getter)(entity)
            })
            .collect();
        let moves: Vec<_> = (0..count)
            .flat_map(move |left_entity_index| {
                let left_value = current_values[left_entity_index];
                let mut candidates: Vec<(usize, f64, usize)> = ((left_entity_index + 1)..count)
                    .enumerate()
                    .filter_map(|(order, right_entity_index)| {
                        if left_value == current_values[right_entity_index] {
                            return None;
                        }
                        let distance =
                            distance_meter(solution, left_entity_index, right_entity_index);
                        distance
                            .is_finite()
                            .then_some((right_entity_index, distance, order))
                    })
                    .collect();
                truncate_nearby_candidates(&mut candidates, max_nearby);
                candidates.into_iter().map({
                    let binding = binding.clone();
                    let descriptor = descriptor.clone();
                    move |(right_entity_index, _, _)| {
                        DescriptorScalarMoveUnion::Swap(DescriptorSwapMove::new(
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
        self.open_cursor(score_director).count()
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
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorPillarChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorPillarChangeMoveSelector")
            .field("binding", &self.binding)
            .field("minimum_sub_pillar_size", &self.minimum_sub_pillar_size)
            .field("maximum_sub_pillar_size", &self.maximum_sub_pillar_size)
            .finish()
    }
}

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorPillarChangeMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorScalarMoveUnion<S>> + 'a {
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
                binding.values_for_entity_index(&descriptor, solution, entity_index)
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
        moves.into_iter()
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
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorScalarMoveUnion<S>> + 'a {
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
                if !pillars_are_swap_compatible(
                    &pillars[left_index],
                    &pillars[right_index],
                    |entity_index| {
                        binding.values_for_entity_index(&descriptor, solution, entity_index)
                    },
                ) {
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
        moves.into_iter()
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
            .field("recreate_heuristic_type", &self.recreate_heuristic_type)
            .finish()
    }
}

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorRuinRecreateMoveSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorScalarMoveUnion<S>> + 'a {
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
                    );
                    mov.is_doable(score_director)
                        .then_some(DescriptorScalarMoveUnion::RuinRecreate(mov))
                }
            })
            .collect();
        moves.into_iter()
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

pub enum DescriptorLeafSelector<S> {
    Change(DescriptorChangeMoveSelector<S>),
    Swap(DescriptorSwapMoveSelector<S>),
    NearbyChange(DescriptorNearbyChangeMoveSelector<S>),
    NearbySwap(DescriptorNearbySwapMoveSelector<S>),
    PillarChange(DescriptorPillarChangeMoveSelector<S>),
    PillarSwap(DescriptorPillarSwapMoveSelector<S>),
    RuinRecreate(DescriptorRuinRecreateMoveSelector<S>),
}

impl<S> Debug for DescriptorLeafSelector<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(selector) => selector.fmt(f),
            Self::Swap(selector) => selector.fmt(f),
            Self::NearbyChange(selector) => selector.fmt(f),
            Self::NearbySwap(selector) => selector.fmt(f),
            Self::PillarChange(selector) => selector.fmt(f),
            Self::PillarSwap(selector) => selector.fmt(f),
            Self::RuinRecreate(selector) => selector.fmt(f),
        }
    }
}

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorLeafSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorScalarMoveUnion<S>> + 'a {
        enum DescriptorLeafIter<A, B, C, Dd, E, F, G> {
            Change(A),
            Swap(B),
            NearbyChange(C),
            NearbySwap(Dd),
            PillarChange(E),
            PillarSwap(F),
            RuinRecreate(G),
        }

        impl<T, A, B, C, Dd, E, F, G> Iterator for DescriptorLeafIter<A, B, C, Dd, E, F, G>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
            C: Iterator<Item = T>,
            Dd: Iterator<Item = T>,
            E: Iterator<Item = T>,
            F: Iterator<Item = T>,
            G: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Change(iter) => iter.next(),
                    Self::Swap(iter) => iter.next(),
                    Self::NearbyChange(iter) => iter.next(),
                    Self::NearbySwap(iter) => iter.next(),
                    Self::PillarChange(iter) => iter.next(),
                    Self::PillarSwap(iter) => iter.next(),
                    Self::RuinRecreate(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Change(selector) => {
                DescriptorLeafIter::Change(selector.open_cursor(score_director))
            }
            Self::Swap(selector) => DescriptorLeafIter::Swap(selector.open_cursor(score_director)),
            Self::NearbyChange(selector) => {
                DescriptorLeafIter::NearbyChange(selector.open_cursor(score_director))
            }
            Self::NearbySwap(selector) => {
                DescriptorLeafIter::NearbySwap(selector.open_cursor(score_director))
            }
            Self::PillarChange(selector) => {
                DescriptorLeafIter::PillarChange(selector.open_cursor(score_director))
            }
            Self::PillarSwap(selector) => {
                DescriptorLeafIter::PillarSwap(selector.open_cursor(score_director))
            }
            Self::RuinRecreate(selector) => {
                DescriptorLeafIter::RuinRecreate(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Change(selector) => selector.size(score_director),
            Self::Swap(selector) => selector.size(score_director),
            Self::NearbyChange(selector) => selector.size(score_director),
            Self::NearbySwap(selector) => selector.size(score_director),
            Self::PillarChange(selector) => selector.size(score_director),
            Self::PillarSwap(selector) => selector.size(score_director),
            Self::RuinRecreate(selector) => selector.size(score_director),
        }
    }
}

#[allow(clippy::large_enum_variant)] // Inline storage keeps selector assembly zero-erasure.
pub enum DescriptorSelectorNode<S> {
    Leaf(DescriptorLeafSelector<S>),
    Cartesian(DescriptorCartesianSelector<S>),
}

impl<S> Debug for DescriptorSelectorNode<S>
where
    S: PlanningSolution + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leaf(selector) => selector.fmt(f),
            Self::Cartesian(selector) => selector.fmt(f),
        }
    }
}

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorSelectorNode<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = DescriptorScalarMoveUnion<S>> + 'a {
        enum DescriptorNodeIter<A, B> {
            Leaf(A),
            Cartesian(B),
        }

        impl<T, A, B> Iterator for DescriptorNodeIter<A, B>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Leaf(iter) => iter.next(),
                    Self::Cartesian(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Leaf(selector) => DescriptorNodeIter::Leaf(selector.open_cursor(score_director)),
            Self::Cartesian(selector) => {
                DescriptorNodeIter::Cartesian(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Leaf(selector) => selector.size(score_director),
            Self::Cartesian(selector) => selector.size(score_director),
        }
    }
}

fn wrap_descriptor_composite<S>(
    mov: crate::heuristic::r#move::SequentialCompositeMove<S, DescriptorScalarMoveUnion<S>>,
) -> DescriptorScalarMoveUnion<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    DescriptorScalarMoveUnion::Composite(mov)
}

fn build_descriptor_flat_selector<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
    random_seed: Option<u64>,
) -> DescriptorFlatSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    let bindings = collect_bindings(descriptor);
    let mut leaves = Vec::new();

    fn require_matches<S>(
        label: &str,
        entity_class: Option<&str>,
        variable_name: Option<&str>,
        matched: &[VariableBinding],
    ) where
        S: PlanningSolution + 'static,
        S::Score: Score,
    {
        assert!(
            !matched.is_empty(),
            "{label} selector matched no scalar planning variables for entity_class={:?} variable_name={:?}",
            entity_class,
            variable_name,
        );
    }

    fn collect<S>(
        cfg: &MoveSelectorConfig,
        descriptor: &SolutionDescriptor,
        bindings: &[VariableBinding],
        random_seed: Option<u64>,
        leaves: &mut Vec<DescriptorLeafSelector<S>>,
    ) where
        S: PlanningSolution + 'static,
        S::Score: Score,
    {
        match cfg {
            MoveSelectorConfig::ChangeMoveSelector(change) => {
                let matched = find_binding(
                    bindings,
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "change_move",
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                    &matched,
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
                require_matches::<S>(
                    "swap_move",
                    swap.target.entity_class.as_deref(),
                    swap.target.variable_name.as_deref(),
                    &matched,
                );
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::Swap(
                        DescriptorSwapMoveSelector::new(binding, descriptor.clone()),
                    ));
                }
            }
            MoveSelectorConfig::NearbyChangeMoveSelector(nearby_change) => {
                let matched = find_binding(
                    bindings,
                    nearby_change.target.entity_class.as_deref(),
                    nearby_change.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "nearby_change_move",
                    nearby_change.target.entity_class.as_deref(),
                    nearby_change.target.variable_name.as_deref(),
                    &matched,
                );
                for binding in matched {
                    assert!(
                        binding.nearby_value_distance_meter.is_some(),
                        "nearby_change_move selector requires nearby_value_distance_meter for {}::{}",
                        binding.entity_type_name,
                        binding.variable_name,
                    );
                    leaves.push(DescriptorLeafSelector::NearbyChange(
                        DescriptorNearbyChangeMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            max_nearby: nearby_change.max_nearby,
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::NearbySwapMoveSelector(nearby_swap) => {
                let matched = find_binding(
                    bindings,
                    nearby_swap.target.entity_class.as_deref(),
                    nearby_swap.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "nearby_swap_move",
                    nearby_swap.target.entity_class.as_deref(),
                    nearby_swap.target.variable_name.as_deref(),
                    &matched,
                );
                for binding in matched {
                    assert!(
                        binding.nearby_entity_distance_meter.is_some(),
                        "nearby_swap_move selector requires nearby_entity_distance_meter for {}::{}",
                        binding.entity_type_name,
                        binding.variable_name,
                    );
                    leaves.push(DescriptorLeafSelector::NearbySwap(
                        DescriptorNearbySwapMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            max_nearby: nearby_swap.max_nearby,
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::PillarChangeMoveSelector(pillar_change) => {
                let matched = find_binding(
                    bindings,
                    pillar_change.target.entity_class.as_deref(),
                    pillar_change.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "pillar_change_move",
                    pillar_change.target.entity_class.as_deref(),
                    pillar_change.target.variable_name.as_deref(),
                    &matched,
                );
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::PillarChange(
                        DescriptorPillarChangeMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            minimum_sub_pillar_size: pillar_change.minimum_sub_pillar_size,
                            maximum_sub_pillar_size: pillar_change.maximum_sub_pillar_size,
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::PillarSwapMoveSelector(pillar_swap) => {
                let matched = find_binding(
                    bindings,
                    pillar_swap.target.entity_class.as_deref(),
                    pillar_swap.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "pillar_swap_move",
                    pillar_swap.target.entity_class.as_deref(),
                    pillar_swap.target.variable_name.as_deref(),
                    &matched,
                );
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::PillarSwap(
                        DescriptorPillarSwapMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            minimum_sub_pillar_size: pillar_swap.minimum_sub_pillar_size,
                            maximum_sub_pillar_size: pillar_swap.maximum_sub_pillar_size,
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::RuinRecreateMoveSelector(ruin_recreate) => {
                let matched = find_binding(
                    bindings,
                    ruin_recreate.target.entity_class.as_deref(),
                    ruin_recreate.target.variable_name.as_deref(),
                );
                require_matches::<S>(
                    "ruin_recreate_move",
                    ruin_recreate.target.entity_class.as_deref(),
                    ruin_recreate.target.variable_name.as_deref(),
                    &matched,
                );
                for binding in matched {
                    let rng = match scoped_seed(
                        random_seed,
                        binding.descriptor_index,
                        binding.variable_name,
                        "descriptor_ruin_recreate_move_selector",
                    ) {
                        Some(seed) => SmallRng::seed_from_u64(seed),
                        None => SmallRng::from_rng(&mut rand::rng()),
                    };
                    leaves.push(DescriptorLeafSelector::RuinRecreate(
                        DescriptorRuinRecreateMoveSelector {
                            binding,
                            solution_descriptor: descriptor.clone(),
                            min_ruin_count: ruin_recreate.min_ruin_count.max(1),
                            max_ruin_count: ruin_recreate.max_ruin_count.max(1),
                            moves_per_step: ruin_recreate.moves_per_step.unwrap_or(10).max(1),
                            recreate_heuristic_type: ruin_recreate.recreate_heuristic_type,
                            rng: RefCell::new(rng),
                            _phantom: PhantomData,
                        },
                    ));
                }
            }
            MoveSelectorConfig::UnionMoveSelector(union) => {
                for child in &union.selectors {
                    collect::<S>(child, descriptor, bindings, random_seed, leaves);
                }
            }
            MoveSelectorConfig::LimitedNeighborhood(_) => {
                panic!("limited_neighborhood must be handled by the canonical runtime");
            }
            MoveSelectorConfig::ListChangeMoveSelector(_)
            | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
            | MoveSelectorConfig::ListSwapMoveSelector(_)
            | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
            | MoveSelectorConfig::SublistChangeMoveSelector(_)
            | MoveSelectorConfig::SublistSwapMoveSelector(_)
            | MoveSelectorConfig::ListReverseMoveSelector(_)
            | MoveSelectorConfig::KOptMoveSelector(_)
            | MoveSelectorConfig::ListRuinMoveSelector(_) => {
                panic!("list move selector configured against a scalar-variable context");
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                panic!(
                    "nested cartesian_product move selectors are not supported inside descriptor cartesian children"
                );
            }
        }
    }

    match config {
        Some(cfg) => collect::<S>(cfg, descriptor, &bindings, random_seed, &mut leaves),
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
        "move selector configuration produced no scalar neighborhoods"
    );

    VecUnionSelector::new(leaves)
}

pub fn build_descriptor_move_selector<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
    random_seed: Option<u64>,
) -> DescriptorSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn selector_requires_score_during_move(config: &MoveSelectorConfig) -> bool {
        match config {
            MoveSelectorConfig::RuinRecreateMoveSelector(_) => true,
            MoveSelectorConfig::LimitedNeighborhood(limit) => {
                selector_requires_score_during_move(limit.selector.as_ref())
            }
            MoveSelectorConfig::UnionMoveSelector(union) => union
                .selectors
                .iter()
                .any(selector_requires_score_during_move),
            MoveSelectorConfig::CartesianProductMoveSelector(_) => true,
            _ => false,
        }
    }

    fn assert_cartesian_left_preview_safe(config: &MoveSelectorConfig) {
        assert!(
            !selector_requires_score_during_move(config),
            "cartesian_product left child cannot contain ruin_recreate_move_selector because preview directors do not calculate scores",
        );
    }

    fn collect_nodes<S>(
        config: Option<&MoveSelectorConfig>,
        descriptor: &SolutionDescriptor,
        random_seed: Option<u64>,
        nodes: &mut Vec<DescriptorSelectorNode<S>>,
    ) where
        S: PlanningSolution + 'static,
        S::Score: Score,
    {
        match config {
            Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
                for child in &union.selectors {
                    collect_nodes::<S>(Some(child), descriptor, random_seed, nodes);
                }
            }
            Some(MoveSelectorConfig::CartesianProductMoveSelector(cartesian)) => {
                assert_eq!(
                    cartesian.selectors.len(),
                    2,
                    "cartesian_product move selector requires exactly two child selectors"
                );
                assert_cartesian_left_preview_safe(&cartesian.selectors[0]);
                let left = build_descriptor_flat_selector::<S>(
                    Some(&cartesian.selectors[0]),
                    descriptor,
                    random_seed,
                );
                let right = build_descriptor_flat_selector::<S>(
                    Some(&cartesian.selectors[1]),
                    descriptor,
                    random_seed,
                );
                nodes.push(DescriptorSelectorNode::Cartesian(
                    CartesianProductSelector::new(left, right, wrap_descriptor_composite::<S>),
                ));
            }
            other => {
                let flat = build_descriptor_flat_selector::<S>(other, descriptor, random_seed);
                nodes.extend(
                    flat.into_selectors()
                        .into_iter()
                        .map(DescriptorSelectorNode::Leaf),
                );
            }
        }
    }

    let mut nodes = Vec::new();
    collect_nodes::<S>(config, descriptor, random_seed, &mut nodes);
    assert!(
        !nodes.is_empty(),
        "move selector configuration produced no scalar neighborhoods"
    );
    VecUnionSelector::new(nodes)
}
