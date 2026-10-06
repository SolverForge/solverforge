/* Typed row chains beyond the Penta adapters.

The internal operator tree has no fixed arity ceiling: a 4-source chain
(assignment x shift x employee x shift) built from nested `JoinNode`s must
resolve typed borrowed rows, keep an independent key at each step, and score
without erasing to `dyn` or cloning entities. A later key reads an earlier
binding out of the nested pair row.
*/

use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::operator::{CollectionNode, JoinNode, Operator, Pair};
use crate::stream::relational::Leaf;

use super::fixtures::{
    rel_assignments, rel_employees, rel_shifts, sample, RelAssignment, RelEmployee, RelSchedule,
    RelShift,
};

fn assignment_key_leaf(row: &Leaf<'_, RelAssignment>) -> u32 {
    row.entity.shift_id
}
fn shift_key_leaf(row: &Leaf<'_, RelShift>) -> u32 {
    row.entity.id
}
fn employee_code_leaf(row: &Leaf<'_, RelEmployee>) -> String {
    row.entity.code.clone()
}
// Third-step key reads the assignment binding out of the (assignment, shift) pair.
fn pair_employee_code(row: &Pair<Leaf<'_, RelAssignment>, Leaf<'_, RelShift>>) -> String {
    row.left.entity.employee_code.clone()
}
// Fourth-step key reads the shift binding out of the nested tri row.
fn tri_shift_id(
    row: &Pair<Pair<Leaf<'_, RelAssignment>, Leaf<'_, RelShift>>, Leaf<'_, RelEmployee>>,
) -> u32 {
    row.left.right.entity.id
}

#[test]
fn four_source_typed_chain_resolves_and_counts() {
    let a = CollectionNode::new(
        source(
            rel_assignments as fn(&RelSchedule) -> &[RelAssignment],
            ChangeSource::Descriptor(1),
        ),
        1,
    );
    let s = CollectionNode::new(
        source(
            rel_shifts as fn(&RelSchedule) -> &[RelShift],
            ChangeSource::Descriptor(0),
        ),
        0,
    );
    let e = CollectionNode::new(
        source(
            rel_employees as fn(&RelSchedule) -> &[RelEmployee],
            ChangeSource::Descriptor(2),
        ),
        2,
    );
    let s2 = CollectionNode::new(
        source(
            rel_shifts as fn(&RelSchedule) -> &[RelShift],
            ChangeSource::Descriptor(0),
        ),
        3,
    );

    // (assignment x shift) -> x employee -> x shift, each its own key.
    let pair = JoinNode::new(a, s, equal_bi(assignment_key_leaf, shift_key_leaf));
    let tri = JoinNode::new(pair, e, equal_bi(pair_employee_code, employee_code_leaf));
    let quad = JoinNode::new(tri, s2, equal_bi(tri_shift_id, shift_key_leaf));

    let schedule = sample();
    let mut count = 0usize;
    quad.visit_all(&schedule, &mut |_row| {
        count += 1;
    });
    // Each (assignment x shift x employee) tri row extends with every shift
    // whose id equals the tri row's shift id: a0 and a2 (both shift 10) give
    // one tri row each, and s0 (id 10) and s1 (id 11) are both candidates for
    // the fourth leaf, but only s0 matches id 10. The fourth join pairs each
    // tri row with every matching shift row: s0 and s1 both probe, s0 matches.
    assert_eq!(count, 3);
}
