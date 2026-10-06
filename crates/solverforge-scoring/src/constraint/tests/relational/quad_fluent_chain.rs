/* Four-source fluent chain: `.join((collection, condition))` four times.

Exercises the second-level fluent extension (`Tri::join` → `Quad`) with an
independent key domain at each step, a later key reading earlier bindings, and
a filter between joins. Production must agree with an independent oracle.
*/

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::{equal_bi, equal_on};
use crate::stream::relational::{operator::Pair, Leaf};
use crate::stream::ConstraintFactory;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Assignment {
    shift_id: u32,
    employee_code: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Shift {
    id: u32,
    night: bool,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Employee {
    code: String,
    team: u32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Team {
    id: u32,
    size: i64,
}

#[derive(Clone)]
struct Schedule {
    assignments: Vec<Assignment>,
    shifts: Vec<Shift>,
    employees: Vec<Employee>,
    teams: Vec<Team>,
}

fn assignments(s: &Schedule) -> &[Assignment] {
    s.assignments.as_slice()
}
fn shifts(s: &Schedule) -> &[Shift] {
    s.shifts.as_slice()
}
fn employees(s: &Schedule) -> &[Employee] {
    s.employees.as_slice()
}
fn teams(s: &Schedule) -> &[Team] {
    s.teams.as_slice()
}

// Four relationships, four independent key domains:
//   1) u32:   assignment.shift_id == shift.id
//   2) String: assignment.employee_code (read off the (assignment, shift) row)
//              == employee.code
//   3) u32:   employee.team (read off the (assignment, shift, employee) row)
//              == team.id
fn four_source_chain() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            assignments as fn(&Schedule) -> &[Assignment],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(
                shifts as fn(&Schedule) -> &[Shift],
                ChangeSource::Descriptor(1),
            ),
            equal_bi(|a: &Assignment| a.shift_id, |s: &Shift| s.id),
        ))
        .filter(|_a: &Assignment, s: &Shift| s.night)
        .join((
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(2),
            ),
            equal_on(
                |row: &Pair<Leaf<'_, Assignment>, Leaf<'_, Shift>>| {
                    row.left.entity.employee_code.clone()
                },
                |e: &Leaf<'_, Employee>| e.entity.code.clone(),
            ),
        ))
        .join((
            source(
                teams as fn(&Schedule) -> &[Team],
                ChangeSource::Descriptor(3),
            ),
            // Left key reads the (assignment, shift) binding out of the tri row.
            equal_on(
                |row: &Pair<Pair<Leaf<'_, Assignment>, Leaf<'_, Shift>>, Leaf<'_, Employee>>| {
                    row.left.left.entity.employee_code.len() as u32
                },
                |t: &Leaf<'_, Team>| t.entity.id,
            ),
        ))
        .penalize(SoftScore::of(1))
        .named("four source chain")
}

#[test]
fn four_source_fluent_chain_matches_oracle() {
    let schedule = Schedule {
        assignments: vec![Assignment {
            shift_id: 10,
            employee_code: "code-of-length-2".to_string(), // len 16
        }],
        shifts: vec![Shift {
            id: 10,
            night: true,
        }],
        employees: vec![Employee {
            code: "code-of-length-2".to_string(),
            team: 7,
        }],
        teams: vec![Team { id: 16, size: 3 }, Team { id: 99, size: 5 }],
    };
    let c = four_source_chain();
    // one (assignment, shift, employee, team id 16) row.
    assert_eq!(c.evaluate(&schedule), SoftScore::of(-1));
    assert_eq!(c.match_count(&schedule), 1);
}

#[test]
fn four_source_fluent_chain_incremental_matches_full_evaluation() {
    let mut schedule = Schedule {
        assignments: vec![
            Assignment {
                shift_id: 10,
                employee_code: "code-of-length-2".to_string(),
            },
            Assignment {
                shift_id: 11,
                employee_code: "code-of-length-2".to_string(),
            },
        ],
        shifts: vec![
            Shift {
                id: 10,
                night: true,
            },
            Shift {
                id: 11,
                night: true,
            },
        ],
        employees: vec![Employee {
            code: "code-of-length-2".to_string(),
            team: 7,
        }],
        teams: vec![Team { id: 16, size: 3 }],
    };
    let mut c = four_source_chain();
    let mut running = c.initialize(&schedule);
    assert_eq!(running, c.evaluate(&schedule));
    assert_eq!(c.match_count(&schedule), 2);

    // Make shift 11 a day shift: its row falls out (the between-join filter).
    running = running + c.on_retract(&schedule, 1, 1);
    schedule.shifts[1].night = false;
    running = running + c.on_insert(&schedule, 1, 1);
    assert_eq!(running, c.evaluate(&schedule));
    assert_eq!(c.match_count(&schedule), 1);

    // Retract the team (remove it from the solution too): no rows match the
    // fourth relationship any more.
    schedule.teams.clear();
    running = running + c.on_retract(&schedule, 0, 3);
    assert_eq!(running, c.evaluate(&schedule));
    assert_eq!(c.match_count(&schedule), 0);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Site {
    id: u32,
    capacity: i64,
}

fn sites(s: &Schedule2) -> &[Site] {
    s.sites.as_slice()
}

#[derive(Clone)]
struct Schedule2 {
    assignments: Vec<Assignment>,
    shifts: Vec<Shift>,
    employees: Vec<Employee>,
    teams: Vec<Team>,
    sites: Vec<Site>,
}

fn assignments2(s: &Schedule2) -> &[Assignment] {
    s.assignments.as_slice()
}
fn shifts2(s: &Schedule2) -> &[Shift] {
    s.shifts.as_slice()
}
fn employees2(s: &Schedule2) -> &[Employee] {
    s.employees.as_slice()
}
fn teams2(s: &Schedule2) -> &[Team] {
    s.teams.as_slice()
}

// Five independent relationships: u32 -> String -> u32 -> i64.
fn five_source_chain() -> impl IncrementalConstraint<Schedule2, SoftScore> {
    ConstraintFactory::<Schedule2, SoftScore>::new()
        .for_each(source(
            assignments2 as fn(&Schedule2) -> &[Assignment],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(
                shifts2 as fn(&Schedule2) -> &[Shift],
                ChangeSource::Descriptor(1),
            ),
            equal_bi(|a: &Assignment| a.shift_id, |s: &Shift| s.id),
        ))
        .join((
            source(
                employees2 as fn(&Schedule2) -> &[Employee],
                ChangeSource::Descriptor(2),
            ),
            equal_on(
                |row: &Pair<Leaf<'_, Assignment>, Leaf<'_, Shift>>| {
                    row.left.entity.employee_code.clone()
                },
                |e: &Leaf<'_, Employee>| e.entity.code.clone(),
            ),
        ))
        .join((
            source(
                teams2 as fn(&Schedule2) -> &[Team],
                ChangeSource::Descriptor(3),
            ),
            equal_on(
                |row: &Pair<Pair<Leaf<'_, Assignment>, Leaf<'_, Shift>>, Leaf<'_, Employee>>| {
                    row.left.left.entity.employee_code.len() as u32
                },
                |t: &Leaf<'_, Team>| t.entity.id,
            ),
        ))
        .join((
            source(
                sites as fn(&Schedule2) -> &[Site],
                ChangeSource::Descriptor(4),
            ),
            equal_on(
                |row: &Pair<
                    Pair<Pair<Leaf<'_, Assignment>, Leaf<'_, Shift>>, Leaf<'_, Employee>>,
                    Leaf<'_, Team>,
                >| row.left.right.entity.team as i64,
                |s: &Leaf<'_, Site>| s.entity.capacity,
            ),
        ))
        .penalize(SoftScore::of(1))
        .named("five source chain")
}

#[test]
fn five_source_fluent_chain_matches_oracle() {
    let schedule = Schedule2 {
        assignments: vec![Assignment {
            shift_id: 10,
            employee_code: "abcd".to_string(), // len 4
        }],
        shifts: vec![Shift {
            id: 10,
            night: true,
        }],
        employees: vec![Employee {
            code: "abcd".to_string(),
            team: 7,
        }],
        teams: vec![Team { id: 4, size: 3 }],
        sites: vec![
            Site { id: 1, capacity: 7 },
            Site {
                id: 2,
                capacity: 99,
            },
        ],
    };
    let c = five_source_chain();
    // One row: (assignment, shift, employee, team id==len 4, site capacity==team 7).
    assert_eq!(c.evaluate(&schedule), SoftScore::of(-1));
    assert_eq!(c.match_count(&schedule), 1);
}
