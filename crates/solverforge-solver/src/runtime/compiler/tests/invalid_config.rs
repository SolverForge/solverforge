use super::*;

#[test]
fn invalid_acceptor_and_forager_values_fail_compilation_with_the_key_path() {
    let cases = [
        (
            Some(AcceptorConfig::TabuSearch(TabuSearchConfig {
                entity_tabu_size: Some(0),
                ..TabuSearchConfig::default()
            })),
            None,
            "phases[1].local_search.acceptor.entity_tabu_size",
        ),
        (
            Some(AcceptorConfig::SimulatedAnnealing(
                SimulatedAnnealingConfig {
                    decay_rate: Some(0.0),
                    ..SimulatedAnnealingConfig::default()
                },
            )),
            None,
            "phases[1].local_search.acceptor.decay_rate",
        ),
        (
            Some(AcceptorConfig::SimulatedAnnealing(
                SimulatedAnnealingConfig {
                    level_temperatures: Some(vec![1.0, 2.0]),
                    ..SimulatedAnnealingConfig::default()
                },
            )),
            None,
            "phases[1].local_search.acceptor.level_temperatures",
        ),
        (
            Some(AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
                late_acceptance_size: Some(0),
            })),
            None,
            "phases[1].local_search.acceptor.late_acceptance_size",
        ),
        (
            None,
            Some(ForagerConfig::AcceptedCount(AcceptedCountForagerConfig {
                limit: Some(0),
            })),
            "phases[1].local_search.forager.limit",
        ),
    ];
    for (acceptor, forager, path) in cases {
        let error = match compile_policy(acceptor, forager) {
            Ok(_) => panic!("{path}: invalid value must fail compilation"),
            Err(error) => error,
        };
        assert_eq!(error.path, path);
        assert!(
            matches!(error.kind, RuntimeCompileErrorKind::InvalidValue { .. }),
            "{path}: {error}"
        );
    }
    compile_policy(
        Some(AcceptorConfig::TabuSearch(TabuSearchConfig {
            entity_tabu_size: Some(3),
            ..TabuSearchConfig::default()
        })),
        Some(ForagerConfig::AcceptedCount(AcceptedCountForagerConfig {
            limit: Some(2),
        })),
    )
    .expect("valid acceptor and forager values compile");
}
