use std::fs;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use solverforge::prelude::*;
use solverforge::{SolverConfig, SolverEvent, SolverManager, SolverTerminalReason};

static LAST_CONFIG_SECONDS: AtomicU64 = AtomicU64::new(0);
static LAST_BASE_RANDOM_SEED: AtomicU64 = AtomicU64::new(0);
static LAST_BASE_PHASE_COUNT: AtomicUsize = AtomicUsize::new(0);
static LAST_FINAL_RANDOM_SEED: AtomicU64 = AtomicU64::new(0);
static LAST_FINAL_PHASE_COUNT: AtomicUsize = AtomicUsize::new(0);
static LAST_FINAL_TERMINATION_SECONDS: AtomicU64 = AtomicU64::new(0);
static LAST_EXPLICIT_BASE_RANDOM_SEED: AtomicU64 = AtomicU64::new(0);
static LAST_EXPLICIT_BASE_PHASE_COUNT: AtomicUsize = AtomicUsize::new(0);
static LAST_EXPLICIT_FINAL_RANDOM_SEED: AtomicU64 = AtomicU64::new(0);
static LAST_EXPLICIT_FINAL_PHASE_COUNT: AtomicUsize = AtomicUsize::new(0);
static LAST_EXPLICIT_FINAL_TERMINATION_SECONDS: AtomicU64 = AtomicU64::new(0);

#[planning_entity]
struct DummyEntity {
    #[planning_id]
    id: usize,
}

#[problem_fact]
struct DummyVisit {
    #[planning_id]
    id: usize,
}

#[planning_entity]
struct DummyRoute {
    #[planning_id]
    id: usize,

    #[planning_list_variable(element_collection = "visits")]
    visits: Vec<usize>,
}

#[planning_solution(
    constraints = "define_constraints",
    config = "solver_config_for_solution"
)]
struct ConfigurableSolution {
    #[planning_entity_collection]
    entities: Vec<DummyEntity>,

    #[planning_score]
    score: Option<HardSoftScore>,

    time_limit_secs: u64,
}

#[planning_solution(
    constraints = "define_explicit_constraints",
    config = "solver_config_for_explicit_solution",
    solver_toml = "fixtures/configurable_solvable_solver.toml"
)]
struct ExplicitConfigurableSolution {
    #[planning_entity_collection]
    entities: Vec<DummyEntity>,

    #[planning_score]
    score: Option<HardSoftScore>,

    time_limit_secs: u64,
}

#[planning_solution(
    constraints = "define_explicit_list_constraints",
    config = "solver_config_for_explicit_list_solution",
    solver_toml = "fixtures/configurable_solvable_solver.toml"
)]
struct ExplicitListConfigurableSolution {
    #[problem_fact_collection]
    visits: Vec<DummyVisit>,

    #[planning_entity_collection]
    routes: Vec<DummyRoute>,

    #[planning_score]
    score: Option<HardSoftScore>,

    time_limit_secs: u64,
}

fn define_constraints() -> impl ConstraintSet<ConfigurableSolution, HardSoftScore> {
    (
        ConstraintFactory::<ConfigurableSolution, HardSoftScore>::new()
            .entities()
            .penalize_with(|_| HardSoftScore::of(0, 0))
            .named("noop"),
    )
}

fn define_explicit_constraints() -> impl ConstraintSet<ExplicitConfigurableSolution, HardSoftScore>
{
    (
        ConstraintFactory::<ExplicitConfigurableSolution, HardSoftScore>::new()
            .entities()
            .penalize_with(|_| HardSoftScore::of(0, 0))
            .named("noop"),
    )
}

fn define_explicit_list_constraints(
) -> impl ConstraintSet<ExplicitListConfigurableSolution, HardSoftScore> {
}

fn solver_config_for_solution(
    solution: &ConfigurableSolution,
    config: SolverConfig,
) -> SolverConfig {
    LAST_CONFIG_SECONDS.store(solution.time_limit_secs, Ordering::SeqCst);
    LAST_BASE_RANDOM_SEED.store(config.random_seed.unwrap_or_default(), Ordering::SeqCst);
    LAST_BASE_PHASE_COUNT.store(config.phases.len(), Ordering::SeqCst);

    let config = config.with_termination_seconds(solution.time_limit_secs);

    LAST_FINAL_RANDOM_SEED.store(config.random_seed.unwrap_or_default(), Ordering::SeqCst);
    LAST_FINAL_PHASE_COUNT.store(config.phases.len(), Ordering::SeqCst);
    LAST_FINAL_TERMINATION_SECONDS.store(
        config
            .time_limit()
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
        Ordering::SeqCst,
    );

    config
}

fn solver_config_for_explicit_solution(
    solution: &ExplicitConfigurableSolution,
    config: SolverConfig,
) -> SolverConfig {
    LAST_EXPLICIT_BASE_RANDOM_SEED.store(config.random_seed.unwrap_or_default(), Ordering::SeqCst);
    LAST_EXPLICIT_BASE_PHASE_COUNT.store(config.phases.len(), Ordering::SeqCst);

    let config = config.with_termination_seconds(solution.time_limit_secs);

    LAST_EXPLICIT_FINAL_RANDOM_SEED.store(config.random_seed.unwrap_or_default(), Ordering::SeqCst);
    LAST_EXPLICIT_FINAL_PHASE_COUNT.store(config.phases.len(), Ordering::SeqCst);
    LAST_EXPLICIT_FINAL_TERMINATION_SECONDS.store(
        config
            .time_limit()
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
        Ordering::SeqCst,
    );

    config
}

fn solver_config_for_explicit_list_solution(
    solution: &ExplicitListConfigurableSolution,
    config: SolverConfig,
) -> SolverConfig {
    config.with_termination_seconds(solution.time_limit_secs)
}

fn cwd_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TempSolverConfigDir {
    original_dir: std::path::PathBuf,
    temp_dir: std::path::PathBuf,
}

impl TempSolverConfigDir {
    fn new(contents: Option<&str>) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let original_dir = std::env::current_dir().expect("current directory should be readable");
        let suffix = COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir = std::env::temp_dir().join(format!(
            "solverforge-configurable-solution-{}-{suffix}",
            std::process::id()
        ));

        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("temp solver directory should be created");
        if let Some(contents) = contents {
            fs::write(temp_dir.join("solver.toml"), contents)
                .expect("solver.toml should be written");
        }
        std::env::set_current_dir(&temp_dir).expect("current directory should switch to temp");

        Self {
            original_dir,
            temp_dir,
        }
    }
}

impl Drop for TempSolverConfigDir {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original_dir)
            .expect("current directory should restore after test");
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

#[test]
fn planning_solution_config_provider_decorates_solver_toml_for_retained_runtime_solves() {
    static MANAGER: SolverManager<ConfigurableSolution> = SolverManager::new();

    let _cwd_lock = cwd_test_lock().lock().expect("cwd lock should be acquired");
    let _temp_solver_dir = TempSolverConfigDir::new(Some(
        r#"
random_seed = 19

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"
"#,
    ));

    let (job_id, mut receiver) = MANAGER
        .solve(ConfigurableSolution {
            entities: Vec::new(),
            score: None,
            time_limit_secs: 7,
        })
        .expect("job should start");

    let mut completed = false;

    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::BestSolution { .. } => {}
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(
                    metadata.terminal_reason,
                    Some(SolverTerminalReason::Completed)
                );
                assert_eq!(solution.score, Some(HardSoftScore::of(0, 0)));
                completed = true;
                break;
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    assert!(completed, "expected a completed event");
    assert_eq!(LAST_CONFIG_SECONDS.load(Ordering::SeqCst), 7);
    assert_eq!(LAST_BASE_RANDOM_SEED.load(Ordering::SeqCst), 19);
    assert_eq!(LAST_BASE_PHASE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(LAST_FINAL_RANDOM_SEED.load(Ordering::SeqCst), 19);
    assert_eq!(LAST_FINAL_PHASE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(LAST_FINAL_TERMINATION_SECONDS.load(Ordering::SeqCst), 7);

    MANAGER.delete(job_id).expect("delete completed job");
}

#[test]
fn planning_solution_solver_toml_path_is_independent_of_cwd() {
    static MANAGER: SolverManager<ExplicitConfigurableSolution> = SolverManager::new();

    let _cwd_lock = cwd_test_lock().lock().expect("cwd lock should be acquired");
    let _temp_solver_dir = TempSolverConfigDir::new(None);

    let (job_id, mut receiver) = MANAGER
        .solve(ExplicitConfigurableSolution {
            entities: Vec::new(),
            score: None,
            time_limit_secs: 11,
        })
        .expect("job should start");

    let mut completed = false;

    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::BestSolution { .. } => {}
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(
                    metadata.terminal_reason,
                    Some(SolverTerminalReason::Completed)
                );
                assert_eq!(solution.score, Some(HardSoftScore::of(0, 0)));
                completed = true;
                break;
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    assert!(completed, "expected a completed event");
    assert_eq!(LAST_EXPLICIT_BASE_RANDOM_SEED.load(Ordering::SeqCst), 23);
    assert_eq!(LAST_EXPLICIT_BASE_PHASE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(LAST_EXPLICIT_FINAL_RANDOM_SEED.load(Ordering::SeqCst), 23);
    assert_eq!(LAST_EXPLICIT_FINAL_PHASE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(
        LAST_EXPLICIT_FINAL_TERMINATION_SECONDS.load(Ordering::SeqCst),
        11
    );

    MANAGER.delete(job_id).expect("delete completed job");
}

#[test]
fn planning_solution_solver_toml_path_compiles_and_runs_for_list_only_solutions() {
    static MANAGER: SolverManager<ExplicitListConfigurableSolution> = SolverManager::new();

    let _cwd_lock = cwd_test_lock().lock().expect("cwd lock should be acquired");
    let _temp_solver_dir = TempSolverConfigDir::new(None);

    let (job_id, mut receiver) = MANAGER
        .solve(ExplicitListConfigurableSolution {
            visits: Vec::new(),
            routes: Vec::new(),
            score: None,
            time_limit_secs: 3,
        })
        .expect("job should start");

    let mut completed = false;

    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::BestSolution { .. } => {}
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(
                    metadata.terminal_reason,
                    Some(SolverTerminalReason::Completed)
                );
                assert_eq!(solution.score, Some(HardSoftScore::of(0, 0)));
                completed = true;
                break;
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    assert!(completed, "expected a completed event");

    MANAGER.delete(job_id).expect("delete completed job");
}
