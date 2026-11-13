use solverforge::prelude::*;
use solverforge::stream::collector::count;
use solverforge::stream::ConstraintFactory;

use crate::{Nurse, Shift};

/// Nurses and shifts for the grouped analysis tests.
#[planning_solution(constraints = "define_constraints")]
pub struct Roster {
    /// Nurses shifts can be assigned to.
    #[problem_fact_collection]
    pub nurses: Vec<Nurse>,

    /// Shifts the solver assigns.
    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    /// The roster's score.
    #[planning_score]
    pub score: Option<HardSoftScore>,
}

/// The same rule written as a join and as a group, plus a second terminal
/// on the shared grouped node.
#[solverforge_constraints]
pub fn define_constraints() -> impl ConstraintSet<Roster, HardSoftScore> {
    let same_day_join = ConstraintFactory::<Roster, HardSoftScore>::new()
        .for_each(Roster::shifts())
        .join((
            ConstraintFactory::<Roster, HardSoftScore>::new().for_each(Roster::shifts()),
            |left: &Shift, right: &Shift| {
                left.id < right.id
                    && left.day == right.day
                    && left.nurse_idx.is_some()
                    && left.nurse_idx == right.nurse_idx
            },
        ))
        .penalize(HardSoftScore::ONE_HARD)
        .named("Two shifts on one day (join)");

    let per_nurse = ConstraintFactory::<Roster, HardSoftScore>::new()
        .for_each(Roster::shifts())
        .group_by(
            |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
            count(),
        );

    (
        same_day_join,
        per_nurse
            .penalize(|nurse: &usize, shifts: &usize| {
                HardSoftScore::of_hard(if *nurse == usize::MAX {
                    0
                } else {
                    shifts.saturating_sub(1) as i64
                })
            })
            .named("Shifts per nurse beyond one (group_by)"),
        per_nurse
            .reward(|nurse: &usize, shifts: &usize| {
                HardSoftScore::of_soft(if *nurse == usize::MAX { 0 } else { *shifts as i64 })
            })
            .named("Assigned shifts (group_by)"),
    )
}
