use crate::score::{BendableScore, HardSoftScore};

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Holder {
    score: BendableScore<2, 3>,
}

#[test]
fn bendable_score_round_trips_with_the_derived_wire_shape() {
    let holder = Holder {
        score: BendableScore::of([-1, -2], [-10, 20, i64::MIN]),
    };
    let text = toml::to_string(&holder).expect("score serializes");
    assert_eq!(
        text,
        "[score]\nhard = [-1, -2]\nsoft = [-10, 20, -9223372036854775808]\n"
    );
    let parsed: Holder = toml::from_str(&text).expect("score deserializes");
    assert_eq!(parsed, holder);
}

#[test]
fn bendable_score_uses_the_same_field_names_as_the_derived_scores() {
    #[derive(serde::Serialize)]
    struct Pair {
        bendable: BendableScore<1, 1>,
        hard_soft: HardSoftScore,
    }
    let text = toml::to_string(&Pair {
        bendable: BendableScore::of([-3], [4]),
        hard_soft: HardSoftScore::of(-3, 4),
    })
    .expect("scores serialize");
    assert!(
        text.contains("[bendable]\nhard = [-3]\nsoft = [4]\n"),
        "{text}"
    );
    assert!(text.contains("[hard_soft]\n"), "{text}");
}

#[test]
fn bendable_score_rejects_a_wrong_number_of_levels() {
    for (input, expected) in [
        (
            "[score]\nhard = [0]\nsoft = [0, 0, 0]\n",
            "invalid length 1, expected a list of exactly 2 score levels",
        ),
        (
            "[score]\nhard = [0, 0]\nsoft = [0, 0, 0, 0]\n",
            "invalid length 4, expected a list of exactly 3 score levels",
        ),
        ("[score]\nhard = [0, 0]\n", "missing field `soft`"),
        (
            "[score]\nhard = [0, 0]\nhard = [0, 0]\nsoft = [0, 0, 0]\n",
            "duplicate key",
        ),
    ] {
        let error = toml::from_str::<Holder>(input).expect_err(input);
        assert!(error.to_string().contains(expected), "{input}: {error}");
    }
}
