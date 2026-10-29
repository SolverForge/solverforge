//! Every stream family reads its rows through the accessors
//! `#[planning_solution]` generates, and the incremental score stays equal to
//! a from-scratch score through changes to every entity class.

#[path = "model_sources/domain/mod.rs"]
mod domain;

use domain::{Employee, Plan, Shift, Visit};
use solverforge::prelude::*;
use solverforge::stream::{joiner, ConstraintFactory};

const SHIFTS: usize = 0;
const VISITS: usize = 1;

struct VisitHours;

impl Projection<Visit> for VisitHours {
    type Out = (usize, i64);
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, visit: &Visit, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        if let Some(employee) = visit.employee_idx {
            sink.emit((employee, visit.hours));
        }
    }
}

fn factory() -> ConstraintFactory<Plan, HardSoftScore> {
    ConstraintFactory::new()
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        // Unary over an entity source.
        factory()
            .for_each(Plan::shifts())
            .filter(|shift: &Shift| shift.employee_idx.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned shift"),
        // Keyed cross join of an entity source with a fact source.
        factory()
            .for_each(Plan::shifts())
            .join((
                Plan::employees(),
                joiner::equal_bi(
                    |shift: &Shift| shift.employee_idx,
                    |employee: &Employee| Some(employee.id),
                ),
            ))
            .filter(|shift: &Shift, employee: &Employee| shift.day == employee.day_off)
            .penalize(HardSoftScore::ONE_HARD)
            .named("shift on day off"),
        // Existence across two entity sources.
        factory()
            .for_each(Plan::shifts())
            .if_exists((
                factory().for_each(Plan::visits()),
                joiner::equal_bi(
                    |shift: &Shift| shift.employee_idx,
                    |visit: &Visit| visit.employee_idx,
                ),
            ))
            .penalize(HardSoftScore::ONE_SOFT)
            .named("shift and visit for one employee"),
        // Balance over an entity source.
        factory()
            .for_each(Plan::shifts())
            .balance(|shift: &Shift| shift.employee_idx)
            .penalize(HardSoftScore::ONE_SOFT)
            .named("balanced shifts"),
        // Projection of an entity source, grouped.
        factory()
            .for_each(Plan::visits())
            .project(VisitHours)
            .group_by(|row: &(usize, i64)| row.0, sum(|row: &(usize, i64)| row.1))
            .penalize(|_: &usize, hours: &i64| HardSoftScore::of_soft((*hours - 4).max(0)))
            .named("visit overtime"),
    )
}

fn plan() -> Plan {
    Plan {
        employees: (0..3).map(|id| Employee { id, day_off: id }).collect(),
        shifts: (0..5)
            .map(|id| Shift {
                id,
                day: id % 3,
                employee_idx: None,
            })
            .collect(),
        visits: (0..4)
            .map(|id| Visit {
                id,
                hours: id as i64 + 2,
                employee_idx: None,
            })
            .collect(),
        score: None,
    }
}

fn from_scratch(solution: &Plan) -> HardSoftScore {
    ScoreDirector::new(solution.clone(), constraints()).calculate_score()
}

#[test]
fn every_family_tracks_changes_on_every_entity_class() {
    let mut director = ScoreDirector::new(plan(), constraints());
    assert_eq!(
        director.calculate_score(),
        from_scratch(director.working_solution())
    );

    for step in 0..30usize {
        let employee = (step % 4 < 3).then_some(step % 3);
        let (descriptor, entity) = if step % 2 == 0 {
            (SHIFTS, step % 5)
        } else {
            (VISITS, step % 4)
        };
        director.before_variable_changed(descriptor, entity);
        let solution = director.working_solution_mut();
        match descriptor {
            SHIFTS => solution.shifts[entity].employee_idx = employee,
            _ => solution.visits[entity].employee_idx = employee,
        }
        director.after_variable_changed(descriptor, entity);

        assert_eq!(
            director.calculate_score(),
            from_scratch(director.working_solution()),
            "step {step}: descriptor {descriptor}, entity {entity} -> {employee:?}"
        );
    }
}
