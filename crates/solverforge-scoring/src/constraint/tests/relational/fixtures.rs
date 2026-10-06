/* Explicit relational fixtures for independent-key join verification.

These types are test-domain stand-ins, not the reporter's model: field
names and weights here are chosen for the regression suite, never inferred
from user code. The two relationships use deliberately disjoint key
domains (`u32` shift ids vs `String` employee codes) so any shared-key
conflation is observable instead of accidentally passing.
*/

use solverforge_core::score::SoftScore;
use solverforge_core::PlanningSolution;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct RelShift {
    pub(super) id: u32,
    pub(super) night: bool,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct RelAssignment {
    pub(super) shift_id: u32,
    pub(super) employee_code: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct RelEmployee {
    pub(super) code: String,
}

#[derive(Clone)]
pub(super) struct RelSchedule {
    pub(super) shifts: Vec<RelShift>,
    pub(super) assignments: Vec<RelAssignment>,
    pub(super) employees: Vec<RelEmployee>,
    pub(super) score: Option<SoftScore>,
}

impl PlanningSolution for RelSchedule {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

pub(super) fn rel_shifts(schedule: &RelSchedule) -> &[RelShift] {
    schedule.shifts.as_slice()
}

pub(super) fn rel_assignments(schedule: &RelSchedule) -> &[RelAssignment] {
    schedule.assignments.as_slice()
}

pub(super) fn rel_employees(schedule: &RelSchedule) -> &[RelEmployee] {
    schedule.employees.as_slice()
}

/* Canonical sample: two independent relationships over disjoint domains.

R1 (u32): assignment.shift_id == shift.id.
R2 (String): assignment.employee_code == employee.code.

Rows (assignment, shift, employee) with the night-shift filter applied:
- (a0, s0, e0): shift 10 night, emp "e0" -> match.
- (a1, s1, e1): shift 11 day, emp "e1" -> filtered out.
- (a2, s0, e0): duplicate of a0's keys, distinct row identity -> match.
*/
pub(super) fn sample() -> RelSchedule {
    RelSchedule {
        shifts: vec![
            RelShift {
                id: 10,
                night: true,
            },
            RelShift {
                id: 11,
                night: false,
            },
        ],
        assignments: vec![
            RelAssignment {
                shift_id: 10,
                employee_code: "e0".to_string(),
            },
            RelAssignment {
                shift_id: 11,
                employee_code: "e1".to_string(),
            },
            RelAssignment {
                shift_id: 10,
                employee_code: "e0".to_string(),
            },
        ],
        employees: vec![
            RelEmployee {
                code: "e0".to_string(),
            },
            RelEmployee {
                code: "e1".to_string(),
            },
        ],
        score: None,
    }
}

/* Coincidence sample: shift ids rendered as strings collide with employee
codes, so a shared string key domain merges the two relationships and
admits a spurious (a0, s0, e1) row where "10" is treated as one domain.
The oracle rejects it; only (a0, s0, e0) is a real match.
*/
pub(super) fn coincidence_sample() -> RelSchedule {
    RelSchedule {
        shifts: vec![RelShift {
            id: 10,
            night: true,
        }],
        assignments: vec![RelAssignment {
            shift_id: 10,
            employee_code: "e0".to_string(),
        }],
        employees: vec![
            RelEmployee {
                code: "e0".to_string(),
            },
            RelEmployee {
                code: "10".to_string(),
            },
        ],
        score: None,
    }
}

// Shared relationship keys for the fixtures' assignment↔shift topology.
pub(super) fn assignment_key(assignment: &RelAssignment) -> u32 {
    assignment.shift_id
}

pub(super) fn shift_key(shift: &RelShift) -> u32 {
    shift.id
}
