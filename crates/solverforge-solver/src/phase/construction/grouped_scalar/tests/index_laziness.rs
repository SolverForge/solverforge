use super::*;
use crate::builder::{bind_scalar_groups, ScalarGroupBindingKind, ScalarVariableSlot, ValueSource};
use crate::planning::{ScalarGroup, ScalarGroupLimits, ScalarTarget};
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct Plan {
    values: Vec<Option<usize>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<SoftScore> {
        self.score
    }

    fn set_score(&mut self, score: Option<SoftScore>) {
        self.score = score;
    }
}

#[test]
fn required_only_pulls_do_not_build_the_value_sequence_index() {
    let slot = ScalarVariableSlot::new(
        0,
        0,
        "Row",
        |plan: &Plan| plan.values.len(),
        "value",
        |plan, entity, _| plan.values[entity],
        |plan, entity, _, value| plan.values[entity] = value,
        ValueSource::EntitySlice {
            values_for_entity: |_, _, _| &[1],
        },
        true,
    );
    let mut groups = bind_scalar_groups(
        vec![ScalarGroup::assignment(
            "assignment",
            ScalarTarget::from_descriptor_index(0, "value"),
        )
        .with_required_entity(|_: &Plan, _| true)],
        &[slot],
    );
    let ScalarGroupBindingKind::Assignment(binding) = groups.remove(0).kind else {
        panic!("expected assignment binding");
    };
    let mut cursor = ScalarAssignmentMoveCursor::required(
        binding,
        Plan {
            values: vec![Some(0), None],
            score: None,
        },
        ScalarAssignmentMoveOptions::for_construction(ScalarGroupLimits::new()),
    );
    assert!(cursor.index_cache.is_none());
    assert!(cursor.next_move_with_control(&mut || false).is_some());
    assert!(
        cursor.index_cache.is_none(),
        "required-only pulls must not build an unused value-sequence index"
    );
}
