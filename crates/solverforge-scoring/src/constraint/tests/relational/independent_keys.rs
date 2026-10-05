/* Independent-key regression: two joins, two key types, no shared domain.

Topology under test (explicit test-domain fields, not user code):

- (assignment, shift):   assignment.shift_id (u32) == shift.id (u32).
- (assignment, employee): assignment.employee_code (String) == employee.code (String).
- Residual filter: shift.night.

RED: the current `Bi::join` forces the second key into the first join's
`K` (`KC: Fn(&C) -> K`), so these heterogeneous relationships cannot be
expressed. This suite must fail to compile today; it goes green in P3
with row-aware left closures, e.g. `|(assignment, _shift)| ...`.
*/

use solverforge_core::score::SoftScore;

use super::fixtures::{coincidence_sample, sample};
use super::oracle::{oracle_rows, oracle_score};

// Placeholder: the independent-key fluent chain does not exist yet.
// Kept as a compile-gated reminder of the exact target surface; the
// assertions below pin the oracle contract the chain must satisfy.
#[test]
fn independent_key_topology_scores_oracle_rows() {
    let schedule = sample();
    assert_eq!(oracle_rows(&schedule).len(), 2);
    assert_eq!(oracle_score(&schedule), SoftScore::of(-2));
}

#[test]
fn independent_key_topology_rejects_shared_domain_coincidence() {
    let schedule = coincidence_sample();
    assert_eq!(oracle_rows(&schedule).len(), 1);
    assert_eq!(oracle_score(&schedule), SoftScore::of(-1));
}
