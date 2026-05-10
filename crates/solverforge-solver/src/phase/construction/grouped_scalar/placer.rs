use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_config::{ConstructionHeuristicType, ConstructionObligation};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::assignment_candidate::{remaining_required_count, ScalarAssignmentMoveOptions};
use super::assignment_stream::ScalarAssignmentMoveCursor;
use super::move_build::compound_move_for_group_candidate;
use super::placement::{
    assignment_target_binding, group_slot_id, never_completed, placement_for_group_candidate,
    push_or_merge_placement, scalar_slots_for_candidate,
};
use super::placer_stream::{
    assignment_placement_move_limit, next_assignment_placement, next_candidate_placement,
    sort_grouped_placements, AssignmentPlacementGenerator, CandidatePlacementGenerator,
};
use crate::builder::context::{
    ScalarAssignmentBinding, ScalarGroupBinding, ScalarGroupBindingKind, ScalarGroupLimits,
};
use crate::descriptor::ResolvedVariableBinding;
use crate::heuristic::r#move::{CompoundScalarMove, Move};
use crate::phase::construction::capabilities::{
    grouped_heuristic_requires_entity_order, grouped_heuristic_requires_value_order,
};
use crate::phase::construction::{EntityPlacer, Placement};

pub(super) type ScalarGroupPlacement<S> = Placement<S, CompoundScalarMove<S>>;

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

impl<S> EntityPlacer<S, CompoundScalarMove<S>> for ScalarGroupPlacer<S>
where
    S: PlanningSolution + 'static,
{
    fn get_placements<D: Director<S>>(&self, score_director: &D) -> Vec<ScalarGroupPlacement<S>> {
        let mut keep = never_completed::<S>;
        let mut generated_moves = 0;
        let mut placements = Vec::new();
        match self.group.kind {
            ScalarGroupBindingKind::Candidates { candidate_provider } => {
                let mut generator =
                    self.candidate_placement_generator(score_director, candidate_provider);
                while let Some(placement) =
                    next_candidate_placement(&mut generator, &mut generated_moves, &mut keep)
                {
                    placements.push(placement);
                }
            }
            ScalarGroupBindingKind::Assignment(assignment) => {
                if let Some(mut generator) =
                    self.assignment_placement_generator(score_director, assignment)
                {
                    while let Some(placement) = next_assignment_placement(
                        &mut generator,
                        score_director,
                        &mut generated_moves,
                        &mut keep,
                    ) {
                        placements.push(placement);
                    }
                }
            }
        }
        placements
    }

    fn get_next_placement<D, IsCompleted>(
        &self,
        score_director: &D,
        mut is_completed: IsCompleted,
    ) -> Option<(ScalarGroupPlacement<S>, u64)>
    where
        D: Director<S>,
        IsCompleted: FnMut(&ScalarGroupPlacement<S>) -> bool,
    {
        let mut generated_moves = 0;
        let placement = match self.group.kind {
            ScalarGroupBindingKind::Candidates { candidate_provider } => {
                let mut generator =
                    self.candidate_placement_generator(score_director, candidate_provider);
                next_candidate_placement(&mut generator, &mut generated_moves, &mut is_completed)
            }
            ScalarGroupBindingKind::Assignment(assignment) => {
                let mut generator =
                    self.assignment_placement_generator(score_director, assignment)?;
                next_assignment_placement(
                    &mut generator,
                    score_director,
                    &mut generated_moves,
                    &mut is_completed,
                )
            }
        };
        placement.map(|placement| (placement, generated_moves))
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

            let group_slot = group_slot_id(self.group_index, &candidate, &scalar_slots);
            let Some(mut mov) =
                compound_move_for_group_candidate(&self.group, solution, &candidate)
            else {
                continue;
            };
            if !mov.is_doable(score_director) {
                continue;
            }
            mov = mov.with_construction_value_order_key(candidate.construction_value_order_key());
            let placement = placement_for_group_candidate(
                sequence,
                &candidate,
                group_slot,
                scalar_slots,
                keep_current_legal,
                mov,
            )
            .with_construction_entity_order_key(candidate.construction_entity_order_key());
            push_or_merge_placement(&mut placements, placement);
            seen_candidates.push(candidate);
            accepted += 1;
        }
        sort_grouped_placements(&mut placements, self.heuristic);
        CandidatePlacementGenerator {
            placements: placements.into_iter(),
        }
    }

    fn assignment_placement_generator<D>(
        &self,
        score_director: &D,
        assignment: ScalarAssignmentBinding<S>,
    ) -> Option<AssignmentPlacementGenerator<S>>
    where
        D: Director<S>,
    {
        let solution = score_director.working_solution();
        let Some(target_binding) = assignment_target_binding(&assignment, &self.scalar_bindings)
        else {
            panic!(
                "assignment-backed grouped scalar construction targets unknown scalar slot {}.{}",
                assignment.target.entity_type_name, assignment.target.variable_name
            );
        };
        let options = ScalarAssignmentMoveOptions::for_construction(self.limits);
        if options.max_moves == 0 {
            return None;
        }

        let required_remaining = remaining_required_count(&assignment, solution);
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
            ScalarAssignmentMoveCursor::required_construction(assignment, solution.clone(), options)
        } else if target_required {
            ScalarAssignmentMoveCursor::required(assignment, solution.clone(), options)
        } else {
            ScalarAssignmentMoveCursor::optional_construction(assignment, solution.clone(), options)
        };

        Some(AssignmentPlacementGenerator {
            group_index: self.group_index,
            assignment,
            target_binding: target_binding.clone(),
            cursor,
            pending: None,
            options,
            accepted: 0,
        })
    }
}
