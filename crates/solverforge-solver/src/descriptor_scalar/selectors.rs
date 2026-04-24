use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use smallvec::SmallVec;
use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor, ValueRangeType};
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::{
    CartesianProductCursor, CartesianProductSelector, VecUnionSelector,
};
use crate::heuristic::selector::entity::EntityReference;
use crate::heuristic::selector::move_selector::{
    ArenaMoveCursor, MoveCandidateRef, MoveCursor, MoveSelector,
};
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

const SWAP_LEGALITY_WORD_BITS: usize = usize::BITS as usize;

fn validate_ruin_recreate_bounds(min_ruin_count: usize, max_ruin_count: usize) {
    assert!(
        min_ruin_count >= 1,
        "descriptor ruin_recreate_move_selector requires min_ruin_count >= 1"
    );
    assert!(
        max_ruin_count >= min_ruin_count,
        "descriptor ruin_recreate_move_selector requires max_ruin_count >= min_ruin_count"
    );
}

enum SwapLegalityDomain {
    Unspecified,
    Empty,
    CountableRange {
        from: i64,
        to: i64,
    },
    SolutionCount {
        count: usize,
    },
    EntityCurrentValueBits {
        current_value_ids: Vec<usize>,
        accepted_value_words: Vec<Vec<usize>>,
    },
}

struct SwapLegalityIndex {
    current_values: Vec<Option<usize>>,
    allows_unassigned: bool,
    domain: SwapLegalityDomain,
}

impl SwapLegalityIndex {
    fn new(
        binding: &VariableBinding,
        descriptor: &SolutionDescriptor,
        solution: &dyn Any,
        count: usize,
        lookup_context: &str,
    ) -> Self {
        let current_values = (0..count)
            .map(|entity_index| {
                let entity = descriptor
                    .get_entity(solution, binding.descriptor_index, entity_index)
                    .expect(lookup_context);
                (binding.getter)(entity)
            })
            .collect::<Vec<_>>();

        let domain = match (&binding.provider, &binding.range_type) {
            (Some(_), _) => Self::build_entity_domain(
                binding,
                descriptor,
                solution,
                lookup_context,
                &current_values,
            ),
            (_, ValueRangeType::CountableRange { from, to }) => {
                SwapLegalityDomain::CountableRange {
                    from: *from,
                    to: *to,
                }
            }
            _ if binding.has_unspecified_value_range() => SwapLegalityDomain::Unspecified,
            _ => binding
                .solution_value_count(descriptor, solution)
                .map(|count| SwapLegalityDomain::SolutionCount { count })
                .unwrap_or(SwapLegalityDomain::Empty),
        };

        Self {
            current_values,
            allows_unassigned: binding.allows_unassigned,
            domain,
        }
    }

    fn build_entity_domain(
        binding: &VariableBinding,
        descriptor: &SolutionDescriptor,
        solution: &dyn Any,
        lookup_context: &str,
        current_values: &[Option<usize>],
    ) -> SwapLegalityDomain {
        let mut assigned_value_ids = HashMap::new();
        let mut unassigned_value_id = None;
        let mut current_value_ids = Vec::with_capacity(current_values.len());
        for current_value in current_values {
            let value_id = match current_value {
                Some(value) => match assigned_value_ids.get(value) {
                    Some(value_id) => *value_id,
                    None => {
                        let value_id =
                            assigned_value_ids.len() + usize::from(unassigned_value_id.is_some());
                        assigned_value_ids.insert(*value, value_id);
                        value_id
                    }
                },
                None => match unassigned_value_id {
                    Some(value_id) => value_id,
                    None => {
                        let value_id = assigned_value_ids.len();
                        unassigned_value_id = Some(value_id);
                        value_id
                    }
                },
            };
            current_value_ids.push(value_id);
        }

        let value_count = assigned_value_ids.len() + usize::from(unassigned_value_id.is_some());
        let word_count =
            value_count.saturating_add(SWAP_LEGALITY_WORD_BITS - 1) / SWAP_LEGALITY_WORD_BITS;
        let mut accepted_value_words = Vec::with_capacity(current_values.len());
        for entity_index in 0..current_values.len() {
            let entity = descriptor
                .get_entity(solution, binding.descriptor_index, entity_index)
                .expect(lookup_context);
            let mut words = vec![0usize; word_count];
            if binding.allows_unassigned {
                if let Some(value_id) = unassigned_value_id {
                    Self::set_bit(&mut words, value_id);
                }
            }
            for allowed_value in binding.values_for_entity(descriptor, solution, entity) {
                if let Some(value_id) = assigned_value_ids.get(&allowed_value) {
                    Self::set_bit(&mut words, *value_id);
                }
            }
            accepted_value_words.push(words);
        }

        SwapLegalityDomain::EntityCurrentValueBits {
            current_value_ids,
            accepted_value_words,
        }
    }

    fn set_bit(words: &mut [usize], value_id: usize) {
        words[value_id / SWAP_LEGALITY_WORD_BITS] |= 1usize << (value_id % SWAP_LEGALITY_WORD_BITS);
    }

    fn has_bit(words: &[usize], value_id: usize) -> bool {
        words[value_id / SWAP_LEGALITY_WORD_BITS] & (1usize << (value_id % SWAP_LEGALITY_WORD_BITS))
            != 0
    }

    fn accepts_value_from_entity(&self, entity_index: usize, value_entity_index: usize) -> bool {
        let candidate = self.current_values[value_entity_index];
        match &self.domain {
            SwapLegalityDomain::Unspecified => candidate.is_some(),
            SwapLegalityDomain::Empty => candidate.is_none() && self.allows_unassigned,
            SwapLegalityDomain::CountableRange { from, to } => candidate
                .map_or(self.allows_unassigned, |value| {
                    VariableBinding::countable_range_contains(*from, *to, value)
                }),
            SwapLegalityDomain::SolutionCount { count } => {
                candidate.map_or(self.allows_unassigned, |value| value < *count)
            }
            SwapLegalityDomain::EntityCurrentValueBits {
                current_value_ids,
                accepted_value_words,
            } => Self::has_bit(
                &accepted_value_words[entity_index],
                current_value_ids[value_entity_index],
            ),
        }
    }

    fn can_swap(&self, left_entity_index: usize, right_entity_index: usize) -> bool {
        if left_entity_index == right_entity_index
            || self.current_values[left_entity_index] == self.current_values[right_entity_index]
        {
            return false;
        }

        self.accepts_value_from_entity(left_entity_index, right_entity_index)
            && self.accepts_value_from_entity(right_entity_index, left_entity_index)
    }

    fn values_for_swap(
        &self,
        left_entity_index: usize,
        right_entity_index: usize,
    ) -> Option<(Option<usize>, Option<usize>)> {
        self.can_swap(left_entity_index, right_entity_index)
            .then(|| {
                (
                    self.current_values[left_entity_index],
                    self.current_values[right_entity_index],
                )
            })
    }

    fn count_legal_pairs(&self) -> usize {
        let mut total = 0;
        for left_entity_index in 0..self.current_values.len() {
            for right_entity_index in (left_entity_index + 1)..self.current_values.len() {
                if self.can_swap(left_entity_index, right_entity_index) {
                    total += 1;
                }
            }
        }
        total
    }
}

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
    type Cursor<'a>
        = ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
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
        ArenaMoveCursor::from_moves(moves)
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
    type Cursor<'a>
        = ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let binding = self.binding.clone();
        let descriptor = self.solution_descriptor.clone();
        let solution = score_director.working_solution() as &dyn Any;
        let legality_index = SwapLegalityIndex::new(
            &binding,
            &descriptor,
            solution,
            count,
            "entity lookup failed for swap selector",
        );

        let mut moves = Vec::new();
        for left_entity_index in 0..count {
            for right_entity_index in (left_entity_index + 1)..count {
                if let Some((left_value, right_value)) =
                    legality_index.values_for_swap(left_entity_index, right_entity_index)
                {
                    moves.push(DescriptorScalarMoveUnion::Swap(
                        DescriptorSwapMove::new_validated(
                            binding.clone(),
                            left_entity_index,
                            left_value,
                            right_entity_index,
                            right_value,
                            descriptor.clone(),
                        ),
                    ));
                }
            }
        }
        ArenaMoveCursor::from_moves(moves)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let solution = score_director.working_solution() as &dyn Any;
        let legality_index = SwapLegalityIndex::new(
            &self.binding,
            &self.solution_descriptor,
            solution,
            count,
            "entity lookup failed for swap selector",
        );
        legality_index.count_legal_pairs()
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
    type Cursor<'a>
        = ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
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
        ArenaMoveCursor::from_moves(moves)
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
    type Cursor<'a>
        = ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
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
        let legality_index = SwapLegalityIndex::new(
            &binding,
            &descriptor,
            solution,
            count,
            "entity lookup failed for nearby swap selector",
        );
        let mut moves = Vec::new();
        for left_entity_index in 0..count {
            let mut candidates: Vec<(usize, f64, usize)> = ((left_entity_index + 1)..count)
                .enumerate()
                .filter_map(|(order, right_entity_index)| {
                    if !legality_index.can_swap(left_entity_index, right_entity_index) {
                        return None;
                    }
                    let distance = distance_meter(solution, left_entity_index, right_entity_index);
                    distance
                        .is_finite()
                        .then_some((right_entity_index, distance, order))
                })
                .collect();
            truncate_nearby_candidates(&mut candidates, max_nearby);
            for (right_entity_index, _, _) in candidates {
                let Some((left_value, right_value)) =
                    legality_index.values_for_swap(left_entity_index, right_entity_index)
                else {
                    continue;
                };
                moves.push(DescriptorScalarMoveUnion::Swap(
                    DescriptorSwapMove::new_validated(
                        binding.clone(),
                        left_entity_index,
                        left_value,
                        right_entity_index,
                        right_value,
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
    type Cursor<'a>
        = ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        match self {
            Self::Change(selector) => {
                ArenaMoveCursor::from_moves(selector.iter_moves(score_director))
            }
            Self::Swap(selector) => {
                ArenaMoveCursor::from_moves(selector.iter_moves(score_director))
            }
            Self::NearbyChange(selector) => {
                ArenaMoveCursor::from_moves(selector.iter_moves(score_director))
            }
            Self::NearbySwap(selector) => {
                ArenaMoveCursor::from_moves(selector.iter_moves(score_director))
            }
            Self::PillarChange(selector) => {
                ArenaMoveCursor::from_moves(selector.iter_moves(score_director))
            }
            Self::PillarSwap(selector) => {
                ArenaMoveCursor::from_moves(selector.iter_moves(score_director))
            }
            Self::RuinRecreate(selector) => {
                ArenaMoveCursor::from_moves(selector.iter_moves(score_director))
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

pub enum DescriptorSelectorCursor<S>
where
    S: PlanningSolution + 'static,
{
    Leaf(ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>),
    Cartesian(CartesianProductCursor<S, DescriptorScalarMoveUnion<S>>),
}

impl<S> MoveCursor<S, DescriptorScalarMoveUnion<S>> for DescriptorSelectorCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(
        &mut self,
    ) -> Option<(usize, MoveCandidateRef<'_, S, DescriptorScalarMoveUnion<S>>)> {
        match self {
            Self::Leaf(cursor) => cursor.next_candidate(),
            Self::Cartesian(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(
        &self,
        index: usize,
    ) -> Option<MoveCandidateRef<'_, S, DescriptorScalarMoveUnion<S>>> {
        match self {
            Self::Leaf(cursor) => cursor.candidate(index),
            Self::Cartesian(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: usize) -> DescriptorScalarMoveUnion<S> {
        match self {
            Self::Leaf(cursor) => cursor.take_candidate(index),
            Self::Cartesian(cursor) => cursor.take_candidate(index),
        }
    }
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
    type Cursor<'a>
        = DescriptorSelectorCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        match self {
            Self::Leaf(selector) => {
                DescriptorSelectorCursor::Leaf(selector.open_cursor(score_director))
            }
            Self::Cartesian(selector) => {
                DescriptorSelectorCursor::Cartesian(selector.open_cursor(score_director))
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
                validate_ruin_recreate_bounds(
                    ruin_recreate.min_ruin_count,
                    ruin_recreate.max_ruin_count,
                );
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
                            min_ruin_count: ruin_recreate.min_ruin_count,
                            max_ruin_count: ruin_recreate.max_ruin_count,
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
