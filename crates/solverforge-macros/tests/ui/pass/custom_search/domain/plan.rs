use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::CrossEntityDistanceMeter;
use std::fmt::Debug;

use super::{Task, Worker};

#[planning_solution(constraints = "constraints", search = "search")]
pub struct Plan {
    #[problem_fact_collection]
    pub workers: Vec<Worker>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .penalize(HardSoftScore::ONE_SOFT)
            .named("penalize_tasks"),
    )
}

fn search<DM, IDM>(ctx: SearchContext<Plan, usize, DM, IDM>) -> impl Search<Plan, usize, DM, IDM>
where
    DM: CrossEntityDistanceMeter<Plan> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<Plan> + Clone + Debug + Send + 'static,
{
    ctx.defaults().phase("noop", |_| NoopPhase)
}

#[derive(Debug)]
struct NoopPhase;

impl solverforge::__internal::CustomSearchPhase<Plan> for NoopPhase {
    fn solve<D, ProgressCb>(
        &mut self,
        _solver_scope: &mut solverforge::__internal::SolverScope<'_, Plan, D, ProgressCb>,
    ) where
        D: solverforge::__internal::Director<Plan>,
        ProgressCb: solverforge::__internal::ProgressCallback<Plan>,
    {
    }

    fn phase_type_name(&self) -> &'static str {
        "noop"
    }
}
