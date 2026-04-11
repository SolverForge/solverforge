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

#[planning_entity]
struct DummyEntity {
    #[planning_id]
    id: usize,
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

fn define_constraints() -> impl ConstraintSet<ConfigurableSolution, HardSoftScore> {
    (
        ConstraintFactory::<ConfigurableSolution, HardSoftScore>::new()
            .entities()
            .penalize_with(|_| HardSoftScore::of(0, 0))
            .named("noop"),
    )
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

fn cwd_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TempSolverConfigDir {
    original_dir: std::path::PathBuf,
    temp_dir: std::path::PathBuf,
}

impl TempSolverConfigDir {
    fn new(contents: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let original_dir = std::env::current_dir().expect("current directory should be readable");
        let suffix = COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir = std::env::temp_dir().join(format!(
            "solverforge-configurable-solution-{}-{suffix}",
            std::process::id()
        ));

        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("temp solver directory should be created");
        fs::write(temp_dir.join("solver.toml"), contents).expect("solver.toml should be written");
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
    let _temp_solver_dir = TempSolverConfigDir::new(
        r#"
random_seed = 19

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"
"#,
    );

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
