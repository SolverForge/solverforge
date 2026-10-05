/* Shared fluent-chain helpers: row-view key closures for the canonical
operator-backed `Tri` tests. */

use super::fixtures::{RelAssignment, RelEmployee, RelShift};

pub(super) fn row_pair_employee_code(
    row: &crate::stream::relational::operator::Pair<
        crate::stream::relational::Leaf<'_, RelAssignment>,
        crate::stream::relational::Leaf<'_, RelShift>,
    >,
) -> String {
    row.left.entity.employee_code.clone()
}

pub(super) fn employee_key(employee: &crate::stream::relational::Leaf<'_, RelEmployee>) -> String {
    employee.entity.code.clone()
}
