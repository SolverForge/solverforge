use std::fmt::Debug;
use std::sync::Mutex;

use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::{CrossEntityDistanceMeter, ProgressCallback, SolverConfig, SolverScope};

use super::{Resource, Task};

/// Tasks split into this many partitions, by task id modulo the count.
pub const PARTITIONS: usize = 4;

#[planning_solution(constraints = "constraints", config = "solver_config", search = "search")]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    /// The solver TOML a solve of this plan runs with; also the key its
    /// partition records are filed under.
    pub solver_toml: &'static str,
}

impl Plan {
    pub fn new(solver_toml: &'static str) -> Self {
        Self {
            resources: (0..3).map(|id| Resource { id }).collect(),
            tasks: (0..8)
                .map(|id| Task {
                    id,
                    resource_idx: None,
                })
                .collect(),
            score: None,
            solver_toml,
        }
    }
}

fn solver_config(solution: &Plan, _base: SolverConfig) -> SolverConfig {
    SolverConfig::from_toml_str(solution.solver_toml).expect("the test config parses")
}

/// Each unassigned task costs one hard point, each assigned task its
/// resource index plus one soft point.
fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned task"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_some())
            .penalize(|task: &Task| {
                HardSoftScore::of(0, task.resource_idx.unwrap_or_default() as i64 + 1)
            })
            .named("placement cost"),
    )
}

fn search<DM, IDM>(ctx: SearchContext<Plan, usize, DM, IDM>) -> impl Search<Plan, usize, DM, IDM>
where
    DM: CrossEntityDistanceMeter<Plan> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<Plan> + Clone + Debug + Send + 'static,
{
    ctx.defaults()
        .phase("record", |_| RecordPhase)
        .partitioner("by_task_id", |_| ByTaskId)
}

/// Splits the tasks by id modulo [`PARTITIONS`]; every partition keeps all
/// resources.
#[derive(Debug)]
struct ByTaskId;

impl SolutionPartitioner<Plan> for ByTaskId {
    fn partition(&self, solution: &Plan) -> Vec<Plan> {
        (0..PARTITIONS)
            .map(|part| Plan {
                tasks: solution
                    .tasks
                    .iter()
                    .filter(|task| task.id % PARTITIONS == part)
                    .cloned()
                    .collect(),
                ..solution.clone()
            })
            .collect()
    }

    fn merge(&self, original: &Plan, partitions: Vec<Plan>) -> Plan {
        let mut merged = original.clone();
        for task in partitions.into_iter().flat_map(|partition| partition.tasks) {
            let id = task.id;
            merged.tasks[id] = task;
        }
        merged
    }
}

/// What the `record` child phase saw in one partition.
#[derive(Clone, Debug)]
pub struct Record {
    pub task_ids: Vec<usize>,
    pub all_assigned: bool,
    pub thread: std::thread::ThreadId,
    /// Threads of the rayon pool the phase runs in.
    pub pool_threads: usize,
}

static RECORDS: Mutex<Vec<(&'static str, Record)>> = Mutex::new(Vec::new());

/// Removes and returns the records of solves run with `solver_toml`.
pub fn take_records(solver_toml: &'static str) -> Vec<Record> {
    let mut records = RECORDS.lock().unwrap();
    let (taken, kept) = records
        .drain(..)
        .partition::<Vec<_>, _>(|(key, _)| std::ptr::eq(*key, solver_toml));
    *records = kept;
    taken.into_iter().map(|(_, record)| record).collect()
}

/// A child phase recording the partition it runs on.
#[derive(Debug)]
struct RecordPhase;

impl CustomSearchPhase<Plan> for RecordPhase {
    fn solve<D, ProgressCb>(&mut self, solver_scope: &mut SolverScope<'_, Plan, D, ProgressCb>)
    where
        D: Director<Plan>,
        ProgressCb: ProgressCallback<Plan>,
    {
        let plan = solver_scope.working_solution();
        let record = Record {
            task_ids: plan.tasks.iter().map(|task| task.id).collect(),
            all_assigned: plan.tasks.iter().all(|task| task.resource_idx.is_some()),
            thread: std::thread::current().id(),
            pool_threads: rayon::current_num_threads(),
        };
        RECORDS.lock().unwrap().push((plan.solver_toml, record));
    }

    fn phase_type_name(&self) -> &'static str {
        "record"
    }
}
