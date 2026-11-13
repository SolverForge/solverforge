//! Grouped constraints report their matches in constraint analysis: one
//! match per group with a non-zero weight, justified by the group's shifts,
//! for every terminal on a compiler-shared grouped node.

#[path = "grouped_constraint_analysis/mod.rs"]
mod domain;

pub use domain::{define_constraints, Nurse, Roster, Shift};

use solverforge::prelude::*;
use solverforge::ScoreDirector;

fn roster() -> Roster {
    Roster {
        nurses: vec![Nurse { id: 0 }, Nurse { id: 1 }],
        shifts: vec![
            Shift {
                id: 0,
                day: 0,
                nurse_idx: Some(0),
            },
            Shift {
                id: 1,
                day: 0,
                nurse_idx: Some(0),
            },
            Shift {
                id: 2,
                day: 1,
                nurse_idx: Some(1),
            },
        ],
        score: None,
    }
}

#[test]
fn grouped_constraints_justify_their_score_like_the_equivalent_join() {
    let mut director = ScoreDirector::new(roster(), define_constraints());
    assert_eq!(director.calculate_score(), HardSoftScore::of(-2, 3));

    let analyses = director.constraint_analyses();
    assert_eq!(analyses.len(), 3);
    for analysis in &analyses {
        let total = analysis
            .matches
            .iter()
            .fold(HardSoftScore::ZERO, |sum, detailed| sum + detailed.score);
        assert_eq!(total, analysis.score, "{}", analysis.constraint_ref.name);
    }

    let join = &analyses[0];
    let grouped = &analyses[1];
    assert_eq!(join.matches.len(), 1);
    assert_eq!(grouped.score, HardSoftScore::of_hard(-1));
    assert_eq!(grouped.matches.len(), 1, "only nurse 0 costs score");
    let shifts = grouped.matches[0]
        .justification
        .entities
        .iter()
        .map(|entity| entity.display.clone())
        .collect::<Vec<_>>();
    assert_eq!(shifts.len(), 2);
    assert!(shifts
        .iter()
        .all(|shift| shift.contains("nurse_idx: Some(0)")));

    let assigned = &analyses[2];
    assert_eq!(assigned.matches.len(), 2, "both nurses earn a reward");
}
