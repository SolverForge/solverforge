use super::*;
use solverforge_config::{
    AcceptorConfig, LateAcceptanceConfig, SimulatedAnnealingConfig, TabuSearchConfig,
};
use solverforge_core::score::SoftScore;
use std::any::Any;

#[derive(Clone, Debug)]
struct TestSolution {
    score: Option<SoftScore>,
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
    let config = AcceptorConfig::HillClimbing;
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
}

#[test]
fn test_acceptor_builder_tabu_search() {
    let config = AcceptorConfig::TabuSearch(TabuSearchConfig {
        entity_tabu_size: Some(10),
        ..Default::default()
    });
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
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
    let acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
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
        let result = std::panic::catch_unwind(|| {
            let config = AcceptorConfig::TabuSearch(config);
            let _: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
        });
        let message = panic_message(result.expect_err("zero tabu size must panic"));
        assert_eq!(
            message,
            format!("tabu_search field `{field_name}` must be greater than 0")
        );
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
        starting_temperature: Some("2".to_string()),
        decay_rate: None,
    });
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
}

#[test]
fn test_acceptor_builder_simulated_annealing_accepts_fractional_scalar() {
    let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
        starting_temperature: Some("2.5".to_string()),
        decay_rate: None,
    });
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
}

#[test]
fn test_acceptor_builder_late_acceptance() {
    let config = AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
        late_acceptance_size: Some(500),
    });
    let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
}
