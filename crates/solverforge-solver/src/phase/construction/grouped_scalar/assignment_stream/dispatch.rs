use super::*;

pub(super) fn next_family_move<S, ShouldStop>(
    cursor: &mut AssignmentFamilyCursor,
    index: Option<&AssignedValueSequenceIndex>,
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &mut ScalarAssignmentState,
    should_stop: &mut ShouldStop,
) -> Option<CompoundScalarMove<S>>
where
    S: PlanningSolution,
    ShouldStop: FnMut() -> bool,
{
    if should_stop() {
        return None;
    }
    match cursor {
        AssignmentFamilyCursor::EntityValues(entity_cursor) => {
            let candidate = entity_cursor.next(group, solution, state, should_stop);
            if candidate.is_none() && !should_stop() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::Capacity(capacity_cursor) => {
            let candidate = capacity_cursor.next(group, solution, state, should_stop);
            if candidate.is_none() && !should_stop() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::OptionalAdjustment(optional_cursor) => {
            let candidate = optional_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::PairWindow(pair_cursor) => {
            let candidate = pair_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::CycleWindow(cycle_cursor) => {
            let candidate = cycle_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueWindow(value_window_cursor) => {
            let candidate = value_window_cursor.next(
                index.expect("value-family open must initialize the index"),
                group,
                solution,
                state,
            );
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueLongWindow(long_window_cursor) => {
            let candidate = long_window_cursor.next(
                index.expect("value-family open must initialize the index"),
                group,
                solution,
                state,
            );
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueRunGapSwap(run_gap_cursor) => {
            let candidate = run_gap_cursor.next(
                index.expect("value-family open must initialize the index"),
                group,
                solution,
                state,
            );
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueRunRelease(run_release_cursor) => {
            let candidate = run_release_cursor.next(
                index.expect("value-family open must initialize the index"),
                group,
                solution,
                state,
            );
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueWindowCycle(value_window_cycle_cursor) => {
            let candidate = value_window_cycle_cursor.next(
                index.expect("value-family open must initialize the index"),
                group,
                solution,
                state,
            );
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueBlockReassignment(block_cursor) => {
            let candidate = block_cursor.next(
                index.expect("value-family open must initialize the index"),
                group,
                solution,
                state,
            );
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::Empty => None,
    }
}
