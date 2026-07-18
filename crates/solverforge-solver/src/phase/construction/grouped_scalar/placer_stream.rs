use std::collections::HashSet;

use solverforge_config::{ConstructionHeuristicType, ConstructionObligation};
use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_stream::ScalarAssignmentMoveCursor;
use super::move_build::compound_move_for_prevalidated_group_candidate;
use super::placement::{assignment_group_slot, assignment_move_target, principal_assignment_edit};
use crate::builder::context::{ScalarAssignmentBinding, ScalarCandidate, ScalarGroupBinding};
use crate::heuristic::r#move::CompoundScalarMove;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor,
};
use crate::heuristic::selector::EntityReference;
use crate::phase::construction::{
    ConstructionGroupSlotId, ConstructionSlotId, ConstructionTarget, Placement,
};

pub(super) struct CandidatePlacementSeed<S>
where
    S: PlanningSolution + 'static,
{
    pub(super) sequence: usize,
    pub(super) entity_ref: EntityReference,
    pub(super) candidates: Vec<ScalarCandidate<S>>,
    pub(super) group_slot: ConstructionGroupSlotId,
    pub(super) scalar_slots: Vec<ConstructionSlotId>,
    pub(super) keep_current_legal: bool,
    pub(super) entity_order_key: Option<i64>,
}

pub(crate) struct CandidatePlacementGenerator<S>
where
    S: PlanningSolution + 'static,
{
    pub(super) placements: std::vec::IntoIter<CandidatePlacementSeed<S>>,
    pub(super) group: ScalarGroupBinding<S>,
}

pub(crate) struct AssignmentPlacementGenerator<S>
where
    S: PlanningSolution + 'static,
{
    pub(super) group_index: usize,
    pub(super) assignment: ScalarAssignmentBinding<S>,
    pub(super) cursor: ScalarAssignmentMoveCursor<S>,
    pub(super) pending: Option<CompoundScalarMove<S>>,
    pub(super) options: ScalarAssignmentMoveOptions,
    pub(super) accepted: usize,
}

#[allow(clippy::large_enum_variant)]
enum ScalarGroupCandidateSource<S>
where
    S: PlanningSolution + 'static,
{
    Empty,
    Candidates {
        group: ScalarGroupBinding<S>,
        candidates: std::vec::IntoIter<ScalarCandidate<S>>,
    },
    Assignment {
        generator: AssignmentPlacementGenerator<S>,
        active_entity: usize,
        completed_slots: HashSet<ConstructionSlotId>,
        first: Option<(CompoundScalarMove<S>, ConstructionTarget)>,
    },
}

pub(crate) struct ScalarGroupCandidateCursor<S>
where
    S: PlanningSolution + 'static,
{
    store: CandidateStore<S, CompoundScalarMove<S>>,
    targets: Vec<ConstructionTarget>,
    source: ScalarGroupCandidateSource<S>,
}

impl<S> ScalarGroupCandidateCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn empty() -> Self {
        Self {
            store: CandidateStore::new(),
            targets: Vec::new(),
            source: ScalarGroupCandidateSource::Empty,
        }
    }

    fn candidates(group: ScalarGroupBinding<S>, candidates: Vec<ScalarCandidate<S>>) -> Self {
        Self {
            store: CandidateStore::with_capacity(candidates.len()),
            targets: Vec::new(),
            source: ScalarGroupCandidateSource::Candidates {
                group,
                candidates: candidates.into_iter(),
            },
        }
    }

    fn assignment(
        generator: AssignmentPlacementGenerator<S>,
        active_entity: usize,
        completed_slots: HashSet<ConstructionSlotId>,
        first: (CompoundScalarMove<S>, ConstructionTarget),
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            targets: Vec::new(),
            source: ScalarGroupCandidateSource::Assignment {
                generator,
                active_entity,
                completed_slots,
                first: Some(first),
            },
        }
    }

    pub(super) fn construction_target(
        &self,
        candidate_id: CandidateId,
    ) -> Option<&ConstructionTarget> {
        self.targets.get(candidate_id.index())
    }
}

impl<S> MoveCursor<S, CompoundScalarMove<S>> for ScalarGroupCandidateCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.next_candidate_with_control(&mut || false)
    }

    fn next_candidate_with_control<ShouldStop>(
        &mut self,
        should_stop: &mut ShouldStop,
    ) -> Option<CandidateId>
    where
        ShouldStop: FnMut() -> bool,
    {
        if should_stop() {
            return None;
        }
        let (mov, target) = match &mut self.source {
            ScalarGroupCandidateSource::Empty => return None,
            ScalarGroupCandidateSource::Candidates { group, candidates } => {
                let candidate = candidates.next()?;
                let order_key = candidate.construction_value_order_key();
                let mov = compound_move_for_prevalidated_group_candidate(group, &candidate)
                    .with_construction_value_order_key(order_key);
                (mov, None)
            }
            ScalarGroupCandidateSource::Assignment {
                generator,
                active_entity,
                completed_slots,
                first,
            } => {
                let (mov, target) = if let Some(first) = first.take() {
                    first
                } else {
                    let (entity_index, mov, target) =
                        next_assignment_candidate(generator, completed_slots, should_stop)?;
                    if entity_index != *active_entity {
                        generator.pending = Some(mov);
                        return None;
                    }
                    (mov, target)
                };
                generator.accepted += 1;
                (mov, Some(target))
            }
        };

        let candidate_id = self.store.push(mov);
        if let Some(target) = target {
            debug_assert_eq!(self.targets.len(), candidate_id.index());
            self.targets.push(target);
        }
        Some(candidate_id)
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, CompoundScalarMove<S>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> CompoundScalarMove<S> {
        self.store.take_candidate(id)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

pub(super) fn next_candidate_placement<S, IsCompleted, ShouldStop>(
    generator: &mut CandidatePlacementGenerator<S>,
    is_completed: &mut IsCompleted,
    should_stop: &mut ShouldStop,
) -> Option<Placement<S, CompoundScalarMove<S>, ScalarGroupCandidateCursor<S>>>
where
    S: PlanningSolution + 'static,
    IsCompleted: FnMut(&Placement<S, CompoundScalarMove<S>, ScalarGroupCandidateCursor<S>>) -> bool,
    ShouldStop: FnMut() -> bool,
{
    while !should_stop() {
        let seed = generator.placements.next()?;
        let placement = Placement::new(
            seed.entity_ref,
            ScalarGroupCandidateCursor::candidates(generator.group.clone(), seed.candidates),
        )
        .with_group_slot(seed.group_slot)
        .with_scalar_slots(seed.scalar_slots)
        .with_keep_current_legal(seed.keep_current_legal);
        if !is_completed(&placement) {
            return Some(placement);
        }
    }
    None
}

pub(super) fn next_assignment_placement<S, IsCompleted, ShouldStop>(
    mut generator: AssignmentPlacementGenerator<S>,
    mut is_completed: IsCompleted,
    mut should_stop: ShouldStop,
) -> Option<Placement<S, CompoundScalarMove<S>, ScalarGroupCandidateCursor<S>>>
where
    S: PlanningSolution + 'static,
    IsCompleted: FnMut(&Placement<S, CompoundScalarMove<S>, ScalarGroupCandidateCursor<S>>) -> bool,
    ShouldStop: FnMut() -> bool,
{
    let completed_slots = completed_assignment_slots(&generator, &mut is_completed);
    if generator.accepted < generator.options.max_moves && !should_stop() {
        let (entity_index, mov, target) =
            next_assignment_candidate(&mut generator, &completed_slots, &mut should_stop)?;
        let group_slot = assignment_group_slot(generator.group_index, entity_index);
        let scalar_slots = target.scalar_slots().to_vec();
        let allows_unassigned = generator.assignment.target.allows_unassigned;
        let placement = Placement::new(
            EntityReference::new(generator.assignment.target.descriptor_index, entity_index),
            ScalarGroupCandidateCursor::assignment(
                generator,
                entity_index,
                completed_slots,
                (mov, target),
            ),
        )
        .with_group_slot(group_slot)
        .with_scalar_slots(scalar_slots)
        .with_keep_current_legal(allows_unassigned)
        .with_candidate_target(ScalarGroupCandidateCursor::construction_target);
        if !is_completed(&placement) {
            return Some(placement);
        }
    }
    None
}

fn next_assignment_candidate<S, ShouldStop>(
    generator: &mut AssignmentPlacementGenerator<S>,
    completed_slots: &HashSet<ConstructionSlotId>,
    should_stop: &mut ShouldStop,
) -> Option<(usize, CompoundScalarMove<S>, ConstructionTarget)>
where
    S: PlanningSolution + 'static,
    ShouldStop: FnMut() -> bool,
{
    loop {
        if should_stop() {
            return None;
        }
        if generator.accepted >= generator.options.max_moves {
            return None;
        }
        let mov = if let Some(pending) = generator.pending.take() {
            pending
        } else {
            generator.cursor.next_move_with_control(should_stop)?
        };
        if mov.edits().iter().any(|edit| {
            completed_slots.contains(&ConstructionSlotId::new(
                generator.assignment.target().construction_binding_index(),
                edit.entity_index,
            ))
        }) {
            continue;
        }
        let snapshot = generator.cursor.construction_snapshot();
        if mov.edits().is_empty()
            || !mov
                .edits()
                .iter()
                .any(|edit| edit.current_value(snapshot) != edit.to_value)
        {
            continue;
        }
        let anchor = mov
            .edits()
            .iter()
            .find(|edit| edit.current_value(snapshot).is_none() && edit.to_value.is_some())
            .or_else(|| mov.edits().iter().find(|edit| edit.to_value.is_some()))
            .or_else(|| mov.edits().first())?;
        let entity_index = anchor.entity_index;
        let order_key = principal_assignment_edit(&mov, entity_index)
            .and_then(|principal| principal.to_value)
            .and_then(|value| {
                generator
                    .assignment
                    .value_order_key(snapshot, entity_index, value)
            });
        let mov = mov.with_construction_value_order_key(order_key);
        let group_slot = assignment_group_slot(generator.group_index, entity_index);
        let target = assignment_move_target(&group_slot);
        return Some((entity_index, mov, target));
    }
}

fn completed_assignment_slots<S, IsCompleted>(
    generator: &AssignmentPlacementGenerator<S>,
    is_completed: &mut IsCompleted,
) -> HashSet<ConstructionSlotId>
where
    S: PlanningSolution + 'static,
    IsCompleted: FnMut(&Placement<S, CompoundScalarMove<S>, ScalarGroupCandidateCursor<S>>) -> bool,
{
    (0..generator
        .assignment
        .entity_count(generator.cursor.construction_snapshot()))
        .filter_map(|entity_index| {
            let slot_id = ConstructionSlotId::new(
                generator.assignment.target().construction_binding_index(),
                entity_index,
            );
            let placement = Placement::new(
                EntityReference::new(generator.assignment.target.descriptor_index, entity_index),
                ScalarGroupCandidateCursor::empty(),
            )
            .with_scalar_slots(vec![slot_id]);
            is_completed(&placement).then_some(slot_id)
        })
        .collect()
}

pub(super) fn assignment_placement_move_limit(
    heuristic: ConstructionHeuristicType,
    construction_obligation: ConstructionObligation,
    required_only: bool,
    entity_count: usize,
    options: ScalarAssignmentMoveOptions,
) -> usize {
    if matches!(
        heuristic,
        ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::FirstFitDecreasing
    ) && matches!(
        construction_obligation,
        ConstructionObligation::AssignWhenCandidateExists
    ) {
        if required_only && matches!(heuristic, ConstructionHeuristicType::FirstFit) {
            return options.max_moves.min(1);
        }
        if options.max_moves != usize::MAX {
            return options.max_moves;
        }
        return entity_count
            .saturating_mul(options.max_rematch_size)
            .clamp(256, 4096);
    }
    options.max_moves
}
