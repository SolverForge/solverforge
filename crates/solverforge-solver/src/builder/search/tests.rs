use std::any::TypeId;

use solverforge_config::{CustomPhaseConfig, PartitionedSearchConfig, PhaseConfig, SolverConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use crate::run::ChannelProgressCallback;
use crate::scope::{ProgressCallback, SolverScope};

use super::{CustomSearchPhase, Search, SearchContext};
use crate::builder::RuntimeModel;

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

#[derive(Debug)]
struct MarkerPhase(&'static str);

impl CustomSearchPhase<TestSolution> for MarkerPhase {
    fn solve<D, ProgressCb>(
        &mut self,
        _solver_scope: &mut SolverScope<'_, TestSolution, D, ProgressCb>,
    ) where
        D: solverforge_scoring::Director<TestSolution>,
        ProgressCb: ProgressCallback<TestSolution>,
    {
    }

    fn phase_type_name(&self) -> &'static str {
        self.0
    }
}

fn search_context() -> SearchContext<TestSolution> {
    SearchContext::new(
        SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>()),
        RuntimeModel::new(Vec::new()),
        Some(7),
    )
}

fn custom_config(names: &[&str]) -> SolverConfig {
    SolverConfig {
        phases: names
            .iter()
            .map(|name| {
                PhaseConfig::Custom(CustomPhaseConfig {
                    name: (*name).to_string(),
                })
            })
            .collect(),
        ..SolverConfig::default()
    }
}

fn partitioned_config(name: &str) -> SolverConfig {
    SolverConfig {
        phases: vec![PhaseConfig::PartitionedSearch(PartitionedSearchConfig {
            partitioner: Some(name.to_string()),
            ..Default::default()
        })],
        ..SolverConfig::default()
    }
}

#[test]
fn custom_search_builds_registered_names_in_configured_order() {
    let search = search_context()
        .defaults()
        .phase("weekend_repair", |_| MarkerPhase("weekend_repair"))
        .phase("nurse_search", |_| MarkerPhase("nurse_search"));
    let phases = Search::<TestSolution>::build::<
        ScoreDirector<TestSolution, ()>,
        ChannelProgressCallback<TestSolution>,
    >(search, &custom_config(&["nurse_search", "weekend_repair"]));
    let debug = format!("{phases:?}");

    let nurse_pos = debug
        .find("nurse_search")
        .expect("configured nurse_search phase should be built");
    let weekend_pos = debug
        .find("weekend_repair")
        .expect("configured weekend_repair phase should be built");
    assert!(nurse_pos < weekend_pos);
}

#[test]
#[should_panic(expected = "custom phase `repair` was registered more than once")]
fn custom_search_rejects_duplicate_registered_names() {
    let _ = search_context()
        .defaults()
        .phase("repair", |_| MarkerPhase("first"))
        .phase("repair", |_| MarkerPhase("second"));
}

#[test]
#[should_panic(
    expected = "custom phase `missing` was not registered by the solution search function"
)]
fn custom_search_rejects_unregistered_configured_name() {
    let search = search_context()
        .defaults()
        .phase("registered", |_| MarkerPhase("registered"));
    let _ = Search::<TestSolution>::build::<
        ScoreDirector<TestSolution, ()>,
        ChannelProgressCallback<TestSolution>,
    >(search, &custom_config(&["missing"]));
}

#[test]
fn partitioned_search_builds_registered_partitioner_name() {
    let search = search_context()
        .defaults()
        .partitioned_phase("by_task", |_context, config| {
            assert_eq!(config.partitioner.as_deref(), Some("by_task"));
            MarkerPhase("by_task")
        });
    let phases = Search::<TestSolution>::build::<
        ScoreDirector<TestSolution, ()>,
        ChannelProgressCallback<TestSolution>,
    >(search, &partitioned_config("by_task"));
    let debug = format!("{phases:?}");

    assert!(debug.contains("by_task"));
}

#[test]
#[should_panic(
    expected = "partitioned_search partitioner `missing` was not registered by the solution search function"
)]
fn partitioned_search_rejects_unregistered_partitioner_name() {
    let search = search_context()
        .defaults()
        .partitioned_phase("registered", |_context, _config| MarkerPhase("registered"));
    let _ = Search::<TestSolution>::build::<
        ScoreDirector<TestSolution, ()>,
        ChannelProgressCallback<TestSolution>,
    >(search, &partitioned_config("missing"));
}

#[test]
#[should_panic(expected = "custom phase `missing` requires a typed solution search function")]
fn stock_runtime_rejects_custom_phase_without_search_registration() {
    let context = search_context();
    let _ = crate::runtime::build_phases(
        &custom_config(&["missing"]),
        context.descriptor(),
        context.model(),
    );
}

#[test]
#[should_panic(
    expected = "partitioned_search partitioner `missing` requires typed partitioner registration"
)]
fn stock_runtime_rejects_partitioned_search_without_registration() {
    let context = search_context();
    let _ = crate::runtime::build_phases(
        &partitioned_config("missing"),
        context.descriptor(),
        context.model(),
    );
}
