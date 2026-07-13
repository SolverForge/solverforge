#[derive(Clone, Debug)]
struct Worker;

#[derive(Clone, Debug)]
struct Task {
    worker_idx: Option<usize>,
}

#[derive(Clone, Debug)]
struct Plan {
    workers: Vec<Worker>,
    tasks: Vec<Task>,
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

#[derive(Clone, Debug)]
struct RestrictedTask {
    worker_idx: Option<usize>,
    allowed_workers: Vec<usize>,
}

#[derive(Clone, Debug)]
struct RestrictedPlan {
    workers: Vec<Worker>,
    tasks: Vec<RestrictedTask>,
    score: Option<SoftScore>,
}

impl PlanningSolution for RestrictedPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct PlanScoreDirector {
    working_solution: Plan,
    descriptor: SolutionDescriptor,
    score_mode: PlanScoreMode,
}

#[derive(Clone, Copy, Debug)]
enum PlanScoreMode {
    AllAssignedBonus,
    ByWorker {
        unassigned_score: i64,
        assigned_scores: [i64; 3],
    },
}

impl PlanScoreDirector {
    fn new(solution: Plan, descriptor: SolutionDescriptor) -> Self {
        Self::with_mode(solution, descriptor, PlanScoreMode::AllAssignedBonus)
    }

    fn with_mode(
        solution: Plan,
        descriptor: SolutionDescriptor,
        score_mode: PlanScoreMode,
    ) -> Self {
        Self {
            working_solution: solution,
            descriptor,
            score_mode,
        }
    }

}

impl solverforge_scoring::Director<Plan> for PlanScoreDirector {
    fn working_solution(&self) -> &Plan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut Plan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = match self.score_mode {
            PlanScoreMode::AllAssignedBonus => {
                if self
                    .working_solution
                    .tasks
                    .iter()
                    .all(|task| task.worker_idx.is_some())
                {
                    SoftScore::of(10)
                } else {
                    SoftScore::of(0)
                }
            }
            PlanScoreMode::ByWorker {
                unassigned_score,
                assigned_scores,
            } => SoftScore::of(
                self.working_solution.tasks[0]
                    .worker_idx
                    .map(|worker_idx| assigned_scores[worker_idx])
                    .unwrap_or(unassigned_score),
            ),
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> Plan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.tasks.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.tasks.len())
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

fn get_worker_idx(entity: &dyn Any) -> Option<usize> {
    entity
        .downcast_ref::<Task>()
        .expect("task expected")
        .worker_idx
}

fn set_worker_idx(entity: &mut dyn Any, value: Option<usize>) {
    entity
        .downcast_mut::<Task>()
        .expect("task expected")
        .worker_idx = value;
}

fn nearby_worker_value_distance(solution: &dyn Any, entity_index: usize, value: usize) -> f64 {
    let plan = solution.downcast_ref::<Plan>().expect("plan expected");
    let current = plan.tasks[entity_index].worker_idx.unwrap_or(0);
    current.abs_diff(value) as f64
}

fn nearby_worker_candidates(solution: &dyn Any, entity_index: usize, _variable_index: usize) -> &[usize] {
    let plan = solution.downcast_ref::<Plan>().expect("plan expected");
    match (entity_index, plan.workers.len()) {
        (0, 3..) => &[1, 2],
        (1, 3..) => &[0, 2],
        (0, 2..) => &[1],
        (1, 2..) => &[0],
        _ => &[],
    }
}

fn nearby_task_candidates(solution: &dyn Any, entity_index: usize, _variable_index: usize) -> &[usize] {
    let _ = solution.downcast_ref::<Plan>().expect("plan expected");
    match entity_index {
        0 => &[1, 2],
        1 => &[2],
        _ => &[],
    }
}

fn restricted_nearby_task_candidates(
    solution: &dyn Any,
    entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    let _ = solution
        .downcast_ref::<RestrictedPlan>()
        .expect("restricted plan expected");
    match entity_index {
        0 => &[1, 2],
        1 => &[2],
        _ => &[],
    }
}

fn nearby_worker_entity_distance(_solution: &dyn Any, left: usize, right: usize) -> f64 {
    match (left, right) {
        (0, 1) => 0.0,
        (0, 2) => 1.0,
        (1, 2) => 0.5,
        _ => left.abs_diff(right) as f64,
    }
}

fn restricted_get_worker_idx(entity: &dyn Any) -> Option<usize> {
    entity
        .downcast_ref::<RestrictedTask>()
        .expect("restricted task expected")
        .worker_idx
}

fn restricted_set_worker_idx(entity: &mut dyn Any, value: Option<usize>) {
    entity
        .downcast_mut::<RestrictedTask>()
        .expect("restricted task expected")
        .worker_idx = value;
}

fn restricted_allowed_workers(entity: &dyn Any) -> Vec<usize> {
    entity
        .downcast_ref::<RestrictedTask>()
        .expect("restricted task expected")
        .allowed_workers
        .clone()
}

static PANIC_ON_RESTRICTED_ALLOWED_WORKERS: AtomicBool = AtomicBool::new(false);

struct RestrictedAllowedWorkersPanicGuard;

impl RestrictedAllowedWorkersPanicGuard {
    fn enable() -> Self {
        PANIC_ON_RESTRICTED_ALLOWED_WORKERS.store(true, Ordering::SeqCst);
        Self
    }
}

impl Drop for RestrictedAllowedWorkersPanicGuard {
    fn drop(&mut self) {
        PANIC_ON_RESTRICTED_ALLOWED_WORKERS.store(false, Ordering::SeqCst);
    }
}

fn restricted_allowed_workers_panic_after_index(entity: &dyn Any) -> Vec<usize> {
    assert!(
        !PANIC_ON_RESTRICTED_ALLOWED_WORKERS.load(Ordering::SeqCst),
        "descriptor swap move rescanned entity value ranges after selector indexing"
    );
    restricted_allowed_workers(entity)
}
