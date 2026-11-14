//! Constraint matches name each entity by its planning id when its type has
//! a `#[planning_id]`: `Display` ids render with `Display`, others with
//! `Debug`, and types without one report none. Every stream family carries
//! the ids, before and after the director initializes its constraints.

#[path = "planning_id_constraint_analysis/mod.rs"]
mod domain;

pub use domain::{define_constraints, Nurse, Roster, Shift, ShiftId, Ward};

use solverforge::prelude::*;
use solverforge::ScoreDirector;

fn roster() -> Roster {
    let shift = |id, day, nurse_idx| Shift {
        id: ShiftId(id),
        day,
        nurse_idx,
    };
    Roster {
        nurses: vec![
            Nurse {
                name: "anna".to_string(),
                index: 0,
                unavailable_day: 5,
            },
            Nurse {
                name: "bruno".to_string(),
                index: 1,
                unavailable_day: 1,
            },
            Nurse {
                name: "carla".to_string(),
                index: 2,
                unavailable_day: 5,
            },
        ],
        wards: vec![
            Ward {
                name: "north".to_string(),
                unstaffed: true,
            },
            Ward {
                name: "south".to_string(),
                unstaffed: false,
            },
        ],
        shifts: vec![
            shift(10, 0, Some(0)),
            shift(11, 0, Some(0)),
            shift(12, 1, Some(1)),
            shift(13, 1, None),
        ],
        score: None,
    }
}

/// A match as its score and the planning ids of its entities.
type MatchIds = (HardSoftScore, Vec<Option<String>>);

/// Each constraint's matches as (score, planning ids of the match's
/// entities), sorted by match.
fn planning_ids(
    director: &ScoreDirector<Roster, impl ConstraintSet<Roster, HardSoftScore>>,
) -> Vec<(String, Vec<MatchIds>)> {
    director
        .constraint_analyses()
        .into_iter()
        .map(|analysis| {
            let total = analysis
                .matches
                .iter()
                .fold(HardSoftScore::ZERO, |sum, detailed| sum + detailed.score);
            assert_eq!(total, analysis.score, "{}", analysis.constraint_ref.name);
            let mut matches = analysis
                .matches
                .iter()
                .map(|detailed| {
                    let ids = detailed
                        .justification
                        .entities
                        .iter()
                        .map(|entity| entity.planning_id.clone())
                        .collect::<Vec<_>>();
                    (detailed.score, ids)
                })
                .collect::<Vec<_>>();
            matches.sort_by(|left, right| left.1.cmp(&right.1));
            (analysis.constraint_ref.name.to_string(), matches)
        })
        .collect()
}

fn id(text: &str) -> Option<String> {
    Some(text.to_string())
}

#[test]
fn every_stream_family_reports_planning_ids() {
    let mut director = ScoreDirector::new(roster(), define_constraints());
    let before = planning_ids(&director);
    director.calculate_score();
    let after = planning_ids(&director);
    assert_eq!(before, after, "evaluation and retained rows agree");

    let expected = vec![
        (
            "Unassigned shift".to_string(),
            vec![(HardSoftScore::of_hard(-1), vec![id("ShiftId(13)")])],
        ),
        (
            "Shift on an unavailable day".to_string(),
            vec![(
                HardSoftScore::of_hard(-1),
                vec![id("ShiftId(12)"), id("bruno")],
            )],
        ),
        (
            "Shifts per nurse beyond one".to_string(),
            vec![(
                HardSoftScore::of_soft(-1),
                vec![id("ShiftId(10)"), id("ShiftId(11)")],
            )],
        ),
        (
            "Shifts per nurse".to_string(),
            vec![
                (
                    HardSoftScore::of_soft(2),
                    vec![id("anna"), id("ShiftId(10)"), id("ShiftId(11)")],
                ),
                (
                    HardSoftScore::of_soft(1),
                    vec![id("bruno"), id("ShiftId(12)")],
                ),
            ],
        ),
        (
            "Projected shifts per nurse beyond one".to_string(),
            vec![(
                HardSoftScore::of_soft(-1),
                vec![id("ShiftId(10)"), id("ShiftId(11)")],
            )],
        ),
        (
            "Unstaffed ward".to_string(),
            vec![(HardSoftScore::of_soft(-1), vec![None])],
        ),
    ];
    assert_eq!(after, expected);
}

#[test]
fn entities_without_a_planning_id_still_report_their_display() {
    let director = ScoreDirector::new(roster(), define_constraints());
    let analyses = director.constraint_analyses();
    let ward = &analyses
        .iter()
        .find(|analysis| analysis.constraint_ref.name == "Unstaffed ward")
        .expect("ward constraint")
        .matches[0]
        .justification
        .entities[0];
    assert_eq!(ward.planning_id, None);
    assert!(ward.display.contains("north"), "{}", ward.display);
}
