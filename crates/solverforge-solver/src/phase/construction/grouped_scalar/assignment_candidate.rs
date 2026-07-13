use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::selector::move_selector::MoveStreamContext;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScalarAssignmentMoveOptions {
    pub(crate) value_candidate_limit: Option<usize>,
    pub(crate) max_moves: usize,
    pub(crate) max_depth: usize,
    pub(crate) max_rematch_size: usize,
    pub(crate) selection_context: MoveStreamContext,
    pub(crate) required_scarcity_ordering: bool,
}

impl ScalarAssignmentMoveOptions {
    pub(crate) fn for_construction(limits: crate::builder::ScalarGroupLimits) -> Self {
        Self {
            value_candidate_limit: limits.value_candidate_limit,
            max_moves: limits.group_candidate_limit.unwrap_or(usize::MAX),
            max_depth: limits.max_augmenting_depth.unwrap_or(3),
            max_rematch_size: limits.max_rematch_size.unwrap_or(4).max(2),
            selection_context: MoveStreamContext::default(),
            required_scarcity_ordering: true,
        }
    }

    pub(crate) fn for_selector(
        limits: crate::builder::ScalarGroupLimits,
        value_candidate_limit: Option<usize>,
        max_moves_per_step: usize,
        selection_context: MoveStreamContext,
    ) -> Self {
        Self {
            value_candidate_limit: value_candidate_limit.or(limits.value_candidate_limit),
            max_moves: max_moves_per_step,
            max_depth: limits.max_augmenting_depth.unwrap_or(3),
            max_rematch_size: limits.max_rematch_size.unwrap_or(4).max(2),
            selection_context,
            required_scarcity_ordering: true,
        }
    }

    pub(crate) fn with_max_moves(mut self, max_moves: usize) -> Self {
        self.max_moves = max_moves;
        self
    }
}

#[derive(Clone, Copy)]
pub(super) struct AssignmentMoveIntent {
    pub(super) allow_optional_displacement: bool,
    pub(super) reason: &'static str,
}

impl AssignmentMoveIntent {
    pub(super) const fn required() -> Self {
        Self {
            allow_optional_displacement: true,
            reason: "scalar_assignment_required",
        }
    }

    pub(super) const fn optional() -> Self {
        Self {
            allow_optional_displacement: false,
            reason: "scalar_assignment_optional",
        }
    }

    pub(super) const fn capacity_repair() -> Self {
        Self {
            allow_optional_displacement: true,
            reason: "scalar_assignment_capacity_repair",
        }
    }

    pub(super) const fn reassignment() -> Self {
        Self {
            allow_optional_displacement: true,
            reason: "scalar_assignment_reassignment",
        }
    }
}

pub(crate) fn remaining_required_count<S>(group: &ScalarAssignmentBinding<S>, solution: &S) -> u64 {
    group.remaining_required_count(solution)
}

pub(super) fn ordered_entities<S, F>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    mut predicate: F,
) -> Vec<usize>
where
    F: FnMut(usize) -> bool,
{
    let mut entities = (0..group.entity_count(solution))
        .filter(|entity_index| predicate(*entity_index))
        .collect::<Vec<_>>();
    entities.sort_by_key(|entity_index| {
        (
            group.entity_order_key(solution, *entity_index),
            *entity_index,
        )
    });
    entities
}

pub(super) fn order_candidates<T: Clone>(
    candidates: &mut [T],
    options: ScalarAssignmentMoveOptions,
    salt: u64,
) {
    options
        .selection_context
        .apply_selection_order_without_replacement(candidates, salt);
}
