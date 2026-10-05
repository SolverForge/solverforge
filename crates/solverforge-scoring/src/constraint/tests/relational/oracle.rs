/* Independent nested-loop oracle for the relational fixtures.

Enumerates ordered (assignment, shift, employee) identities with a signed
score per row. Uses only the authored predicates — never production
indexes, key extractors, or traversal. All phases compare production
results against this oracle on (a) accumulated signed delta score,
(b) production full recomputation, (c) oracle score, and (d) row
multiset / match count.
*/

use solverforge_core::score::SoftScore;

use super::fixtures::{rel_assignments, rel_employees, rel_shifts, RelSchedule};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct OracleRow {
    pub(super) assignment_idx: usize,
    pub(super) shift_idx: usize,
    pub(super) employee_idx: usize,
}

pub(super) fn oracle_rows(schedule: &RelSchedule) -> Vec<OracleRow> {
    let shifts = rel_shifts(schedule);
    let assignments = rel_assignments(schedule);
    let employees = rel_employees(schedule);
    let mut rows = Vec::new();
    for (a_idx, assignment) in assignments.iter().enumerate() {
        for (s_idx, shift) in shifts.iter().enumerate() {
            if assignment.shift_id != shift.id {
                continue;
            }
            if !shift.night {
                continue;
            }
            for (e_idx, employee) in employees.iter().enumerate() {
                if assignment.employee_code != employee.code {
                    continue;
                }
                rows.push(OracleRow {
                    assignment_idx: a_idx,
                    shift_idx: s_idx,
                    employee_idx: e_idx,
                });
            }
        }
    }
    rows
}

pub(super) fn oracle_score(schedule: &RelSchedule) -> SoftScore {
    SoftScore::of(-(oracle_rows(schedule).len() as i64))
}

#[cfg(test)]
mod oracle_self_check {
    use super::super::fixtures::{coincidence_sample, sample};
    use super::*;

    #[test]
    fn oracle_matches_hand_authored_rows() {
        let rows = oracle_rows(&sample());
        assert_eq!(
            rows,
            vec![
                OracleRow {
                    assignment_idx: 0,
                    shift_idx: 0,
                    employee_idx: 0,
                },
                OracleRow {
                    assignment_idx: 2,
                    shift_idx: 0,
                    employee_idx: 0,
                },
            ]
        );
        assert_eq!(oracle_score(&sample()), SoftScore::of(-2));
    }

    #[test]
    fn oracle_rejects_shared_domain_coincidence() {
        // Employee "10" collides textually with shift id 10, but the oracle
        // compares each relationship in its own typed domain: only e0 joins.
        let rows = oracle_rows(&coincidence_sample());
        assert_eq!(
            rows,
            vec![OracleRow {
                assignment_idx: 0,
                shift_idx: 0,
                employee_idx: 0,
            }]
        );
    }
}
