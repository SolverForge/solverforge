use super::*;
use crate::builder::{bind_scalar_groups, ScalarGroupBindingKind, ScalarVariableSlot, ValueSource};
use crate::planning::{ScalarGroup, ScalarGroupLimits, ScalarTarget};
use solverforge_core::score::SoftScore;

use crate::heuristic::selector::move_selector::{MoveCursor, MoveSelector};
use solverforge_core::domain::SolutionDescriptor;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};

static CLONES: AtomicUsize = AtomicUsize::new(0);

struct Plan {
    values: Vec<Option<usize>>,
    score: Option<SoftScore>,
}

impl Clone for Plan {
    fn clone(&self) -> Self {
        CLONES.fetch_add(1, Ordering::Relaxed);
        Self {
            values: self.values.clone(),
            score: self.score,
        }
    }
}

#[test]
fn grouped_activation_and_pulls_lend_the_solution_without_cloning() {
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
    let mut group = bind_scalar_groups(
        vec![ScalarGroup::assignment(
            "assignment",
            ScalarTarget::from_descriptor_index(0, "value"),
        )
        .with_required_entity(|_: &Plan, _| true)],
        &[slot],
    )
    .remove(0);
    group
        .assignment_mut()
        .expect("assignment binding")
        .target_mut()
        .set_construction_binding_index(0);
    let placer = super::super::placer::ScalarGroupPlacer::new(
        0,
        group.clone(),
        vec![],
        ScalarGroupLimits::new(),
        solverforge_config::ConstructionHeuristicType::FirstFit,
        solverforge_config::ConstructionObligation::AssignWhenCandidateExists,
        true,
    );
    let selector =
        crate::builder::selector::GroupedScalarSelector::new(group, None, Some(8), false);
    let plan = Plan {
        values: vec![None],
        score: None,
    };
    let descriptor = SolutionDescriptor::new("Plan", TypeId::of::<Plan>());
    let director = ScoreDirector::simple(plan, descriptor, |_, _| 1);
    CLONES.store(0, Ordering::Relaxed);
    let mut cursor = selector.open_cursor(&director);
    assert_eq!(
        CLONES.load(Ordering::Relaxed),
        0,
        "opening must lend, not snapshot"
    );
    let id = cursor
        .next_candidate(director.working_solution())
        .expect("required candidate");
    assert!(cursor.candidate(id).is_some());
    assert_eq!(
        CLONES.load(Ordering::Relaxed),
        0,
        "activation and generation must lend"
    );
    assert_eq!(director.working_solution().values, vec![None]);
    use crate::phase::construction::{EntityPlacer, EntityPlacerCursor};
    let mut construction = placer.open_cursor(&director);
    let mut placement = construction
        .next_placement(&director, |_| false, || false)
        .expect("required placement");
    let id = placement
        .candidates_mut()
        .next_candidate(director.working_solution())
        .expect("placement candidate");
    assert!(placement.candidates_mut().candidate(id).is_some());
    assert_eq!(
        CLONES.load(Ordering::Relaxed),
        0,
        "construction must lend too"
    );
    assert_eq!(director.working_solution().values, vec![None]);
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
    let solution = Plan {
        values: vec![Some(0), None],
        score: None,
    };
    let mut cursor = ScalarAssignmentMoveCursor::required(
        &binding,
        &solution,
        ScalarAssignmentMoveOptions::for_construction(ScalarGroupLimits::new()),
    );
    assert!(cursor.index_cache.is_none());
    assert!(cursor
        .next_move_with_control(&binding, &solution, &mut || false)
        .is_some());
    assert!(
        cursor.index_cache.is_none(),
        "required-only pulls must not build an unused value-sequence index"
    );
}

fn union_edit_rows(
    mov: &crate::heuristic::r#move::ScalarMoveUnion<Plan, usize>,
) -> Vec<(usize, Option<usize>)> {
    match mov {
        crate::heuristic::r#move::ScalarMoveUnion::CompoundScalar(compound) => compound
            .edits()
            .iter()
            .map(|edit| (edit.entity_index, edit.to_value))
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn grouped_ls_cursor_reactivates_from_committed_state_between_steps() {
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
    let mut group = bind_scalar_groups(
        vec![ScalarGroup::assignment(
            "assignment",
            ScalarTarget::from_descriptor_index(0, "value"),
        )
        .with_required_entity(|_: &Plan, entity| entity % 2 == 0)],
        &[slot],
    )
    .remove(0);
    group
        .assignment_mut()
        .expect("assignment binding")
        .target_mut()
        .set_construction_binding_index(0);
    let selector =
        crate::builder::selector::GroupedScalarSelector::new(group, None, Some(8), false);
    // Two rows: even (required, unassigned), odd (optional, already assigned).
    let mut director = ScoreDirector::simple(
        Plan {
            values: vec![None, Some(1)],
            score: None,
        },
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
        |_, _| 1,
    );

    let mut first = Vec::new();
    let mut first_cursor = selector.open_cursor(&director);
    while let Some(id) = first_cursor.next_candidate(director.working_solution()) {
        first.push(union_edit_rows(&first_cursor.take_candidate(id)));
    }
    assert!(
        first
            .iter()
            .flatten()
            .any(|(entity, to_value)| *entity == 0 && *to_value == Some(1)),
        "required row 0 must be offered while it is unassigned"
    );

    // Commit a step: the required row's assignment lands through the funnel.
    // The step loop drops the step cursor first; the reopened step-2 cursor
    // must therefore re-derive its view from the committed working solution.
    director.working_solution_mut().values[0] = Some(1);

    let mut second = Vec::new();
    let mut second_cursor = selector.open_cursor(&director);
    while let Some(id) = second_cursor.next_candidate(director.working_solution()) {
        second.push(union_edit_rows(&second_cursor.take_candidate(id)));
    }

    // A stale activation (built before the commit) would still offer the
    // required re-fill of row 0 from None. Only the committed baseline can
    // remove that chain.
    assert!(
        second
            .iter()
            .flatten()
            .all(|(entity, to_value)| !(*entity == 0 && *to_value == Some(1))),
        "the reactivated cursor must not refill the already-committed row: {second:?}"
    );
}
