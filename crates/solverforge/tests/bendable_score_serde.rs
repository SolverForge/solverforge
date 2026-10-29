//! With the facade `serde` feature, a solution holding a `BendableScore`
//! derives serde like one holding any other score type.
#![cfg(feature = "serde")]

use serde::{Deserialize, Serialize};
use solverforge::BendableScore;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Schedule {
    shifts: Vec<u32>,
    score: Option<BendableScore<2, 3>>,
}

#[test]
fn solution_with_bendable_score_round_trips() {
    let schedule = Schedule {
        shifts: vec![1, 2],
        score: Some(BendableScore::of([0, -1], [-5, 0, 7])),
    };
    let text = toml::to_string(&schedule).expect("solution serializes");
    let parsed: Schedule = toml::from_str(&text).expect("solution deserializes");
    assert_eq!(parsed, schedule);

    let rejected =
        toml::from_str::<Schedule>("shifts = []\n[score]\nhard = [0]\nsoft = [0, 0, 0]\n")
            .expect_err("one hard level is not two");
    assert!(
        rejected.to_string().contains("invalid length 1"),
        "{rejected}"
    );
}
