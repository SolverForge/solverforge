//! Native scalar swap legality against every value source, without
//! materializing the value range at cursor open.

use std::cell::Cell;

use super::*;
use crate::builder::{RuntimeScalarSlot, ScalarVariableSlot, ValueSource};
use crate::heuristic::selector::move_selector::{MoveCursor, MoveStreamContext};
use crate::heuristic::selector::scalar_neighborhood::spec::{
    RuntimeScalarRecipe, ScalarNeighborhoodSpec,
};
use crate::heuristic::selector::scalar_neighborhood::ScalarNeighborhoodLeaf;

thread_local! {
    static COUNT_CALLS: Cell<usize> = const { Cell::new(0) };
}

fn row_count(plan: &Plan) -> usize {
    plan.values.len()
}

fn row_get(plan: &Plan, entity_index: usize, _variable_index: usize) -> Option<usize> {
    plan.values[entity_index]
}

fn row_set(plan: &mut Plan, entity_index: usize, _variable_index: usize, value: Option<usize>) {
    plan.values[entity_index] = value;
}

fn candidate_count(plan: &Plan, _provider_index: usize) -> usize {
    COUNT_CALLS.with(|calls| calls.set(calls.get() + 1));
    plan.candidates.len()
}

fn huge_count(_plan: &Plan, _provider_index: usize) -> usize {
    COUNT_CALLS.with(|calls| calls.set(calls.get() + 1));
    1 << 40
}

/// Each entity accepts a different suffix of the candidate list.
fn candidate_suffix(plan: &Plan, entity_index: usize, _variable_index: usize) -> &[usize] {
    &plan.candidates[entity_index.min(plan.candidates.len())..]
}

fn static_slot(
    value_source: ValueSource<Plan>,
    allows_unassigned: bool,
) -> RuntimeScalarSlot<Plan> {
    RuntimeScalarSlot::Static(ScalarVariableSlot::new(
        0,
        0,
        "Row",
        row_count,
        "value",
        row_get,
        row_set,
        value_source,
        allows_unassigned,
    ))
}

fn swap_pairs(slot: RuntimeScalarSlot<Plan>, plan: Plan) -> Vec<(usize, usize)> {
    let director = director(plan);
    let leaf = ScalarNeighborhoodLeaf::from_spec(ScalarNeighborhoodSpec::Swap, slot, Some(23))
        .expect("static slot supplies the swap capability");
    let mut state = leaf.new_stream_state();
    let mut cursor =
        leaf.open_cursor_with_stream_state(&mut state, &director, MoveStreamContext::default());
    let mut pairs = Vec::new();
    while let Some(mov) = cursor.next_owned_candidate(director.working_solution()) {
        match mov.into_recipe() {
            RuntimeScalarRecipe::Swap {
                left_entity_index,
                right_entity_index,
                ..
            } => pairs.push((left_entity_index, right_entity_index)),
            _ => panic!("the swap leaf emits only swap recipes"),
        }
    }
    pairs
}

/// The legality rule the eager per-entity snapshot applied: membership in the
/// materialized value list, `None` per the slot, any assigned value for an
/// empty source.
fn legal_by_materialized_values(
    slot: &RuntimeScalarSlot<Plan>,
    plan: &Plan,
    entity_index: usize,
    value: Option<usize>,
) -> bool {
    let RuntimeScalarSlot::Static(slot) = slot else {
        unreachable!("these tests drive native slots");
    };
    if matches!(slot.value_source, ValueSource::Empty) {
        return value.is_some();
    }
    match value {
        Some(value) => slot.values_for_entity(plan, entity_index).contains(&value),
        None => slot.allows_unassigned,
    }
}

fn expected_pairs(slot: &RuntimeScalarSlot<Plan>, plan: &Plan) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for left in 0..plan.values.len() {
        for right in (left + 1)..plan.values.len() {
            let (left_value, right_value) = (plan.values[left], plan.values[right]);
            if left_value != right_value
                && legal_by_materialized_values(slot, plan, left, right_value)
                && legal_by_materialized_values(slot, plan, right, left_value)
            {
                pairs.push((left, right));
            }
        }
    }
    pairs
}

fn plan() -> Plan {
    Plan {
        values: vec![
            Some(0),
            Some(3),
            None,
            Some(1),
            Some(7),
            Some(2),
            Some(1),
            None,
        ],
        candidates: vec![0, 1, 2, 3],
        score: None,
    }
}

#[test]
fn swap_legality_matches_the_materialized_value_rule_for_every_source() {
    let sources = [
        ValueSource::Empty,
        ValueSource::CountableRange { from: 1, to: 4 },
        ValueSource::SolutionCount {
            count_fn: candidate_count,
            provider_index: 0,
        },
        ValueSource::EntitySlice {
            values_for_entity: candidate_suffix,
        },
    ];
    for source in sources {
        for allows_unassigned in [false, true] {
            let slot = static_slot(source, allows_unassigned);
            let expected = expected_pairs(&slot, &plan());
            let mut actual = swap_pairs(slot, plan());
            let emitted = actual.len();
            actual.sort_unstable();
            actual.dedup();
            assert_eq!(actual.len(), emitted, "{source:?}: no pair repeats");
            assert_eq!(
                actual, expected,
                "{source:?}, allows_unassigned={allows_unassigned}"
            );
        }
    }
}

#[test]
fn counted_source_is_read_once_at_cursor_open() {
    COUNT_CALLS.with(|calls| calls.set(0));
    let slot = static_slot(
        ValueSource::SolutionCount {
            count_fn: candidate_count,
            provider_index: 0,
        },
        false,
    );
    let director = director(plan());
    let leaf = ScalarNeighborhoodLeaf::from_spec(ScalarNeighborhoodSpec::Swap, slot, Some(23))
        .expect("static slot supplies the swap capability");
    let mut state = leaf.new_stream_state();
    let mut cursor =
        leaf.open_cursor_with_stream_state(&mut state, &director, MoveStreamContext::default());
    assert_eq!(COUNT_CALLS.with(Cell::get), 1, "the count is read at open");
    while cursor
        .next_owned_candidate(director.working_solution())
        .is_some()
    {}
    assert_eq!(
        COUNT_CALLS.with(Cell::get),
        1,
        "enumeration tests against the count read at open"
    );
}

#[test]
fn opening_never_materializes_the_value_range() {
    // A 2^40-value range cannot be collected per entity; the cursor must
    // bound-check against it instead.
    COUNT_CALLS.with(|calls| calls.set(0));
    let slot = static_slot(
        ValueSource::SolutionCount {
            count_fn: huge_count,
            provider_index: 0,
        },
        false,
    );
    let pairs = swap_pairs(slot, plan());
    assert_eq!(COUNT_CALLS.with(Cell::get), 1);
    assert!(!pairs.is_empty());
}
