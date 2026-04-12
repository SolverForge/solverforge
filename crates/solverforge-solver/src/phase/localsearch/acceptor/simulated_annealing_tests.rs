use super::*;
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
fn accepts_improving_moves() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(1000.0, 0.99, 42);
    let last = SoftScore::of(-10);
    let better = SoftScore::of(-5);
    assert!(Acceptor::<SimpleSol>::is_accepted(
        &mut acceptor,
        &last,
        &better
    ));
}

#[test]
fn accepts_equal_moves() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(1000.0, 0.99, 42);
    let score = SoftScore::of(-10);
    assert!(Acceptor::<SimpleSol>::is_accepted(
        &mut acceptor,
        &score,
        &score
    ));
}

#[test]
fn rejects_at_zero_temperature() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(0.0, 0.99, 42);
    acceptor.current_temperature = 0.0;
    let last = SoftScore::of(-10);
    let worse = SoftScore::of(-20);
    assert!(!Acceptor::<SimpleSol>::is_accepted(
        &mut acceptor,
        &last,
        &worse
    ));
}

#[test]
fn high_temperature_accepts_most() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(1_000_000.0, 0.99, 42);
    let last = SoftScore::of(-10);
    let worse = SoftScore::of(-11);
    let mut accepted = 0;
    for _ in 0..100 {
        if Acceptor::<SimpleSol>::is_accepted(&mut acceptor, &last, &worse) {
            accepted += 1;
        }
    }
    assert!(accepted > 90);
}

#[test]
fn low_temperature_rejects_most() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(0.001, 0.99, 42);
    let last = SoftScore::of(-10);
    let worse = SoftScore::of(-20);
    let mut accepted = 0;
    for _ in 0..100 {
        if Acceptor::<SimpleSol>::is_accepted(&mut acceptor, &last, &worse) {
            accepted += 1;
        }
    }
    assert!(accepted < 5);
}

#[test]
fn temperature_decays_each_step() {
    let mut acceptor = SimulatedAnnealingAcceptor::with_seed(100.0, 0.5, 42);
    let score = SoftScore::of(0);
    Acceptor::<SimpleSol>::phase_started(&mut acceptor, &score);
    assert!((acceptor.current_temperature - 100.0).abs() < f64::EPSILON);
    Acceptor::<SimpleSol>::step_ended(&mut acceptor, &score);
    assert!((acceptor.current_temperature - 50.0).abs() < f64::EPSILON);
    Acceptor::<SimpleSol>::step_ended(&mut acceptor, &score);
    assert!((acceptor.current_temperature - 25.0).abs() < f64::EPSILON);
}

#[test]
fn auto_calibrate_sets_temperature_from_initial_score() {
    let mut acceptor = SimulatedAnnealingAcceptor::auto_calibrate(0.999);
    let initial = HardSoftScore::of(-576, 0);
    Acceptor::<HardSoftSol>::phase_started(&mut acceptor, &initial);
    assert!(acceptor.current_temperature > 10_000_000.0);
    assert!(acceptor.current_temperature < 20_000_000.0);
}

#[test]
fn seeded_auto_calibrate_matches_unseeded_temperature() {
    let initial = HardSoftScore::of(-576, 0);
    let mut seeded = SimulatedAnnealingAcceptor::auto_calibrate_with_seed(0.999, 42);
    let mut unseeded = SimulatedAnnealingAcceptor::auto_calibrate(0.999);

    Acceptor::<HardSoftSol>::phase_started(&mut seeded, &initial);
    Acceptor::<HardSoftSol>::phase_started(&mut unseeded, &initial);

    assert_eq!(seeded.starting_temperature, unseeded.starting_temperature);
    assert_eq!(seeded.current_temperature, unseeded.current_temperature);
}

#[test]
fn score_delta_to_scalar_simple() {
    assert!((score_delta_to_scalar(&[-5]) - -5.0).abs() < f64::EPSILON);
}

#[test]
fn score_delta_to_scalar_hard_soft() {
    let scalar = score_delta_to_scalar(&[-1, -50]);
    assert!((scalar - -1_000_050.0).abs() < f64::EPSILON);
}
