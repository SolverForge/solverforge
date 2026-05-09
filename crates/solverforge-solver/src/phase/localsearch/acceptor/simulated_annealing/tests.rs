use super::*;
use crate::phase::localsearch::HillClimbingAcceptor;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{HardSoftScore, SoftScore};

#[derive(Clone, Debug)]
struct SimpleSol {
    score: Option<SoftScore>,
}

impl PlanningSolution for SimpleSol {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct HardSoftSol {
    score: Option<HardSoftScore>,
}

impl PlanningSolution for HardSoftSol {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[test]
fn accepts_improving_and_equal_moves() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(1000.0, 0.99, 42);
    let last = SoftScore::of(-10);
    assert!(Acceptor::<SimpleSol>::is_accepted(
        &mut acceptor,
        &last,
        &SoftScore::of(-5),
        None,
    ));
    assert!(Acceptor::<SimpleSol>::is_accepted(
        &mut acceptor,
        &last,
        &last,
        None,
    ));
}

#[test]
fn single_level_high_temperature_accepts_most_worsening_moves() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(1_000_000.0, 0.99, 42);
    Acceptor::<SimpleSol>::phase_started(&mut acceptor, &SoftScore::of(0));
    let last = SoftScore::of(-10);
    let worse = SoftScore::of(-11);
    let mut accepted = 0;
    for _ in 0..100 {
        if Acceptor::<SimpleSol>::is_accepted(&mut acceptor, &last, &worse, None) {
            accepted += 1;
        }
    }
    assert!(accepted > 90);
}

#[test]
fn single_level_low_temperature_rejects_most_worsening_moves() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(0.001, 0.99, 42);
    Acceptor::<SimpleSol>::phase_started(&mut acceptor, &SoftScore::of(0));
    let last = SoftScore::of(-10);
    let worse = SoftScore::of(-20);
    let mut accepted = 0;
    for _ in 0..100 {
        if Acceptor::<SimpleSol>::is_accepted(&mut acceptor, &last, &worse, None) {
            accepted += 1;
        }
    }
    assert!(accepted < 5);
}

#[test]
fn temperature_decays_each_step_until_hill_climbing_threshold() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_level_temperatures_and_seed(
        vec![100.0],
        0.5,
        20.0,
        HardRegressionPolicy::TemperatureControlled,
        42,
    );
    Acceptor::<SimpleSol>::phase_started(&mut acceptor, &SoftScore::of(0));
    assert!((acceptor.current_temperature_for_level(0) - 100.0).abs() < f64::EPSILON);
    Acceptor::<SimpleSol>::step_ended(&mut acceptor, &SoftScore::of(0), None);
    assert!((acceptor.current_temperature_for_level(0) - 50.0).abs() < f64::EPSILON);
    Acceptor::<SimpleSol>::step_ended(&mut acceptor, &SoftScore::of(0), None);
    assert!((acceptor.current_temperature_for_level(0) - 25.0).abs() < f64::EPSILON);
    Acceptor::<SimpleSol>::step_ended(&mut acceptor, &SoftScore::of(0), None);
    assert!((acceptor.current_temperature_for_level(0) - 20.0).abs() < f64::EPSILON);
}

#[test]
fn huge_soft_improvement_does_not_mask_hard_regression() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_level_temperatures_and_seed(
        vec![1.0e-9, 1.0e12],
        1.0,
        1.0e-9,
        HardRegressionPolicy::TemperatureControlled,
        42,
    );
    Acceptor::<HardSoftSol>::phase_started(&mut acceptor, &HardSoftScore::of(-10, -1_000_000));

    let last = HardSoftScore::of(-10, -1_000_000);
    let worse_hard_better_soft = HardSoftScore::of(-11, 0);
    for _ in 0..100 {
        assert!(!Acceptor::<HardSoftSol>::is_accepted(
            &mut acceptor,
            &last,
            &worse_hard_better_soft,
            None,
        ));
    }
}

#[test]
fn hard_improvement_with_soft_regression_is_accepted_as_improving() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(0.0, 1.0, 42);
    let last = HardSoftScore::of(-2, 0);
    let better_hard_worse_soft = HardSoftScore::of(-1, -1_000_000);
    assert!(Acceptor::<HardSoftSol>::is_accepted(
        &mut acceptor,
        &last,
        &better_hard_worse_soft,
        None,
    ));
}

#[test]
fn unchanged_hard_soft_regression_uses_soft_temperature() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_level_temperatures_and_seed(
        vec![0.0, 1_000_000.0],
        1.0,
        1.0e-9,
        HardRegressionPolicy::TemperatureControlled,
        42,
    );
    Acceptor::<HardSoftSol>::phase_started(&mut acceptor, &HardSoftScore::of(0, 0));
    let last = HardSoftScore::of(0, -10);
    let worse_soft = HardSoftScore::of(0, -11);
    let mut accepted = 0;
    for _ in 0..100 {
        if Acceptor::<HardSoftSol>::is_accepted(&mut acceptor, &last, &worse_soft, None) {
            accepted += 1;
        }
    }
    assert!(accepted > 90);
}

#[test]
fn never_accept_hard_regression_policy_rejects_hard_regressions_at_high_temperature() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_level_temperatures_and_seed(
        vec![1.0e12, 1.0e12],
        1.0,
        1.0e-9,
        HardRegressionPolicy::NeverAcceptHardRegression,
        42,
    );
    Acceptor::<HardSoftSol>::phase_started(&mut acceptor, &HardSoftScore::of(0, 0));
    assert!(!Acceptor::<HardSoftSol>::is_accepted(
        &mut acceptor,
        &HardSoftScore::of(-10, 0),
        &HardSoftScore::of(-11, 10_000),
        None,
    ));
}

#[test]
fn cooled_simulated_annealing_matches_hill_climbing() {
    let mut annealing = SimulatedAnnealingAcceptor::with_level_temperatures_and_seed(
        vec![100.0, 100.0],
        0.1,
        1.1,
        HardRegressionPolicy::TemperatureControlled,
        42,
    );
    let mut hill = HillClimbingAcceptor::new();
    let initial = HardSoftScore::of(0, 0);
    Acceptor::<HardSoftSol>::phase_started(&mut annealing, &initial);
    Acceptor::<HardSoftSol>::phase_started(&mut hill, &initial);
    for _ in 0..3 {
        Acceptor::<HardSoftSol>::step_ended(&mut annealing, &initial, None);
    }

    for (last, candidate) in [
        (HardSoftScore::of(0, 0), HardSoftScore::of(0, -1)),
        (HardSoftScore::of(-1, 0), HardSoftScore::of(-2, 10_000)),
        (HardSoftScore::of(-1, 0), HardSoftScore::of(0, -10_000)),
    ] {
        assert_eq!(
            Acceptor::<HardSoftSol>::is_accepted(&mut annealing, &last, &candidate, None),
            Acceptor::<HardSoftSol>::is_accepted(&mut hill, &last, &candidate, None),
        );
    }
}

#[test]
fn sampled_calibration_derives_temperatures_per_level() {
    let calibration = SimulatedAnnealingCalibration {
        sample_size: 2,
        target_acceptance_probability: 0.5,
        fallback_temperature: 1.0,
    };
    let mut acceptor = SimulatedAnnealingAcceptor::with_calibration_and_seed(
        1.0,
        1.0e-9,
        HardRegressionPolicy::TemperatureControlled,
        calibration,
        42,
    );
    Acceptor::<HardSoftSol>::phase_started(&mut acceptor, &HardSoftScore::of(0, 0));

    assert!(!Acceptor::<HardSoftSol>::is_accepted(
        &mut acceptor,
        &HardSoftScore::of(0, 0),
        &HardSoftScore::of(-4, 0),
        None,
    ));
    let _ = Acceptor::<HardSoftSol>::is_accepted(
        &mut acceptor,
        &HardSoftScore::of(0, 0),
        &HardSoftScore::of(0, -10),
        None,
    );

    assert!(acceptor.current_temperature_for_level(0) > 5.0);
    assert!(acceptor.current_temperature_for_level(1) > 14.0);
}

#[test]
fn seeded_auto_calibration_starts_with_same_temperatures() {
    let initial = HardSoftScore::of(-576, -1000);
    let mut first = SimulatedAnnealingAcceptor::with_calibration_and_seed(
        0.999,
        DEFAULT_HILL_CLIMBING_TEMPERATURE,
        HardRegressionPolicy::TemperatureControlled,
        SimulatedAnnealingCalibration::default(),
        42,
    );
    let mut second = SimulatedAnnealingAcceptor::with_calibration_and_seed(
        0.999,
        DEFAULT_HILL_CLIMBING_TEMPERATURE,
        HardRegressionPolicy::TemperatureControlled,
        SimulatedAnnealingCalibration::default(),
        42,
    );

    Acceptor::<HardSoftSol>::phase_started(&mut first, &initial);
    Acceptor::<HardSoftSol>::phase_started(&mut second, &initial);

    assert_eq!(
        first.current_temperature_for_level(0),
        second.current_temperature_for_level(0)
    );
    assert_eq!(
        first.current_temperature_for_level(1),
        second.current_temperature_for_level(1)
    );
}
