use super::*;
use solverforge_config::{
    AcceptorConfig, DiversifiedLateAcceptanceConfig, HardRegressionPolicyConfig,
    LateAcceptanceConfig, SimulatedAnnealingCalibrationConfig, SimulatedAnnealingConfig,
    TabuSearchConfig,
};
use solverforge_core::score::{HardSoftScore, SoftScore};
use std::any::Any;

#[derive(Clone, Debug)]
struct TestSolution {
    score: Option<SoftScore>,
}

#[derive(Clone, Debug)]
struct HardSoftTestSolution {
    score: Option<HardSoftScore>,
}

impl PlanningSolution for HardSoftTestSolution {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[test]
fn test_acceptor_builder_hill_climbing() {
    let config = AcceptorConfig::HillClimbing {};
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config).unwrap();
}

#[test]
fn test_acceptor_builder_tabu_search() {
    let config = AcceptorConfig::TabuSearch(TabuSearchConfig {
        entity_tabu_size: Some(10),
        ..Default::default()
    });
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config).unwrap();
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_string(),
            Err(_) => "<non-string panic>".to_string(),
        },
    }
}

#[test]
fn test_acceptor_builder_tabu_search_normalizes_default_to_move_tabu() {
    let config = AcceptorConfig::TabuSearch(TabuSearchConfig::default());
    let acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config).unwrap();
    let rendered = format!("{acceptor:?}");

    assert!(rendered.contains("move_tabu_size: Some(10)"));
    assert!(rendered.contains("entity_tabu_size: None"));
    assert!(rendered.contains("value_tabu_size: None"));
    assert!(rendered.contains("undo_move_tabu_size: None"));
    assert!(rendered.contains("aspiration_enabled: true"));
}

#[test]
fn test_acceptor_builder_tabu_search_rejects_zero_sizes() {
    for (field_name, config) in [
        (
            "entity_tabu_size",
            TabuSearchConfig {
                entity_tabu_size: Some(0),
                ..Default::default()
            },
        ),
        (
            "value_tabu_size",
            TabuSearchConfig {
                value_tabu_size: Some(0),
                ..Default::default()
            },
        ),
        (
            "move_tabu_size",
            TabuSearchConfig {
                move_tabu_size: Some(0),
                ..Default::default()
            },
        ),
        (
            "undo_move_tabu_size",
            TabuSearchConfig {
                undo_move_tabu_size: Some(0),
                ..Default::default()
            },
        ),
    ] {
        let config = AcceptorConfig::TabuSearch(config);
        let error = AcceptorBuilder::build::<TestSolution>(&config)
            .expect_err("zero tabu size must be rejected");
        assert_eq!(error.field(), field_name);
        assert_eq!(error.message(), "must be greater than 0");
    }
}

#[test]
fn test_acceptor_builder_tabu_search_helper_rejects_zero_size() {
    let result = std::panic::catch_unwind(|| {
        let _ = AcceptorBuilder::tabu_search::<TestSolution>(0);
    });
    let message = panic_message(result.expect_err("zero tabu size must panic"));
    assert_eq!(
        message,
        "tabu_search field `move_tabu_size` must be greater than 0"
    );
}

#[test]
fn test_acceptor_builder_simulated_annealing() {
    let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
        level_temperatures: Some(vec![2.0]),
        decay_rate: None,
        hill_climbing_temperature: None,
        hard_regression_policy: None,
        calibration: None,
    });
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config).unwrap();
}

#[test]
fn test_acceptor_builder_simulated_annealing_accepts_fractional_level_temperature() {
    let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
        level_temperatures: Some(vec![2.5]),
        decay_rate: None,
        hill_climbing_temperature: None,
        hard_regression_policy: None,
        calibration: None,
    });
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config).unwrap();
}

#[test]
fn test_acceptor_builder_simulated_annealing_accepts_hard_regression_policy() {
    let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
        level_temperatures: Some(vec![2.5, 100.0]),
        hard_regression_policy: Some(HardRegressionPolicyConfig::NeverAcceptHardRegression),
        ..Default::default()
    });
    let acceptor: AnyAcceptor<HardSoftTestSolution> = AcceptorBuilder::build(&config).unwrap();
    assert!(format!("{acceptor:?}").contains("NeverAcceptHardRegression"));
}

#[test]
fn test_acceptor_builder_simulated_annealing_rejects_wrong_temperature_level_count() {
    let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
        level_temperatures: Some(vec![2.5]),
        ..Default::default()
    });
    let error = AcceptorBuilder::build::<HardSoftTestSolution>(&config)
        .expect_err("wrong level count must be rejected");
    assert_eq!(error.field(), "level_temperatures");
    assert_eq!(
        error.message(),
        "must have one temperature per score level (2), found 1"
    );
}

#[test]
fn test_acceptor_builder_simulated_annealing_rejects_invalid_decay_rate() {
    for decay_rate in [0.0, -0.5, 1.5, f64::NAN, f64::INFINITY] {
        let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
            decay_rate: Some(decay_rate),
            ..Default::default()
        });
        let error = AcceptorBuilder::build::<TestSolution>(&config)
            .expect_err("invalid decay rate must be rejected");
        assert_eq!(error.field(), "decay_rate");
        assert!(error.message().starts_with("must be finite and in (0, 1]"));
    }
}

#[test]
fn test_acceptor_builder_simulated_annealing_rejects_invalid_temperatures() {
    let cases = [
        (
            SimulatedAnnealingConfig {
                hill_climbing_temperature: Some(-1.0),
                ..Default::default()
            },
            "hill_climbing_temperature",
        ),
        (
            SimulatedAnnealingConfig {
                level_temperatures: Some(vec![f64::NAN]),
                ..Default::default()
            },
            "level_temperatures",
        ),
        (
            SimulatedAnnealingConfig {
                calibration: Some(SimulatedAnnealingCalibrationConfig {
                    sample_size: Some(0),
                    ..Default::default()
                }),
                ..Default::default()
            },
            "calibration.sample_size",
        ),
        (
            SimulatedAnnealingConfig {
                calibration: Some(SimulatedAnnealingCalibrationConfig {
                    fallback_temperature: Some(f64::INFINITY),
                    ..Default::default()
                }),
                ..Default::default()
            },
            "calibration.fallback_temperature",
        ),
    ];
    for (config, field) in cases {
        let config = AcceptorConfig::SimulatedAnnealing(config);
        let error = AcceptorBuilder::build::<TestSolution>(&config)
            .expect_err("invalid temperature must be rejected");
        assert_eq!(error.field(), field);
    }
}

#[test]
fn test_acceptor_builder_simulated_annealing_rejects_invalid_calibration_probability() {
    for probability in [0.0, 1.0, f64::NAN] {
        let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
            calibration: Some(SimulatedAnnealingCalibrationConfig {
                target_acceptance_probability: Some(probability),
                ..Default::default()
            }),
            ..Default::default()
        });
        let error = AcceptorBuilder::build::<TestSolution>(&config)
            .expect_err("invalid calibration probability must be rejected");
        assert_eq!(error.field(), "calibration.target_acceptance_probability");
        assert!(error.message().starts_with("must be in (0, 1)"));
    }
}

#[test]
fn test_acceptor_builder_late_acceptance() {
    let config = AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
        late_acceptance_size: Some(500),
    });
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config).unwrap();
}

#[test]
fn test_acceptor_builder_late_acceptance_rejects_zero_size() {
    for config in [
        AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(0),
        }),
        AcceptorConfig::DiversifiedLateAcceptance(DiversifiedLateAcceptanceConfig {
            late_acceptance_size: Some(0),
        }),
    ] {
        let error = AcceptorBuilder::build::<TestSolution>(&config)
            .expect_err("zero late acceptance size must be rejected");
        assert_eq!(error.field(), "late_acceptance_size");
        assert_eq!(
            error.to_string(),
            "`late_acceptance_size` must be greater than 0"
        );
    }
}
