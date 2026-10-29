// Everything here is named through `solverforge` alone: the prelude, the
// root, and the `config`, `stream` and `stats` modules.

use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::{CrossEntityDistanceMeter, ProgressCallback, SolverScope};

use super::{Resource, Task};

/// Draws the custom phase made from the solve's seeded generator.
pub static PROBE_PHASE_DRAWS: AtomicUsize = AtomicUsize::new(0);

#[planning_solution(constraints = "constraints", search = "search")]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned task"),
        TaskCount {
            constraint_ref: ConstraintRef::new("", "task count"),
        },
    )
}

fn search<DM, IDM>(ctx: SearchContext<Plan, usize, DM, IDM>) -> impl Search<Plan, usize, DM, IDM>
where
    DM: CrossEntityDistanceMeter<Plan> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<Plan> + Clone + Debug + Send + 'static,
{
    ctx.defaults().phase("probe", |_| ProbePhase)
}

/// A custom phase written against the facade's `SolverScope`.
#[derive(Debug)]
struct ProbePhase;

impl CustomSearchPhase<Plan> for ProbePhase {
    fn solve<D, ProgressCb>(&mut self, solver_scope: &mut SolverScope<'_, Plan, D, ProgressCb>)
    where
        D: Director<Plan>,
        ProgressCb: ProgressCallback<Plan>,
    {
        let task_count = solver_scope.working_solution().tasks.len();
        let task = solver_scope.random_below(task_count);
        assert!(task < task_count);
        let _ = solver_scope.random_u64();
        let _: &solverforge::stats::SolverStats = solver_scope.stats();
        PROBE_PHASE_DRAWS.fetch_add(1, Ordering::SeqCst);
    }

    fn phase_type_name(&self) -> &'static str {
        "probe"
    }
}

/// A hand-written constraint using only prelude names: one soft point per task.
struct TaskCount {
    constraint_ref: ConstraintRef,
}

impl SingleConstraint for TaskCount {}

impl IncrementalConstraint<Plan, HardSoftScore> for TaskCount {
    fn evaluate(&self, solution: &Plan) -> HardSoftScore {
        HardSoftScore::of_soft(-(solution.tasks.len() as i64))
    }

    fn match_count(&self, solution: &Plan) -> usize {
        solution.tasks.len()
    }

    fn initialize(&mut self, solution: &Plan) -> HardSoftScore {
        self.evaluate(solution)
    }

    fn on_insert(&mut self, _: &Plan, _: usize, _: usize) -> HardSoftScore {
        HardSoftScore::ZERO
    }

    fn on_retract(&mut self, _: &Plan, _: usize, _: usize) -> HardSoftScore {
        HardSoftScore::ZERO
    }

    fn reset(&mut self) {}

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }
}
