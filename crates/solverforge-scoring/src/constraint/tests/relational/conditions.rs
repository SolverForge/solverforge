/* P4 RED: row-aware condition constructors for non-equality relationships.

Comparison, overlap, and arbitrary-predicate conditions over the whole
left row plus the new right entity. Each condition must expose its
extractors for index planning (ordered/interval/scan) while `matches`
stays the exact semantic check. Fails until the row-aware constructors
land in `stream::joiner`.
*/

use crate::stream::joiner::{pair_row, Joiner};
use crate::stream::relational::{Concat, Leaf};

use super::fixtures::{RelAssignment, RelEmployee, RelShift};

fn sample_row<'e>(
    assignment: &'e RelAssignment,
    shift: &'e RelShift,
) -> Concat<'e, Leaf<'e, RelAssignment>, RelShift> {
    pair_row(assignment, 0, shift, 0)
}

#[test]
fn row_aware_comparison_matches_with_strict_boundaries() {
    use crate::stream::joiner::less_than_on;
    let assignment = RelAssignment {
        shift_id: 10,
        employee_code: "e0".to_string(),
    };
    let shift = RelShift {
        id: 10,
        night: true,
    };
    let row = sample_row(&assignment, &shift);
    let early = RelEmployee {
        code: "e0".to_string(),
    };
    let _ = (row, early);
    let condition = less_than_on(
        |row: &Concat<Leaf<'_, RelAssignment>, RelShift>| row.left.entity.shift_id,
        |employee: &RelEmployee| employee.code.len() as u32,
    );
    let row2 = sample_row(&assignment, &shift);
    assert!(condition.matches(
        &row2,
        &RelEmployee {
            code: "e0-extra-long-code".to_string(),
        }
    ));
    assert!(!condition.matches(
        &row2,
        &RelEmployee {
            code: "".to_string(),
        }
    ));
}

#[test]
fn row_aware_overlap_keeps_half_open_endpoint_semantics() {
    use crate::stream::joiner::overlap_on;
    let assignment = RelAssignment {
        shift_id: 10,
        employee_code: "e0".to_string(),
    };
    let shift = RelShift {
        id: 10,
        night: true,
    };
    let row = sample_row(&assignment, &shift);
    let condition = overlap_on(
        |row: &Concat<Leaf<'_, RelAssignment>, RelShift>| row.right.entity.id as i64,
        |row: &Concat<Leaf<'_, RelAssignment>, RelShift>| row.right.entity.id as i64 + 10,
        |employee: &RelEmployee| employee.code.len() as i64,
        |employee: &RelEmployee| employee.code.len() as i64 + 5,
    );
    // Row interval [10, 20); employee "e012345678" has len 10 -> [10, 15): overlap.
    assert!(condition.matches(
        &row,
        &RelEmployee {
            code: "e012345678".to_string(),
        }
    ));
    // Empty code -> [0, 5): disjoint.
    assert!(!condition.matches(
        &row,
        &RelEmployee {
            code: "".to_string(),
        }
    ));
}

#[test]
fn planned_strategies_route_indexable_conditions_to_indexes() {
    use crate::stream::joiner::Strategy;
    use crate::stream::joiner::{equal_on, filtering_on, less_than_on, overlap_on, plan_strategy};

    let eq = equal_on(
        |row: &Concat<Leaf<'_, RelAssignment>, RelShift>| row.left.entity.shift_id,
        |shift: &RelShift| shift.id,
    );
    assert_eq!(plan_strategy(&eq), Strategy::EquiHash);

    let cmp = less_than_on(
        |row: &Concat<Leaf<'_, RelAssignment>, RelShift>| row.left.entity.shift_id,
        |shift: &RelShift| shift.id,
    );
    assert_eq!(plan_strategy(&cmp), Strategy::OrderedScan);

    let ov = overlap_on(
        |row: &Concat<Leaf<'_, RelAssignment>, RelShift>| row.right.entity.id as i64,
        |row: &Concat<Leaf<'_, RelAssignment>, RelShift>| row.right.entity.id as i64 + 10,
        |employee: &RelEmployee| employee.code.len() as i64,
        |employee: &RelEmployee| employee.code.len() as i64 + 5,
    );
    assert_eq!(plan_strategy(&ov), Strategy::IntervalNarrow);

    let pred = filtering_on(
        |row: &Concat<Leaf<'_, RelAssignment>, RelShift>, employee: &RelEmployee| {
            row.left.entity.employee_code == employee.code
        },
    );
    assert_eq!(plan_strategy(&pred), Strategy::Scan);
}

#[test]
fn and_normalizes_heterogeneous_conditions_with_combined_residual() {
    use crate::stream::joiner::{equal_on, less_than_on, plan_strategy, Joiner, Strategy};

    fn eq_key(row: &Concat<Leaf<'_, RelAssignment>, RelShift>) -> u32 {
        row.left.entity.shift_id
    }
    fn eq_right(shift: &RelShift) -> u32 {
        shift.id
    }
    fn cmp_key(row: &Concat<Leaf<'_, RelAssignment>, RelShift>) -> i64 {
        row.left.entity.shift_id as i64
    }
    fn cmp_right(shift: &RelShift) -> i64 {
        shift.id as i64 + 50
    }

    // Reordering AND conditions changes neither strategy nor matches.
    assert_eq!(
        plan_strategy(&equal_on(eq_key, eq_right).and(less_than_on(cmp_key, cmp_right))),
        Strategy::EquiHash
    );
    assert_eq!(
        plan_strategy(&less_than_on(cmp_key, cmp_right).and(equal_on(eq_key, eq_right))),
        Strategy::EquiHash
    );

    let assignment = RelAssignment {
        shift_id: 10,
        employee_code: "e0".to_string(),
    };
    let shift = RelShift {
        id: 10,
        night: true,
    };
    let row = sample_row(&assignment, &shift);
    // eq: 10 == 10 hit; cmp: 10 < 60 hit.
    assert!(equal_on(eq_key, eq_right)
        .and(less_than_on(cmp_key, cmp_right))
        .matches(&row, &shift));
    let low = RelShift { id: 0, night: true };
    // eq: 10 == 0 miss (cmp alone would hit: 10 < 50).
    assert!(!equal_on(eq_key, eq_right)
        .and(less_than_on(cmp_key, cmp_right))
        .matches(&row, &low));
}

#[test]
fn row_aware_predicate_matches_arbitrary_typed_conditions() {
    use crate::stream::joiner::filtering_on;
    let assignment = RelAssignment {
        shift_id: 10,
        employee_code: "e0".to_string(),
    };
    let shift = RelShift {
        id: 10,
        night: true,
    };
    let row = sample_row(&assignment, &shift);
    let condition = filtering_on(
        |row: &Concat<Leaf<'_, RelAssignment>, RelShift>, employee: &RelEmployee| {
            row.left.entity.employee_code == employee.code && row.right.entity.night
        },
    );
    assert!(condition.matches(
        &row,
        &RelEmployee {
            code: "e0".to_string(),
        }
    ));
    assert!(!condition.matches(
        &row,
        &RelEmployee {
            code: "e1".to_string(),
        }
    ));
}
