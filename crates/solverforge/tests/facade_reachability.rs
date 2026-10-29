//! A project that depends only on `solverforge` can name every type the
//! facade's own signatures use, write a custom constraint and a custom phase,
//! and solve synchronously with an explicit configuration.

#[path = "facade_reachability/domain/mod.rs"]
mod domain;

use std::sync::atomic::Ordering;

use domain::{Plan, Resource, Task, PROBE_PHASE_DRAWS};
use solverforge::config::{
    AcceptedCountForagerConfig, ChangeMoveConfig, ConfigError, ConstructionHeuristicConfig,
    CustomPhaseConfig, DirectorConfig, GreatDelugeConfig, LateAcceptanceConfig,
    LimitedNeighborhoodConfig, ListChangeMoveConfig, LocalSearchConfig, LocalSearchType,
    PartitionedSearchConfig, SimulatedAnnealingConfig, SwapMoveConfig, TabuSearchConfig,
    TerminationConfig, UnionMoveSelectorConfig, VariableTargetConfig,
};
use solverforge::prelude::*;
// The stream paths only need to resolve.
#[allow(unused_imports)]
use solverforge::stream::{
    chain::{Chain, ChainBuilder},
    cross::{Penta, Quad, Tri},
    projected::{Projection, Source},
};
use solverforge::{
    ConstraintRef, ImpactType, PhaseConfig, ProgressCallback, SolverConfig, SolverConfigOverride,
    SolverProgressKind, SolverProgressRef, SolverScope,
};

// Each probe only has to name its type; compiling is the assertion.
type ConfigProbes = (
    TerminationConfig,
    ConfigError,
    ConstructionHeuristicConfig,
    LocalSearchConfig,
    LocalSearchType,
    PartitionedSearchConfig,
    CustomPhaseConfig,
    ChangeMoveConfig,
    SwapMoveConfig,
    UnionMoveSelectorConfig,
    ListChangeMoveConfig,
    LimitedNeighborhoodConfig,
);
type MoreConfigProbes = (
    VariableTargetConfig,
    LateAcceptanceConfig,
    TabuSearchConfig,
    SimulatedAnnealingConfig,
    GreatDelugeConfig,
    AcceptedCountForagerConfig,
    DirectorConfig,
);
type IdentityProbes = (ConstraintRef, ImpactType, SolverProgressKind);

#[allow(dead_code)]
fn probe_types<'a>(
    _: Option<(ConfigProbes, MoreConfigProbes, IdentityProbes)>,
    _: Option<&SolverProgressRef<'a, Plan>>,
    _: Option<&SolverScope<'a, Plan, ScoreDirector<Plan, ()>, ()>>,
    _: Option<&solverforge::stats::SolverStats>,
) {
    fn takes_callback<C: ProgressCallback<Plan>>(_: C) {}
    takes_callback(());
}

fn load_config(text: &str) -> Result<SolverConfig, ConfigError> {
    SolverConfig::from_toml_str(text)
}

fn plan() -> Plan {
    Plan {
        resources: (0..2).map(|id| Resource { id }).collect(),
        tasks: (0..4)
            .map(|id| Task {
                id,
                resource_idx: None,
            })
            .collect(),
        score: None,
    }
}

#[test]
fn solve_with_config_runs_the_given_phases_including_a_facade_custom_phase() {
    let termination = TerminationConfig {
        step_count_limit: Some(10),
        ..TerminationConfig::default()
    };
    let _ = SolverConfigOverride::with_termination(termination.clone());
    let config = SolverConfig {
        random_seed: Some(3),
        termination: Some(termination),
        ..SolverConfig::default()
    }
    .with_phase(PhaseConfig::ConstructionHeuristic(
        ConstructionHeuristicConfig::default(),
    ))
    .with_phase(PhaseConfig::Custom(CustomPhaseConfig {
        name: "probe".to_string(),
    }));

    let before = PROBE_PHASE_DRAWS.load(Ordering::SeqCst);
    let solved = plan()
        .solve_with_config(config)
        .expect("explicitly configured solve succeeds");

    assert_eq!(PROBE_PHASE_DRAWS.load(Ordering::SeqCst), before + 1);
    assert!(solved.tasks.iter().all(|task| task.resource_idx.is_some()));
    assert_eq!(solved.score, Some(HardSoftScore::of(0, -4)));
}

#[test]
fn solve_with_config_reports_an_invalid_configuration() {
    let config = load_config("[[phases]]\ntype = \"custom\"\nname = \"not_registered\"\n")
        .expect("well-formed TOML");
    assert!(plan().solve_with_config(config).is_err());
}
