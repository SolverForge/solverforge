use solverforge::prelude::*;
use solverforge::stream::collector::count;
use solverforge::stream::{joiner, ConstraintFactory, Projection, ProjectionSink};

use crate::{Nurse, Shift, Ward};

/// Nurses, wards and shifts for the planning-id analysis tests.
#[planning_solution(constraints = "define_constraints")]
pub struct Roster {
    /// Nurses shifts can be assigned to.
    #[problem_fact_collection]
    pub nurses: Vec<Nurse>,

    /// Wards of the hospital.
    #[problem_fact_collection]
    pub wards: Vec<Ward>,

    /// Shifts the solver assigns.
    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    /// The roster's score.
    #[planning_score]
    pub score: Option<HardSoftScore>,
}

/// Projects each assigned shift to its nurse index.
pub struct AssignedNurse;

impl Projection<Shift> for AssignedNurse {
    type Out = usize;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, shift: &Shift, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        if let Some(nurse) = shift.nurse_idx {
            sink.emit(nurse);
        }
    }
}

fn factory() -> ConstraintFactory<Roster, HardSoftScore> {
    ConstraintFactory::new()
}

/// One constraint per stream family, in this order: filter, join,
/// group_by, complement, projection, and a filter over a fact without a
/// planning id.
#[solverforge_constraints]
pub fn define_constraints() -> impl ConstraintSet<Roster, HardSoftScore> {
    let unassigned = factory()
        .for_each(Roster::shifts())
        .filter(|shift: &Shift| shift.nurse_idx.is_none())
        .penalize(HardSoftScore::ONE_HARD)
        .named("Unassigned shift");

    let unavailable = factory()
        .for_each(Roster::shifts())
        .join((
            Roster::nurses(),
            joiner::equal_bi(
                |shift: &Shift| shift.nurse_idx,
                |nurse: &Nurse| Some(nurse.index),
            ),
        ))
        .filter(|shift: &Shift, nurse: &Nurse| shift.day == nurse.unavailable_day)
        .penalize(HardSoftScore::ONE_HARD)
        .named("Shift on an unavailable day");

    let per_nurse = factory()
        .for_each(Roster::shifts())
        .group_by(
            |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
            count(),
        )
        .penalize(|nurse: &usize, shifts: &usize| {
            HardSoftScore::of_soft(if *nurse == usize::MAX {
                0
            } else {
                shifts.saturating_sub(1) as i64
            })
        })
        .named("Shifts per nurse beyond one");

    let idle = factory()
        .for_each(Roster::shifts())
        .group_by(
            |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
            count(),
        )
        .complement(Roster::nurses(), |nurse: &Nurse| nurse.index, |_nurse| 0usize)
        .reward(|_nurse: &usize, shifts: &usize| HardSoftScore::of_soft(*shifts as i64))
        .named("Shifts per nurse");

    let projected = factory()
        .for_each(Roster::shifts())
        .project(AssignedNurse)
        .group_by(|nurse: &usize| *nurse, count())
        .penalize(|_nurse: &usize, shifts: &usize| {
            HardSoftScore::of_soft(shifts.saturating_sub(1) as i64)
        })
        .named("Projected shifts per nurse beyond one");

    let unstaffed = factory()
        .for_each(Roster::wards())
        .filter(|ward: &Ward| ward.unstaffed)
        .penalize(HardSoftScore::ONE_SOFT)
        .named("Unstaffed ward");

    (unassigned, unavailable, per_nurse, idle, projected, unstaffed)
}
