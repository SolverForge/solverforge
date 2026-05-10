use std::collections::{HashMap, HashSet};

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{
    ordered_entities, rotate_entity_order, AssignmentMoveIntent, ScalarAssignmentMoveOptions,
};
use super::assignment_path::{assignment_move_for_entity_value, move_from_edits};
use super::assignment_state::{CapacityConflict, ScalarAssignmentState};
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

pub(super) struct EntityValueCursor {
    pub(super) entities: Vec<usize>,
    pub(super) entity_pos: usize,
    pub(super) values: Vec<usize>,
    pub(super) value_pos: usize,
    pub(super) value_degrees: HashMap<usize, usize>,
    pub(super) options: ScalarAssignmentMoveOptions,
    pub(super) kind: AssignmentMoveKind,
}

impl EntityValueCursor {
    pub(super) fn next<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        let intent = self.kind.intent();
        while self.entity_pos < self.entities.len() {
            if self.values.is_empty() {
                let entity_index = self.entities[self.entity_pos];
                self.values = group.candidate_values(
                    solution,
                    entity_index,
                    self.options.value_candidate_limit,
                );
                if self.kind == AssignmentMoveKind::Required {
                    sort_values_by_required_scarcity(
                        group,
                        solution,
                        entity_index,
                        &self.value_degrees,
                        &mut self.values,
                    );
                }
                self.value_pos = 0;
            }
            let entity_index = self.entities[self.entity_pos];
            while self.value_pos < self.values.len() {
                let value = self.values[self.value_pos];
                self.value_pos += 1;
                if let Some(mov) = assignment_move_for_entity_value(
                    group,
                    solution,
                    state,
                    entity_index,
                    value,
                    self.options,
                    intent,
                ) {
                    return Some(mov);
                }
            }
            self.entity_pos += 1;
            self.values.clear();
            self.value_pos = 0;
        }
        None
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AssignmentMoveKind {
    Required,
    Optional,
    CapacityRepair,
    Reassignment,
}

impl AssignmentMoveKind {
    fn intent(self) -> AssignmentMoveIntent {
        match self {
            Self::Required => AssignmentMoveIntent::required(),
            Self::Optional => AssignmentMoveIntent::optional(),
            Self::CapacityRepair => AssignmentMoveIntent::capacity_repair(),
            Self::Reassignment => AssignmentMoveIntent::reassignment(),
        }
    }
}

pub(super) struct CapacityCursor {
    conflicts: Vec<CapacityConflict>,
    conflict_pos: usize,
    occupant_pos: usize,
    seen_entities: HashSet<usize>,
    repair_cursor: Option<EntityValueCursor>,
    options: ScalarAssignmentMoveOptions,
}

impl CapacityCursor {
    pub(super) fn new<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        Self {
            conflicts: state.capacity_conflicts(group, solution),
            conflict_pos: 0,
            occupant_pos: 0,
            seen_entities: HashSet::new(),
            repair_cursor: None,
            options,
        }
    }

    pub(super) fn next<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        loop {
            if let Some(cursor) = &mut self.repair_cursor {
                if let Some(mov) = cursor.next(group, solution, state) {
                    return Some(mov);
                }
                self.repair_cursor = None;
            }

            let conflict = self.conflicts.get(self.conflict_pos)?;
            if self.occupant_pos >= conflict.occupants.len() {
                self.conflict_pos += 1;
                self.occupant_pos = 0;
                continue;
            }

            let entity_index = conflict.occupants[self.occupant_pos];
            self.occupant_pos += 1;
            if !self.seen_entities.insert(entity_index) {
                continue;
            }

            if !state.is_required(entity_index) {
                let edits = [(entity_index, None)];
                if !state.assignment_feasible_after_edits(group, solution, &edits) {
                    continue;
                }
                let scalar_edits = [group.edit(entity_index, None)];
                if let Some(mov) = move_from_edits(
                    group,
                    solution,
                    &scalar_edits,
                    "scalar_assignment_capacity_repair",
                ) {
                    return Some(mov);
                }
                continue;
            }

            self.repair_cursor = Some(EntityValueCursor {
                entities: vec![entity_index],
                entity_pos: 0,
                values: Vec::new(),
                value_pos: 0,
                value_degrees: HashMap::new(),
                options: self.options,
                kind: AssignmentMoveKind::CapacityRepair,
            });
        }
    }
}

#[derive(Clone, Copy)]
enum OptionalAdjustmentKind {
    Release,
    Transfer,
}

pub(super) struct OptionalAdjustmentCursor {
    kind: OptionalAdjustmentKind,
    targets: Vec<usize>,
    sources: Vec<usize>,
    target_pos: usize,
    source_pos: usize,
}

impl OptionalAdjustmentCursor {
    pub(super) fn release<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        let mut targets = ordered_entities(group, solution, |entity_index| {
            !state.is_required(entity_index) && state.current_value(entity_index).is_some()
        });
        rotate_entity_order(&mut targets, options.entity_offset);
        Self {
            kind: OptionalAdjustmentKind::Release,
            targets,
            sources: Vec::new(),
            target_pos: 0,
            source_pos: 0,
        }
    }

    pub(super) fn transfer<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        let mut targets = ordered_entities(group, solution, |entity_index| {
            !state.is_required(entity_index) && state.current_value(entity_index).is_none()
        });
        let mut sources = ordered_entities(group, solution, |entity_index| {
            !state.is_required(entity_index) && state.current_value(entity_index).is_some()
        });
        rotate_entity_order(&mut targets, options.entity_offset);
        rotate_entity_order(&mut sources, options.entity_offset);
        Self {
            kind: OptionalAdjustmentKind::Transfer,
            targets,
            sources,
            target_pos: 0,
            source_pos: 0,
        }
    }

    pub(super) fn next<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        match self.kind {
            OptionalAdjustmentKind::Release => self.next_release(group, solution, state),
            OptionalAdjustmentKind::Transfer => self.next_transfer(group, solution, state),
        }
    }

    fn next_release<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        while self.target_pos < self.targets.len() {
            let entity_index = self.targets[self.target_pos];
            self.target_pos += 1;
            let edits = [(entity_index, None)];
            if !state.assignment_feasible_after_edits(group, solution, &edits) {
                continue;
            }
            let scalar_edits = [group.edit(entity_index, None)];
            if let Some(mov) = move_from_edits(
                group,
                solution,
                &scalar_edits,
                "scalar_assignment_optional_release",
            ) {
                return Some(mov);
            }
        }
        None
    }

    fn next_transfer<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        while self.target_pos < self.targets.len() {
            let target = self.targets[self.target_pos];
            while self.source_pos < self.sources.len() {
                let source = self.sources[self.source_pos];
                self.source_pos += 1;
                let Some(value) = state.current_value(source) else {
                    continue;
                };
                let edits = [(source, None), (target, Some(value))];
                if !state.assignment_feasible_after_edits(group, solution, &edits) {
                    continue;
                }
                let scalar_edits = [group.edit(source, None), group.edit(target, Some(value))];
                if let Some(mov) = move_from_edits(
                    group,
                    solution,
                    &scalar_edits,
                    "scalar_assignment_optional_transfer",
                ) {
                    return Some(mov);
                }
            }
            self.target_pos += 1;
            self.source_pos = 0;
        }
        None
    }
}

pub(super) fn assigned_entities_by_position<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &ScalarAssignmentState,
) -> Vec<usize> {
    let mut entities = (0..group.entity_count(solution))
        .filter(|entity_index| state.current_value(*entity_index).is_some())
        .collect::<Vec<_>>();
    entities.sort_by_key(|entity_index| {
        (
            group.position_key(solution, *entity_index),
            group.entity_order_key(solution, *entity_index),
            *entity_index,
        )
    });
    entities
}

pub(super) fn required_entities_by_scarcity<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &ScalarAssignmentState,
    value_candidate_limit: Option<usize>,
) -> Vec<usize>
where
    S: PlanningSolution,
{
    let mut entities = (0..group.entity_count(solution))
        .filter(|entity_index| {
            state.is_required(*entity_index) && state.current_value(*entity_index).is_none()
        })
        .map(|entity_index| {
            let candidate_count = group
                .candidate_values(solution, entity_index, value_candidate_limit)
                .len();
            (
                candidate_count,
                group.entity_order_key(solution, entity_index),
                entity_index,
            )
        })
        .collect::<Vec<_>>();
    entities.sort_unstable();
    entities
        .into_iter()
        .map(|(_, _, entity_index)| entity_index)
        .collect()
}

pub(super) fn required_value_degrees<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    entities: &[usize],
    value_candidate_limit: Option<usize>,
) -> HashMap<usize, usize>
where
    S: PlanningSolution,
{
    let mut degrees = HashMap::new();
    for entity_index in entities {
        for value in group.candidate_values(solution, *entity_index, value_candidate_limit) {
            *degrees.entry(value).or_insert(0) += 1;
        }
    }
    degrees
}

pub(super) fn sort_values_by_required_scarcity<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    entity_index: usize,
    degrees: &HashMap<usize, usize>,
    values: &mut [usize],
) {
    values.sort_by_key(|value| {
        (
            degrees.get(value).copied().unwrap_or(usize::MAX),
            group.value_order_key(solution, entity_index, *value),
            *value,
        )
    });
}
