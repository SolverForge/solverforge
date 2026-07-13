use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use solverforge_config::{
    CustomPhaseConfig, MoveThreadCount, PartitionedSearchConfig, PhaseConfig, SolverConfig,
};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;

use super::{
    compile_runtime_graph, CompiledRuntimeExecutor, CompiledRuntimeExtension, CompiledRuntimePhase,
    PreparedRuntimePhase, RuntimeCompileErrorKind, RuntimeExtensionKind, RuntimeGraphInput,
};
use crate::builder::{
    CustomSearchPhase, NoDynamicExtensions, NoTypedExtensions, RuntimeModel, SearchContext,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::scope::{ProgressCallback, SolverScope};

static EXTENSION_BUILD_CALLS: AtomicUsize = AtomicUsize::new(0);
static EXTENSION_BUILD_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug)]
struct Plan {
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Debug)]
struct MarkerExtension;

impl CustomSearchPhase<Plan> for MarkerExtension {
    fn solve<D, ProgressCb>(&mut self, _solver_scope: &mut SolverScope<'_, Plan, D, ProgressCb>)
    where
        D: solverforge_scoring::Director<Plan>,
        ProgressCb: ProgressCallback<Plan>,
    {
    }
}

fn context(seed: Option<u64>) -> SearchContext<Plan> {
    SearchContext::new(
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
        RuntimeModel::new(Vec::new()),
        seed,
    )
}

/// Test-only reference meter. The direct executor accepts the same
/// cross-entity metric shape as typed model authoring; it does not require a
/// second ListPositionDistanceMeter implementation.
#[derive(Clone, Debug)]
struct ExtensionMeter;

impl CrossEntityDistanceMeter<Plan> for ExtensionMeter {
    fn distance(&self, _: &Plan, _: usize, _: usize, _: usize, _: usize) -> f64 {
        0.0
    }
}

fn extension_context(
    seed: Option<u64>,
) -> SearchContext<Plan, usize, ExtensionMeter, ExtensionMeter> {
    SearchContext::new(
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
        RuntimeModel::new(Vec::new()),
        seed,
    )
}

fn custom(name: &str) -> PhaseConfig {
    PhaseConfig::Custom(CustomPhaseConfig {
        name: name.to_string(),
    })
}

fn partitioned(name: Option<&str>) -> PhaseConfig {
    PhaseConfig::PartitionedSearch(PartitionedSearchConfig {
        partitioner: name.map(str::to_string),
        thread_count: MoveThreadCount::Count(3),
        log_progress: true,
        child_phases: vec![custom("child_extension")],
        termination: None,
    })
}

#[test]
fn typed_extensions_lower_names_without_invoking_builders_and_freeze_config() {
    let _guard = EXTENSION_BUILD_LOCK.lock().expect("test lock");
    EXTENSION_BUILD_CALLS.store(0, Ordering::SeqCst);
    let mut config = SolverConfig {
        random_seed: Some(41),
        phases: vec![custom("repair"), partitioned(Some("by_task"))],
        ..SolverConfig::default()
    };
    let declaration = context(config.random_seed)
        .defaults()
        .phase("repair", |_| {
            EXTENSION_BUILD_CALLS.fetch_add(1, Ordering::SeqCst);
            MarkerExtension
        })
        .partitioned_phase("by_task", |_context, _config| {
            EXTENSION_BUILD_CALLS.fetch_add(1, Ordering::SeqCst);
            MarkerExtension
        });
    let (context, extensions) = declaration.into_runtime_parts();
    let input = RuntimeGraphInput::new(context, extensions);

    let graph = compile_runtime_graph(&config, input)
        .expect("registered typed extensions compile into graph declarations");
    assert_eq!(EXTENSION_BUILD_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(graph.context().seed(), Some(41));
    assert_eq!(graph.phases().len(), 2);
    assert!(matches!(
        &graph.phases()[0],
        CompiledRuntimePhase::Extension(CompiledRuntimeExtension::Custom { name })
            if name == "repair"
    ));
    let CompiledRuntimePhase::Extension(CompiledRuntimeExtension::Partitioned {
        name,
        config: frozen,
    }) = &graph.phases()[1]
    else {
        panic!("partitioned extension should be lowered as one immutable graph node");
    };
    assert_eq!(name, "by_task");
    assert_eq!(frozen.partitioner.as_deref(), Some("by_task"));
    assert_eq!(frozen.thread_count, MoveThreadCount::Count(3));
    assert!(frozen.log_progress);
    assert_eq!(frozen.child_phases.len(), 1);

    let PhaseConfig::PartitionedSearch(live) = &mut config.phases[1] else {
        panic!("test configuration should retain its partitioned phase");
    };
    live.partitioner = Some("mutated_after_compile".to_string());
    live.thread_count = MoveThreadCount::None;
    assert_eq!(frozen.partitioner.as_deref(), Some("by_task"));
    assert_eq!(frozen.thread_count, MoveThreadCount::Count(3));
}

#[test]
fn typed_extensions_instantiate_fresh_once_per_solve_after_graph_compilation() {
    let _guard = EXTENSION_BUILD_LOCK.lock().expect("test lock");
    EXTENSION_BUILD_CALLS.store(0, Ordering::SeqCst);
    let config = SolverConfig {
        phases: vec![custom("repair")],
        ..SolverConfig::default()
    };
    let declaration = extension_context(config.random_seed)
        .defaults()
        .phase("repair", |_| {
            EXTENSION_BUILD_CALLS.fetch_add(1, Ordering::SeqCst);
            MarkerExtension
        });
    let (context, extensions) = declaration.into_runtime_parts();
    let input = RuntimeGraphInput::new(context, extensions);
    let graph = compile_runtime_graph(&config, input)
        .expect("registered typed extension compiles without builder work");
    assert_eq!(EXTENSION_BUILD_CALLS.load(Ordering::SeqCst), 0);

    let executor = CompiledRuntimeExecutor::new(graph);
    let first = executor
        .instantiate()
        .expect("first solve instantiates the declared extension");
    assert_eq!(EXTENSION_BUILD_CALLS.load(Ordering::SeqCst), 1);
    assert!(matches!(
        first.phases.as_slice(),
        [PreparedRuntimePhase::Extension(_)]
    ));

    let second = executor
        .instantiate()
        .expect("each solve gets one fresh declared extension instance");
    assert_eq!(EXTENSION_BUILD_CALLS.load(Ordering::SeqCst), 2);
    assert!(matches!(
        second.phases.as_slice(),
        [PreparedRuntimePhase::Extension(_)]
    ));
}

#[test]
fn typed_extension_name_errors_are_precise() {
    let missing_custom = SolverConfig {
        phases: vec![custom("")],
        ..SolverConfig::default()
    };
    let error = compile_runtime_graph(
        &missing_custom,
        RuntimeGraphInput::new(context(missing_custom.random_seed), NoTypedExtensions),
    )
    .expect_err("an empty custom extension name is invalid");
    assert!(matches!(
        error.kind,
        RuntimeCompileErrorKind::MissingCustomExtensionName
    ));

    let unknown_custom = SolverConfig {
        phases: vec![custom("missing")],
        ..SolverConfig::default()
    };
    let error = compile_runtime_graph(
        &unknown_custom,
        RuntimeGraphInput::new(context(unknown_custom.random_seed), NoTypedExtensions),
    )
    .expect_err("unregistered typed custom name is invalid");
    assert!(matches!(
        error.kind,
        RuntimeCompileErrorKind::UnregisteredTypedCustomExtension { ref name }
            if name == "missing"
    ));

    let missing_partitioner = SolverConfig {
        phases: vec![partitioned(None)],
        ..SolverConfig::default()
    };
    let error = compile_runtime_graph(
        &missing_partitioner,
        RuntimeGraphInput::new(context(missing_partitioner.random_seed), NoTypedExtensions),
    )
    .expect_err("a partitioned extension needs an explicit partitioner name");
    assert!(matches!(
        error.kind,
        RuntimeCompileErrorKind::MissingPartitionerName
    ));

    let unknown_partitioner = SolverConfig {
        phases: vec![partitioned(Some("missing"))],
        ..SolverConfig::default()
    };
    let error = compile_runtime_graph(
        &unknown_partitioner,
        RuntimeGraphInput::new(context(unknown_partitioner.random_seed), NoTypedExtensions),
    )
    .expect_err("unregistered typed partitioner is invalid");
    assert!(matches!(
        error.kind,
        RuntimeCompileErrorKind::UnregisteredTypedPartitioner { ref name }
            if name == "missing"
    ));
}

#[test]
fn dynamic_extensions_reject_without_any_extension_builder_work() {
    let custom_config = SolverConfig {
        phases: vec![custom("repair")],
        ..SolverConfig::default()
    };
    let error = compile_runtime_graph(
        &custom_config,
        RuntimeGraphInput::new(context(custom_config.random_seed), NoDynamicExtensions),
    )
    .expect_err("dynamic custom extensions are not emulated");
    assert!(matches!(
        error.kind,
        RuntimeCompileErrorKind::UnsupportedDynamicExtension {
            extension: RuntimeExtensionKind::Custom
        }
    ));

    let partitioned_config = SolverConfig {
        phases: vec![partitioned(Some("by_task"))],
        ..SolverConfig::default()
    };
    let error = compile_runtime_graph(
        &partitioned_config,
        RuntimeGraphInput::new(context(partitioned_config.random_seed), NoDynamicExtensions),
    )
    .expect_err("dynamic partitioned extensions are not emulated");
    assert!(matches!(
        error.kind,
        RuntimeCompileErrorKind::UnsupportedDynamicExtension {
            extension: RuntimeExtensionKind::Partitioned
        }
    ));
}

#[test]
fn compiler_rejects_a_context_seed_that_diverges_from_config() {
    let config = SolverConfig {
        random_seed: Some(41),
        ..SolverConfig::default()
    };
    let error = compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context(Some(42)), NoTypedExtensions),
    )
    .expect_err("config remains authoritative for extension context seed");
    assert!(matches!(
        error.kind,
        RuntimeCompileErrorKind::ContextSeedMismatch {
            config_seed: Some(41),
            context_seed: Some(42),
        }
    ));
}

#[test]
#[should_panic(expected = "custom phase `repair` was registered more than once")]
fn typed_extension_registry_rejects_duplicate_names_before_compilation() {
    let _ = context(None)
        .defaults()
        .phase("repair", |_| MarkerExtension)
        .phase("repair", |_| MarkerExtension);
}
