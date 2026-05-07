use std::any::Any;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_config::ConstructionHeuristicType;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::selector::EntityReference;
use crate::phase::construction::{
    BestFitForager, ConstructionHeuristicPhase, EntityPlacer, FirstFitForager, Placement,
    StrongestFitForager, WeakestFitForager,
};
use crate::scope::{ProgressCallback, SolverScope};

use super::super::bindings::ResolvedVariableBinding;
use super::super::move_types::{DescriptorChangeMove, DescriptorMoveUnion};

pub enum DescriptorConstruction<S: PlanningSolution> {
    FirstFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorMoveUnion<S>,
            DescriptorEntityPlacer<S>,
            FirstFitForager<S, DescriptorMoveUnion<S>>,
        >,
    ),
    BestFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorMoveUnion<S>,
            DescriptorEntityPlacer<S>,
            BestFitForager<S, DescriptorMoveUnion<S>>,
        >,
    ),
    WeakestFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorMoveUnion<S>,
            DescriptorEntityPlacer<S>,
            WeakestFitForager<S, DescriptorMoveUnion<S>>,
        >,
    ),
    StrongestFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorMoveUnion<S>,
            DescriptorEntityPlacer<S>,
            StrongestFitForager<S, DescriptorMoveUnion<S>>,
        >,
    ),
}

impl<S: PlanningSolution> Debug for DescriptorConstruction<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FirstFit(phase) => write!(f, "DescriptorConstruction::FirstFit({phase:?})"),
            Self::BestFit(phase) => write!(f, "DescriptorConstruction::BestFit({phase:?})"),
            Self::WeakestFit(phase) => {
                write!(f, "DescriptorConstruction::WeakestFit({phase:?})")
            }
            Self::StrongestFit(phase) => {
                write!(f, "DescriptorConstruction::StrongestFit({phase:?})")
            }
        }
    }
}

impl<S, D, ProgressCb> crate::phase::Phase<S, D, ProgressCb> for DescriptorConstruction<S>
where
    S: PlanningSolution + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        match self {
            Self::FirstFit(phase) => phase.solve(solver_scope),
            Self::BestFit(phase) => phase.solve(solver_scope),
            Self::WeakestFit(phase) => phase.solve(solver_scope),
            Self::StrongestFit(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "DescriptorConstruction"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntityOrder {
    Canonical,
    AscendingKey,
    DescendingKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueOrder {
    Canonical,
    AscendingKey,
}

#[derive(Clone)]
pub struct DescriptorEntityPlacer<S> {
    bindings: Vec<ResolvedVariableBinding<S>>,
    solution_descriptor: SolutionDescriptor,
    entity_order: EntityOrder,
    value_order: ValueOrder,
    value_candidate_limit: Option<usize>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorEntityPlacer<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorEntityPlacer")
            .field("bindings", &self.bindings)
            .field("entity_order", &self.entity_order)
            .field("value_order", &self.value_order)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .finish()
    }
}

impl<S> DescriptorEntityPlacer<S>
where
    S: PlanningSolution + 'static,
{
    pub(super) fn new(
        bindings: Vec<ResolvedVariableBinding<S>>,
        solution_descriptor: SolutionDescriptor,
        entity_order: EntityOrder,
        value_order: ValueOrder,
        value_candidate_limit: Option<usize>,
    ) -> Self {
        Self {
            bindings,
            solution_descriptor,
            entity_order,
            value_order,
            value_candidate_limit,
            _phantom: PhantomData,
        }
    }

    fn entity_order_key(
        &self,
        binding: &ResolvedVariableBinding<S>,
        solution: &S,
        entity_index: usize,
    ) -> i64 {
        binding.entity_order_key(solution, entity_index).unwrap_or_else(|| {
            unreachable!(
                "validated descriptor-driven construction must provide construction_entity_order_key for {}.{}",
                binding.entity_type_name,
                binding.variable_name
            )
        })
    }

    fn value_order_key(
        &self,
        binding: &ResolvedVariableBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> i64 {
        binding
            .value_order_key(solution, entity_index, value)
            .unwrap_or_else(|| {
                unreachable!(
                    "validated descriptor-driven construction must provide construction_value_order_key for {}.{}",
                    binding.entity_type_name,
                    binding.variable_name
                )
            })
    }

    fn ordered_entity_indices<D: Director<S>>(
        &self,
        binding: &ResolvedVariableBinding<S>,
        score_director: &D,
    ) -> Vec<usize> {
        let count = score_director
            .entity_count(binding.descriptor_index)
            .unwrap_or(0);
        let mut entity_indices: Vec<_> = (0..count).collect();
        if self.entity_order != EntityOrder::Canonical {
            let solution = score_director.working_solution();
            entity_indices.sort_by(|left, right| {
                let left_key = self.entity_order_key(binding, solution, *left);
                let right_key = self.entity_order_key(binding, solution, *right);
                match self.entity_order {
                    EntityOrder::Canonical => left.cmp(right),
                    EntityOrder::AscendingKey => left_key.cmp(&right_key).then(left.cmp(right)),
                    EntityOrder::DescendingKey => right_key.cmp(&left_key).then(left.cmp(right)),
                }
            });
        }
        entity_indices
    }

    fn ordered_candidate_values(
        &self,
        binding: &ResolvedVariableBinding<S>,
        solution: &S,
        entity_index: usize,
    ) -> Vec<usize> {
        let mut values: Vec<_> = binding
            .candidate_values_for_entity_index(
                &self.solution_descriptor,
                solution as &dyn Any,
                entity_index,
                self.value_candidate_limit,
            )
            .into_iter()
            .enumerate()
            .collect();
        if self.value_order != ValueOrder::Canonical {
            values.sort_by(|(left_order, left_value), (right_order, right_value)| {
                let left_key = self.value_order_key(binding, solution, entity_index, *left_value);
                let right_key = self.value_order_key(binding, solution, entity_index, *right_value);
                match self.value_order {
                    ValueOrder::Canonical => left_order.cmp(right_order),
                    ValueOrder::AscendingKey => {
                        left_key.cmp(&right_key).then(left_order.cmp(right_order))
                    }
                }
            });
        }
        values.into_iter().map(|(_, value)| value).collect()
    }

    fn descriptor_change_move(
        &self,
        binding: &ResolvedVariableBinding<S>,
        entity_index: usize,
        value: usize,
    ) -> DescriptorMoveUnion<S> {
        let mut mov = DescriptorChangeMove::new(
            binding.clone_binding(),
            entity_index,
            Some(value),
            self.solution_descriptor.clone(),
        );
        if let Some(order_key) = binding.runtime_value_order_key() {
            mov = mov.with_construction_value_order_key(order_key);
        }
        DescriptorMoveUnion::Change(mov)
    }

    fn placement_for_entity(
        &self,
        binding: &ResolvedVariableBinding<S>,
        solution: &S,
        entity_index: usize,
    ) -> Option<Placement<S, DescriptorMoveUnion<S>>> {
        let moves = self
            .ordered_candidate_values(binding, solution, entity_index)
            .into_iter()
            .map(|value| self.descriptor_change_move(binding, entity_index, value))
            .collect::<Vec<_>>();

        if moves.is_empty() {
            return None;
        }

        Some(
            Placement::new(
                EntityReference::new(binding.descriptor_index, entity_index),
                moves,
            )
            .with_slot_id(binding.slot_id(entity_index))
            .with_keep_current_legal(binding.allows_unassigned),
        )
    }
}

impl<S> EntityPlacer<S, DescriptorMoveUnion<S>> for DescriptorEntityPlacer<S>
where
    S: PlanningSolution + 'static,
{
    fn get_placements<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> Vec<Placement<S, DescriptorMoveUnion<S>>> {
        let mut placements = Vec::new();
        let solution = score_director.working_solution();
        let erased_solution = solution as &dyn Any;

        for binding in &self.bindings {
            for entity_index in self.ordered_entity_indices(binding, score_director) {
                let entity = self
                    .solution_descriptor
                    .get_entity(erased_solution, binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for descriptor construction");
                if (binding.getter)(entity).is_some() {
                    continue;
                }

                if let Some(placement) = self.placement_for_entity(binding, solution, entity_index)
                {
                    placements.push(placement);
                }
            }
        }

        placements
    }

    fn get_next_placement<D, IsCompleted>(
        &self,
        score_director: &D,
        mut is_completed: IsCompleted,
    ) -> Option<(Placement<S, DescriptorMoveUnion<S>>, u64)>
    where
        D: Director<S>,
        IsCompleted: FnMut(usize, usize) -> bool,
    {
        let solution = score_director.working_solution();
        let erased_solution = solution as &dyn Any;

        for binding in &self.bindings {
            for entity_index in self.ordered_entity_indices(binding, score_director) {
                let slot_id = binding.slot_id(entity_index);
                if is_completed(slot_id.binding_index(), slot_id.entity_index()) {
                    continue;
                }

                let entity = self
                    .solution_descriptor
                    .get_entity(erased_solution, binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for descriptor construction");
                if (binding.getter)(entity).is_some() {
                    continue;
                }

                if let Some(placement) = self.placement_for_entity(binding, solution, entity_index)
                {
                    let generated_moves = u64::try_from(placement.moves.len()).unwrap_or(u64::MAX);
                    return Some((placement, generated_moves));
                }
            }
        }

        None
    }
}

pub(super) fn descriptor_move_strength<S>(mov: &DescriptorMoveUnion<S>, solution: &S) -> i64
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    match mov {
        DescriptorMoveUnion::Change(mov) => mov.live_value_order_key(solution).unwrap_or(0),
        _ => 0,
    }
}

pub(super) fn entity_order_for_heuristic(heuristic: ConstructionHeuristicType) -> EntityOrder {
    match heuristic {
        ConstructionHeuristicType::FirstFitDecreasing
        | ConstructionHeuristicType::WeakestFitDecreasing
        | ConstructionHeuristicType::StrongestFitDecreasing => EntityOrder::DescendingKey,
        ConstructionHeuristicType::AllocateEntityFromQueue => EntityOrder::AscendingKey,
        _ => EntityOrder::Canonical,
    }
}

pub(super) fn value_order_for_heuristic(heuristic: ConstructionHeuristicType) -> ValueOrder {
    match heuristic {
        ConstructionHeuristicType::AllocateToValueFromQueue => ValueOrder::AscendingKey,
        _ => ValueOrder::Canonical,
    }
}

pub(super) fn heuristic_requires_live_refresh(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::FirstFitDecreasing
            | ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing
            | ConstructionHeuristicType::AllocateEntityFromQueue
            | ConstructionHeuristicType::AllocateToValueFromQueue
    )
}
