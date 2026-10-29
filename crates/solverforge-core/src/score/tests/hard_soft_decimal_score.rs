use crate::score::*;

#[test]
fn test_creation_unscaled() {
    let score = HardSoftDecimalScore::of(-2, -100);
    assert_eq!(score.hard_scaled(), -200000);
    assert_eq!(score.soft_scaled(), -10000000);

    // Wire hard_score() / soft_score() / has_hard_component()
    assert_eq!(
        score.hard_score(),
        HardSoftDecimalScore::of_hard_scaled(-200000)
    );
    assert_eq!(
        score.soft_score(),
        HardSoftDecimalScore::of_soft_scaled(-10000000)
    );
    assert!(score.has_hard_component());
    assert!(!HardSoftDecimalScore::ZERO.has_hard_component());
}

#[test]
fn test_creation_scaled() {
    let score = HardSoftDecimalScore::of_scaled(-30500, -208250);
    assert_eq!(score.hard_scaled(), -30500);
    assert_eq!(score.soft_scaled(), -208250);
}

#[test]
fn test_feasibility() {
    assert!(HardSoftDecimalScore::of(0, -1000).is_feasible());
    assert!(HardSoftDecimalScore::of(10, -50).is_feasible());
    assert!(!HardSoftDecimalScore::of(-1, 0).is_feasible());
    assert!(!HardSoftDecimalScore::of_scaled(-1, 0).is_feasible());
}

#[test]
fn test_comparison() {
    let infeasible = HardSoftDecimalScore::of(-1, 0);
    let feasible = HardSoftDecimalScore::of(0, -1000);
    assert!(feasible > infeasible);
    assert!(feasible.is_better_than(&infeasible));

    let s1 = HardSoftDecimalScore::of(0, -100);
    let s2 = HardSoftDecimalScore::of(0, -50);
    assert!(s2 > s1);
    assert!(s1.is_worse_than(&s2));

    let s3 = HardSoftDecimalScore::of(-2, 0);
    let s4 = HardSoftDecimalScore::of(-1, -1000);
    assert!(s4 > s3);
}

#[test]
fn test_arithmetic() {
    let s1 = HardSoftDecimalScore::of(-1, -100);
    let s2 = HardSoftDecimalScore::of(-1, -50);

    assert_eq!(s1 + s2, HardSoftDecimalScore::of(-2, -150));
    assert_eq!(s1 - s2, HardSoftDecimalScore::of(0, -50));
    assert_eq!(-s1, HardSoftDecimalScore::of(1, 100));
}

#[test]
fn test_arithmetic_scaled() {
    let s1 = HardSoftDecimalScore::of_scaled(-1500, -100500);
    let s2 = HardSoftDecimalScore::of_scaled(-500, -50250);

    let sum = s1 + s2;
    assert_eq!(sum.hard_scaled(), -2000);
    assert_eq!(sum.soft_scaled(), -150750);
}

#[test]
fn test_parse_integer() {
    let score = HardSoftDecimalScore::parse("0hard/-100soft").unwrap();
    assert_eq!(score.hard_scaled(), 0);
    assert_eq!(score.soft_scaled(), -10000000);
}

#[test]
fn test_parse_decimal() {
    let score = HardSoftDecimalScore::parse("-30.5hard/-208.25soft").unwrap();
    assert_eq!(score.hard_scaled(), -3050000);
    assert_eq!(score.soft_scaled(), -20825000);
}

#[test]
fn test_display() {
    let score = HardSoftDecimalScore::of_scaled(-3050000, -20825000);
    assert_eq!(format!("{}", score), "-30.5hard/-208.25soft");
}

#[test]
fn test_display_integer() {
    let score = HardSoftDecimalScore::of(-2, -100);
    assert_eq!(format!("{}", score), "-2hard/-100soft");
}

#[test]
fn test_display_zero() {
    let score = HardSoftDecimalScore::ZERO;
    assert_eq!(format!("{}", score), "0hard/0soft");
}

#[test]
fn test_level_numbers() {
    let score = HardSoftDecimalScore::of_scaled(-2000, -50000);
    assert_eq!(score.level_number(0), -2000);
    assert_eq!(score.level_number(1), -50000);
    assert_eq!(score.to_level_numbers(), vec![-2000, -50000]);
    assert_eq!(
        HardSoftDecimalScore::from_level_numbers(&[-2000, -50000]),
        score
    );
}

#[test]
fn test_level_label() {
    assert_eq!(HardSoftDecimalScore::level_label(0), ScoreLevel::Hard);
    assert_eq!(HardSoftDecimalScore::level_label(1), ScoreLevel::Soft);
}

#[test]
#[should_panic(expected = "HardSoftDecimalScore has 2 levels")]
fn test_level_number_out_of_range_panics() {
    let _ = HardSoftDecimalScore::ZERO.level_number(2);
}

#[test]
fn parse_rejects_non_decimal_and_unrepresentable_text() {
    for input in [
        "NaNhard/0soft",
        "infhard/0soft",
        "-infhard/0soft",
        "1e3hard/0soft",
        "0.000004hard/0soft",
        "0hard/-0.000004soft",
        "92233720368548hard/0soft",
        "-92233720368547.75809hard/0soft",
        ".5hard/0soft",
        "1.hard/0soft",
        "-hard/0soft",
        "+-1hard/0soft",
        "1_000hard/0soft",
    ] {
        assert!(
            HardSoftDecimalScore::parse(input).is_err(),
            "{input} must not parse"
        );
    }
}

#[test]
fn parse_is_exact_fixed_point() {
    assert_eq!(
        HardSoftDecimalScore::parse("-1.5hard/-0.00001soft").unwrap(),
        HardSoftDecimalScore::of_scaled(-150_000, -1)
    );
    assert_eq!(
        HardSoftDecimalScore::parse("+92233720368547.75807hard/-92233720368547.75808soft").unwrap(),
        HardSoftDecimalScore::of_scaled(i64::MAX, i64::MIN)
    );
    assert_eq!(
        HardSoftDecimalScore::parse("94108744501.60629hard/0soft").unwrap(),
        HardSoftDecimalScore::of_scaled(9_410_874_450_160_629, 0)
    );
}

#[test]
fn display_is_exact_and_trims_to_at_most_five_decimals() {
    for (scaled, text) in [
        (0, "0"),
        (1, "0.00001"),
        (-1, "-0.00001"),
        (-50_000, "-0.5"),
        (123_450, "1.2345"),
        (9_410_874_450_160_629, "94108744501.60629"),
        (i64::MAX, "92233720368547.75807"),
        (i64::MIN, "-92233720368547.75808"),
    ] {
        let score = HardSoftDecimalScore::of_scaled(scaled, 0);
        assert_eq!(score.to_string(), format!("{text}hard/0soft"));
        assert_eq!(
            format!("{score:?}"),
            format!("HardSoftDecimalScore({text}, 0)")
        );
    }
}

#[test]
fn display_then_parse_round_trips_every_magnitude() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut values = vec![0, 1, -1, i64::MAX, i64::MIN, i64::MAX - 1, i64::MIN + 1];
    for _ in 0..200_000 {
        // xorshift64*, spread over every bit width.
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let bits = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        values.push((bits >> (bits % 64)) as i64 * if bits & 1 == 0 { 1 } else { -1 });
    }
    for value in values {
        let score = HardSoftDecimalScore::of_scaled(value, value.wrapping_neg());
        let text = score.to_string();
        let fraction_digits = text
            .split('/')
            .filter_map(|part| {
                part.split_once('.')
                    .map(|(_, f)| f.trim_end_matches(char::is_alphabetic).len())
            })
            .max()
            .unwrap_or(0);
        assert!(fraction_digits <= 5, "{text}");
        assert_eq!(HardSoftDecimalScore::parse(&text), Ok(score), "{text}");
    }
}
