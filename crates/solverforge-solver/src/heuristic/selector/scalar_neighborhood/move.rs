mod apply;
mod metadata;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::RuntimeScalarSlot;
use crate::heuristic::r#move::{Move, MoveAffectedEntity, MoveTabuSignature};
use crate::stats::{CandidateTraceCoordinate, CandidateTraceIdentity};

use super::spec::RuntimeScalarRecipe;

/// Owned scalar move from the one typed/dynamic scalar leaf kernel.
#[derive(Clone)]
pub(crate) struct RuntimeScalarMove<S> {
    recipe: RuntimeScalarRecipe<S>,
    entity_indices: Vec<usize>,
}

impl<S> std::fmt::Debug for RuntimeScalarMove<S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeScalarMove")
            .field("family", &recipe_family(&self.recipe))
            .field("slot", &self.slot().id())
            .field("entity_indices", &self.entity_indices)
            .finish()
    }
}

impl<S> RuntimeScalarMove<S> {
    pub(crate) fn from_recipe(recipe: RuntimeScalarRecipe<S>) -> Self {
        // Candidate ordering is part of the selector contract.  In
        // particular, ruin/recreate uses the sampled permutation order for
        // both recreation and its public move view.  Keep the recipe's order
        // intact here; canonicalization belongs only in metadata that
        // explicitly models an unordered relationship (for example a swap
        // tabu key).
        let entity_indices = recipe_entity_indices(&recipe);
        Self {
            recipe,
            entity_indices,
        }
    }

    pub(crate) fn into_recipe(self) -> RuntimeScalarRecipe<S> {
        self.recipe
    }

    pub(crate) fn slot(&self) -> &RuntimeScalarSlot<S> {
        recipe_slot(&self.recipe)
    }
}

/// Undo state remains entirely owned by the selected candidate.
#[derive(Clone, Debug)]
pub(crate) enum RuntimeScalarMoveUndo {
    Change(Option<usize>),
    Swap(Option<usize>, Option<usize>),
    Many(Vec<(usize, Option<usize>)>),
}

impl<S> Move<S> for RuntimeScalarMove<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    type Undo = RuntimeScalarMoveUndo;

    fn is_doable<D: Director<S>>(&self, director: &D) -> bool {
        apply::is_doable(&self.recipe, director.working_solution())
    }

    fn do_move<D: Director<S>>(&self, director: &mut D) -> Self::Undo {
        apply::do_move(&self.recipe, director)
    }

    fn undo_move<D: Director<S>>(&self, director: &mut D, undo: Self::Undo) {
        apply::undo_move(&self.recipe, director, undo)
    }

    fn descriptor_index(&self) -> usize {
        self.slot().descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.slot().variable_name()
    }

    fn telemetry_label(&self) -> &'static str {
        match self.recipe {
            RuntimeScalarRecipe::Change { .. } => "runtime_scalar_change",
            RuntimeScalarRecipe::Swap { .. } => "runtime_scalar_swap",
            RuntimeScalarRecipe::PillarChange { .. } => "runtime_scalar_pillar_change",
            RuntimeScalarRecipe::PillarSwap { .. } => "runtime_scalar_pillar_swap",
            RuntimeScalarRecipe::RuinRecreate { .. } => "runtime_scalar_ruin_recreate",
        }
    }

    fn tabu_signature<D: Director<S>>(&self, director: &D) -> MoveTabuSignature {
        metadata::tabu_signature(&self.recipe, director)
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        let slot = self.slot();
        let identity = match &self.recipe {
            RuntimeScalarRecipe::Change {
                entity_index,
                to_value,
                ..
            } => CandidateTraceIdentity::logical_move(
                slot.descriptor_index(),
                slot.variable_name(),
                "scalar_change",
                [
                    CandidateTraceCoordinate::from(*entity_index),
                    CandidateTraceCoordinate::from(*to_value),
                ],
            ),
            RuntimeScalarRecipe::Swap {
                left_entity_index,
                right_entity_index,
                ..
            } => CandidateTraceIdentity::logical_move(
                slot.descriptor_index(),
                slot.variable_name(),
                "scalar_swap",
                [*left_entity_index, *right_entity_index],
            ),
            RuntimeScalarRecipe::PillarChange {
                entity_indices,
                to_value,
                ..
            } => CandidateTraceIdentity::logical_move(
                slot.descriptor_index(),
                slot.variable_name(),
                "scalar_pillar_change",
                entity_indices
                    .iter()
                    .copied()
                    .map(CandidateTraceCoordinate::from)
                    .chain(std::iter::once(CandidateTraceCoordinate::from(*to_value))),
            ),
            RuntimeScalarRecipe::PillarSwap {
                left_indices,
                right_indices,
                ..
            } => CandidateTraceIdentity::logical_move(
                slot.descriptor_index(),
                slot.variable_name(),
                "scalar_pillar_swap",
                left_indices
                    .iter()
                    .copied()
                    .chain(std::iter::once(usize::MAX))
                    .chain(right_indices.iter().copied())
                    .map(CandidateTraceCoordinate::from),
            ),
            RuntimeScalarRecipe::RuinRecreate { entity_indices, .. } => {
                CandidateTraceIdentity::logical_move(
                    slot.descriptor_index(),
                    slot.variable_name(),
                    "scalar_ruin_recreate",
                    entity_indices
                        .iter()
                        .copied()
                        .map(CandidateTraceCoordinate::from),
                )
            }
        };
        Some(identity)
    }

    fn for_each_affected_entity(&self, visitor: &mut dyn FnMut(MoveAffectedEntity<'_>)) {
        let descriptor_index = self.descriptor_index();
        let variable_name = self.variable_name();
        for &entity_index in &self.entity_indices {
            visitor(MoveAffectedEntity {
                descriptor_index,
                entity_index,
                variable_name,
            });
        }
    }
}

pub(super) fn recipe_slot<S>(recipe: &RuntimeScalarRecipe<S>) -> &RuntimeScalarSlot<S> {
    match recipe {
        RuntimeScalarRecipe::Change { slot, .. }
        | RuntimeScalarRecipe::Swap { slot, .. }
        | RuntimeScalarRecipe::PillarChange { slot, .. }
        | RuntimeScalarRecipe::PillarSwap { slot, .. }
        | RuntimeScalarRecipe::RuinRecreate { slot, .. } => slot,
    }
}

fn recipe_family<S>(recipe: &RuntimeScalarRecipe<S>) -> &'static str {
    match recipe {
        RuntimeScalarRecipe::Change { .. } => "change",
        RuntimeScalarRecipe::Swap { .. } => "swap",
        RuntimeScalarRecipe::PillarChange { .. } => "pillar_change",
        RuntimeScalarRecipe::PillarSwap { .. } => "pillar_swap",
        RuntimeScalarRecipe::RuinRecreate { .. } => "ruin_recreate",
    }
}

fn recipe_entity_indices<S>(recipe: &RuntimeScalarRecipe<S>) -> Vec<usize> {
    match recipe {
        RuntimeScalarRecipe::Change { entity_index, .. } => vec![*entity_index],
        RuntimeScalarRecipe::Swap {
            left_entity_index,
            right_entity_index,
            ..
        } => vec![*left_entity_index, *right_entity_index],
        RuntimeScalarRecipe::PillarChange { entity_indices, .. }
        | RuntimeScalarRecipe::RuinRecreate { entity_indices, .. } => entity_indices.clone(),
        RuntimeScalarRecipe::PillarSwap {
            left_indices,
            right_indices,
            ..
        } => left_indices.iter().chain(right_indices).copied().collect(),
    }
}
