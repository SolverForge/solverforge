use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_config::{ConstructionHeuristicType, ConstructionObligation};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::assignment_candidate::{remaining_required_count, ScalarAssignmentMoveOptions};
use super::assignment_stream::ScalarAssignmentMoveCursor;
use super::placement::{group_slot_id, scalar_slots_for_candidate};
use super::placer_stream::{
    assignment_placement_move_limit, next_assignment_placement, next_candidate_placement,
    AssignmentPlacementGenerator, CandidatePlacementGenerator, CandidatePlacementSeed,
    ScalarGroupCandidateCursor,
};
use crate::builder::context::{
    ScalarAssignmentBinding, ScalarGroupBinding, ScalarGroupBindingKind, ScalarGroupLimits,
};
use crate::descriptor::ResolvedVariableBinding;
use crate::heuristic::r#move::CompoundScalarMove;
use crate::heuristic::selector::EntityReference;
use crate::phase::construction::{EntityPlacer, EntityPlacerCursor, Placement};

pub(super) type ScalarGroupPlacement<S> =
    Placement<S, CompoundScalarMove<S>, ScalarGroupCandidateCursor<S>>;

fn grouped_heuristic_requires_entity_order(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::FirstFitDecreasing
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFitDecreasing
    )
}

fn grouped_heuristic_requires_value_order(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing
    )
}

pub(crate) struct ScalarGroupPlacer<S> {
    group_index: usize,
    group: ScalarGroupBinding<S>,
    scalar_bindings: Vec<ResolvedVariableBinding<S>>,
    limits: ScalarGroupLimits,
    heuristic: ConstructionHeuristicType,
    construction_obligation: ConstructionObligation,
    required_only: bool,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Clone for ScalarGroupPlacer<S> {
    fn clone(&self) -> Self {
        Self {
            group_index: self.group_index,
            group: self.group.clone(),
            scalar_bindings: self.scalar_bindings.clone(),
            limits: self.limits,
            heuristic: self.heuristic,
            construction_obligation: self.construction_obligation,
            required_only: self.required_only,
            _phantom: PhantomData,
        }
    }
}

impl<S> ScalarGroupPlacer<S> {
    pub(crate) fn new(
        group_index: usize,
        group: ScalarGroupBinding<S>,
        scalar_bindings: Vec<ResolvedVariableBinding<S>>,
        limits: ScalarGroupLimits,
        heuristic: ConstructionHeuristicType,
        construction_obligation: ConstructionObligation,
        required_only: bool,
    ) -> Self {
        Self {
            group_index,
            group,
            scalar_bindings,
            limits,
            heuristic,
            construction_obligation,
            required_only,
            _phantom: PhantomData,
        }
    }
}

impl<S> Debug for ScalarGroupPlacer<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarGroupPlacer")
            .field("group_index", &self.group_index)
            .field("group_name", &self.group.group_name)
            .field("limits", &self.limits)
            .field("heuristic", &self.heuristic)
            .field("construction_obligation", &self.construction_obligation)
            .field("required_only", &self.required_only)
            .finish()
    }
}

pub(crate) struct ScalarGroupPlacerCursor<'a, S>
where
    S: PlanningSolution + 'static,
{
    placer: &'a ScalarGroupPlacer<S>,
}

impl<S> EntityPlacerCursor<S, CompoundScalarMove<S>> for ScalarGroupPlacerCursor<'_, S>
where
    S: PlanningSolution + 'static,
{
    type CandidateCursor = ScalarGroupCandidateCursor<S>;

    fn next_placement<D, IsCompleted, ShouldStop>(
        &mut self,
        score_director: &D,
        mut is_completed: IsCompleted,
        mut should_stop: ShouldStop,
    ) -> Option<ScalarGroupPlacement<S>>
    where
        D: Director<S>,
        IsCompleted: FnMut(&ScalarGroupPlacement<S>) -> bool,
        ShouldStop: FnMut() -> bool,
    {
        match &self.placer.group.kind {
            ScalarGroupBindingKind::Candidates { candidate_provider } => {
                let mut generator = self
                    .placer
                    .candidate_placement_generator(score_director, *candidate_provider);
                next_candidate_placement(&mut generator, &mut is_completed, &mut should_stop)
            }
            ScalarGroupBindingKind::Assignment(assignment) => next_assignment_placement(
                self.placer
                    .assignment_placement_generator(score_director, assignment)?,
                &mut is_completed,
                &mut should_stop,
            ),
        }
    }
}

impl<S> EntityPlacer<S, CompoundScalarMove<S>> for ScalarGroupPlacer<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = ScalarGroupPlacerCursor<'a, S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let _ = score_director;
        ScalarGroupPlacerCursor { placer: self }
    }
}

impl<S> ScalarGroupPlacer<S>
where
    S: PlanningSolution + 'static,
{
    fn candidate_placement_generator<D>(
        &self,
        score_director: &D,
        candidate_provider: crate::builder::context::ScalarCandidateProvider<S>,
    ) -> CandidatePlacementGenerator<S>
    where
        D: Director<S>,
    {
        let solution = score_director.working_solution();
        let raw_candidates = candidate_provider(solution, self.limits);
        let total_limit = self.limits.group_candidate_limit.unwrap_or(usize::MAX);
        if total_limit == 0 {
            return CandidatePlacementGenerator {
                placements: Vec::new().into_iter(),
                group: self.group.clone(),
            };
        }

        let mut seen_candidates = Vec::new();
        let mut placements = Vec::new();
        let mut accepted = 0usize;
        for (sequence, candidate) in raw_candidates.into_iter().enumerate() {
            if accepted >= total_limit {
                break;
            }
            if candidate.edits().is_empty()
                || seen_candidates
                    .iter()
                    .any(|seen_candidate| seen_candidate == &candidate)
            {
                continue;
            }
            let Some((scalar_slots, keep_current_legal, has_unfinished_unassigned_slot)) =
                scalar_slots_for_candidate(
                    &self.group,
                    &self.scalar_bindings,
                    solution,
                    &candidate,
                )
            else {
                continue;
            };
            if !has_unfinished_unassigned_slot {
                continue;
            }
            if grouped_heuristic_requires_entity_order(self.heuristic)
                && candidate.construction_entity_order_key().is_none()
            {
                panic!(
                    "grouped scalar construction heuristic {:?} requires construction entity order metadata",
                    self.heuristic
                );
            }
            if grouped_heuristic_requires_value_order(self.heuristic)
                && candidate.construction_value_order_key().is_none()
            {
                panic!(
                    "grouped scalar construction heuristic {:?} requires construction value order metadata",
                    self.heuristic
                );
            }

            if !candidate.edits().iter().any(|edit| {
                let member = self
                    .group
                    .member_for_edit(edit)
                    .expect("validated grouped scalar candidate member must exist");
                member.current_value(solution, edit.entity_index()) != edit.to_value()
            }) {
                continue;
            }
            let group_slot = group_slot_id(self.group_index, &candidate, &scalar_slots);
            if let Some(existing) =
                placements
                    .iter_mut()
                    .find(|existing: &&mut CandidatePlacementSeed<S>| {
                        existing.group_slot == group_slot && existing.scalar_slots == scalar_slots
                    })
            {
                existing.candidates.push(candidate.clone());
            } else {
                let entity_ref = candidate
                    .edits()
                    .first()
                    .map(|edit| EntityReference::new(edit.descriptor_index(), edit.entity_index()))
                    .unwrap_or_else(|| EntityReference::new(0, sequence));
                placements.push(CandidatePlacementSeed {
                    sequence,
                    entity_ref,
                    candidates: vec![candidate.clone()],
                    group_slot,
                    scalar_slots,
                    keep_current_legal,
                    entity_order_key: candidate.construction_entity_order_key(),
                });
            }
            seen_candidates.push(candidate);
            accepted += 1;
        }
        if grouped_heuristic_requires_entity_order(self.heuristic) {
            placements.sort_by(|left, right| {
                right
                    .entity_order_key
                    .cmp(&left.entity_order_key)
                    .then_with(|| left.sequence.cmp(&right.sequence))
            });
        }
        CandidatePlacementGenerator {
            placements: placements.into_iter(),
            group: self.group.clone(),
        }
    }

    fn assignment_placement_generator<D>(
        &self,
        score_director: &D,
        assignment: &ScalarAssignmentBinding<S>,
    ) -> Option<AssignmentPlacementGenerator<S>>
    where
        D: Director<S>,
    {
        let solution = score_director.working_solution();
        let _ = assignment.target().construction_binding_index();
        let options = ScalarAssignmentMoveOptions::for_construction(self.limits);
        if options.max_moves == 0 {
            return None;
        }

        let required_remaining = remaining_required_count(assignment, solution);
        if self.required_only && required_remaining == 0 {
            return None;
        }
        let target_required = self.required_only || required_remaining > 0;
        let max_moves = assignment_placement_move_limit(
            self.heuristic,
            self.construction_obligation,
            self.required_only,
            assignment.entity_count(solution),
            options,
        );
        let options = options.with_max_moves(max_moves);
        let cursor = if self.required_only {
            ScalarAssignmentMoveCursor::required_construction(
                assignment.clone(),
                solution.clone(),
                options,
            )
        } else if target_required {
            ScalarAssignmentMoveCursor::required(assignment.clone(), solution.clone(), options)
        } else {
            ScalarAssignmentMoveCursor::optional_construction(
                assignment.clone(),
                solution.clone(),
                options,
            )
        };

        Some(AssignmentPlacementGenerator {
            group_index: self.group_index,
            assignment: assignment.clone(),
            cursor,
            pending: None,
            options,
            accepted: 0,
        })
    }
}
