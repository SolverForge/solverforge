//! T10 reference differential for the solve-owned pooled assignment base.
//!
//! Random commit/step/replacement sequence: after every step the maintained
//! (pooled) base must equal a freshly rebuilt base, and the pooled step's
//! candidate stream must match a rebuilt step cursor's stream exactly (same
//! edits, same order). Replacement changes the row count, forcing the
//! structural rebuild branch.

use super::*;

use crate::builder::bind_scalar_groups;
use crate::heuristic::r#move::ScalarMoveUnion;
use crate::heuristic::selector::move_selector::MoveCursor;
use crate::planning::{ScalarGroup, ScalarTarget};
use std::any::TypeId;

#[derive(Clone, Debug)]
struct PoolPlan {
    values: Vec<Option<usize>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for PoolPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn pool_group() -> crate::builder::ScalarGroupBinding<PoolPlan> {
    let slot = ScalarVariableSlot::new(
        0,
        0,
        "Row",
        |plan: &PoolPlan| plan.values.len(),
        "value",
        |plan, entity, _| plan.values[entity],
        |plan, entity, _, value| plan.values[entity] = value,
        ValueSource::CountableRange { from: 0, to: 3 },
        true,
    );
    let mut group = bind_scalar_groups(
        vec![ScalarGroup::assignment(
            "pool_group",
            ScalarTarget::from_descriptor_index(0, "value"),
        )],
        &[slot],
    )
    .into_iter()
    .next()
    .expect("one binding");
    group
        .assignment_mut()
        .expect("assignment binding")
        .target_mut()
        .set_construction_binding_index(0);
    group
}

/// Every candidate's (entity, to_value) rows in pull order.
fn pooled_stream<S: PlanningSolution>(
    cursor: &mut crate::builder::selector::GroupedScalarCursor<'_, S>,
    solution: &S,
) -> Vec<Vec<(usize, Option<usize>)>> {
    let mut stream = Vec::new();
    while let Some(id) = cursor.next_candidate(solution) {
        let mov = cursor.take_candidate(id);
        let rows = match &mov {
            ScalarMoveUnion::CompoundScalar(compound) => compound
                .edits()
                .iter()
                .map(|edit| (edit.entity_index, edit.to_value))
                .collect(),
            other => {
                let _ = other;
                panic!("the pooled assignment stream only offers compound scalar moves")
            }
        };
        stream.push(rows);
    }
    stream
}

#[test]
fn pooled_assignment_base_matches_rebuild_across_random_sequences() {
    let group = pool_group();
    let crate::builder::ScalarGroupBindingKind::Assignment(ref binding) = group.kind else {
        panic!("expected assignment binding");
    };
    let selector =
        crate::builder::selector::GroupedScalarSelector::new(group.clone(), None, Some(8), false);
    let mut director = ScoreDirector::simple(
        PoolPlan {
            values: vec![None; 7],
            score: None,
        },
        SolutionDescriptor::new("PoolPlan", TypeId::of::<PoolPlan>()),
        |_, _| 1,
    );

    // Solve-owned slot + deterministic xorshift.
    let mut pool: Option<Box<ScalarAssignmentState>> = None;
    let mut draw: u64 = 0x9E37_79B9_7F4A_7C15;

    for step in 0usize..160 {
        draw = draw
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1_442_695_040_888_963_407);
        let op = (draw >> 33) % 4;
        if op == 0 {
            // Wholesale replacement (partition-merge analogue): change the row
            // count so the structural rebuild branch fires.
            let replacement_rows = ((draw >> 20) % 5) as usize + 5;
            director.working_solution_mut().values = vec![None; replacement_rows];
            pool = None;
        } else if op == 1 {
            // Committed writer: 1-3 random rows change value.
            draw = draw.wrapping_mul(6364136223846793005).wrapping_add(1);
            let touches = ((draw >> 40) % 3) + 1;
            for _ in 0..touches {
                draw = draw.wrapping_mul(6364136223846793005).wrapping_add(1);
                let entity = (draw as usize) % director.working_solution().values.len();
                let next = match (draw >> 24) % 4 {
                    0 => None,
                    value => Some((value - 1) as usize),
                };
                director.working_solution_mut().values[entity] = next;
                draw = draw.wrapping_mul(6364136223846793005).wrapping_add(1);
            }
        } else {
            // Trial-less step: open the pooled cursor, drain its stream
            // exactly, compare to a rebuilt stream; then return the pool.
            let context = crate::heuristic::selector::move_selector::MoveStreamContext::new(
                step as u64,
                0xC0A1_E5CE_AAA0_0002 ^ (step as u64 + 1),
                None,
            );
            let mut pooled_cursor =
                selector.open_cursor_with_retained_state(&mut pool, &director, context);
            let pooled_rows = pooled_stream(&mut pooled_cursor, director.working_solution());
            pool = pooled_cursor
                .into_pooled_assignment()
                .filter(|state| state.row_count() == director.working_solution().values.len())
                .map(Box::new);

            let rebuilt = ScalarAssignmentState::new(binding, director.working_solution());
            if let Some(state) = &pool {
                assert_eq!(
                    state.index_snapshot(),
                    rebuilt.index_snapshot(),
                    "step {step}: maintained base diverged from a fresh rebuild"
                );
            } else {
                // Structural branch replaced the pool; the next step rebuilds
                // from the committed solution.
                pool = Some(Box::new(rebuilt));
            }
            assert!(
                !pooled_rows.is_empty() || step == 0,
                "step {step}: the assignment stream must keep offering candidates"
            );
            let rebuilt_stream = {
                let empty: Option<Box<ScalarAssignmentState>> = None;
                let mut fresh_slot = empty;
                let mut fresh_cursor =
                    selector.open_cursor_with_retained_state(&mut fresh_slot, &director, context);
                let rows = pooled_stream(&mut fresh_cursor, director.working_solution());
                fresh_cursor
                    .into_pooled_assignment()
                    .filter(|state| state.row_count() == director.working_solution().values.len());
                rows
            };
            assert_eq!(
                pooled_rows, rebuilt_stream,
                "step {step}: pooled stream diverged from the rebuilt stream"
            );
        }
    }
}
