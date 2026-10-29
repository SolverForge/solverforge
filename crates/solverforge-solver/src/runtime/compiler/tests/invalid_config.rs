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

#[test]
fn unparsable_best_score_limits_fail_compilation_with_the_key_path() {
    use super::super::score_limits::validate_best_score_limits;
    use solverforge_config::{
        ConstructionHeuristicConfig, LocalSearchConfig, PartitionedSearchConfig, PhaseConfig,
        TerminationConfig,
    };
    use solverforge_core::score::HardSoftScore;

    let limit = |score: &str| {
        Some(TerminationConfig {
            best_score_limit: Some(score.to_string()),
            ..TerminationConfig::default()
        })
    };
    let partitioned = |child_limit: &str| {
        PhaseConfig::PartitionedSearch(PartitionedSearchConfig {
            child_phases: vec![
                PhaseConfig::ConstructionHeuristic(ConstructionHeuristicConfig::default()),
                PhaseConfig::LocalSearch(LocalSearchConfig {
                    termination: limit(child_limit),
                    ..LocalSearchConfig::default()
                }),
            ],
            ..PartitionedSearchConfig::default()
        })
    };
    let cases = [
        (
            SolverConfig {
                termination: limit("0hard/-5sotf"),
                ..SolverConfig::default()
            },
            "termination.best_score_limit",
        ),
        (
            SolverConfig {
                phases: vec![PhaseConfig::ConstructionHeuristic(
                    ConstructionHeuristicConfig {
                        termination: limit("-1"),
                        ..ConstructionHeuristicConfig::default()
                    },
                )],
                ..SolverConfig::default()
            },
            "phases[0].construction.termination.best_score_limit",
        ),
        (
            SolverConfig {
                phases: vec![
                    PhaseConfig::ConstructionHeuristic(ConstructionHeuristicConfig::default()),
                    partitioned("0hard"),
                ],
                ..SolverConfig::default()
            },
            "phases[1].partitioned_search.child_phases[1].local_search.termination.best_score_limit",
        ),
    ];
    for (config, path) in cases {
        let error = validate_best_score_limits::<HardSoftScore>(&config).expect_err(path);
        assert_eq!(error.path, path);
        assert!(
            matches!(error.kind, RuntimeCompileErrorKind::InvalidValue { .. }),
            "{path}: {error}"
        );
    }
    let valid = SolverConfig {
        termination: limit("0hard/-5soft"),
        phases: vec![partitioned("-1hard/0soft")],
        ..SolverConfig::default()
    };
    validate_best_score_limits::<HardSoftScore>(&valid).expect("valid limits compile");
}
