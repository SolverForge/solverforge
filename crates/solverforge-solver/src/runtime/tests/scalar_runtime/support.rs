#[derive(Clone, Debug)]
struct ScalarRuntimeWorker;

#[derive(Clone, Debug)]
struct ScalarRuntimeTask {
    worker_idx: Option<usize>,
}

#[derive(Clone, Debug)]
struct ScalarRuntimePlan {
    score: Option<SoftScore>,
    workers: Vec<ScalarRuntimeWorker>,
    tasks: Vec<ScalarRuntimeTask>,
}

#[derive(Clone, Debug)]
struct ScalarRuntimeDirector {
    working_solution: ScalarRuntimePlan,
    descriptor: SolutionDescriptor,
    score_mode: ScalarRuntimeScoreMode,
}

#[derive(Clone, Copy, Debug)]
enum ScalarRuntimeScoreMode {
    PreferUnassigned,
    ByWorker {
        unassigned_score: i64,
        assigned_scores: [i64; 3],
    },
}

impl ScalarRuntimeDirector {
    fn new(working_solution: ScalarRuntimePlan, descriptor: SolutionDescriptor) -> Self {
        Self::with_score_mode(
            working_solution,
            descriptor,
            ScalarRuntimeScoreMode::PreferUnassigned,
        )
    }

    fn with_score_mode(
        working_solution: ScalarRuntimePlan,
        descriptor: SolutionDescriptor,
        score_mode: ScalarRuntimeScoreMode,
    ) -> Self {
        Self {
            working_solution,
            descriptor,
            score_mode,
        }
    }
}

impl PlanningSolution for ScalarRuntimePlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Director<ScalarRuntimePlan> for ScalarRuntimeDirector {
    fn working_solution(&self) -> &ScalarRuntimePlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut ScalarRuntimePlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = match self.score_mode {
            ScalarRuntimeScoreMode::PreferUnassigned => {
                if self.working_solution.tasks[0].worker_idx.is_none() {
                    SoftScore::of(0)
                } else {
                    SoftScore::of(-1)
                }
            }
            ScalarRuntimeScoreMode::ByWorker {
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

    fn clone_working_solution(&self) -> ScalarRuntimePlan {
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

fn get_runtime_worker_idx(entity: &dyn std::any::Any) -> Option<usize> {
    entity
        .downcast_ref::<ScalarRuntimeTask>()
        .expect("task expected")
        .worker_idx
}

fn set_runtime_worker_idx(entity: &mut dyn std::any::Any, value: Option<usize>) {
    entity
        .downcast_mut::<ScalarRuntimeTask>()
        .expect("task expected")
        .worker_idx = value;
}

fn scalar_runtime_descriptor_with_allows_unassigned(allows_unassigned: bool) -> SolutionDescriptor {
    SolutionDescriptor::new("ScalarRuntimePlan", TypeId::of::<ScalarRuntimePlan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<ScalarRuntimeTask>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |solution: &ScalarRuntimePlan| &solution.tasks,
                    |solution: &mut ScalarRuntimePlan| &mut solution.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(allows_unassigned)
                        .with_value_range("workers")
                        .with_usize_accessors(get_runtime_worker_idx, set_runtime_worker_idx),
                ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<ScalarRuntimeWorker>(), "workers")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |solution: &ScalarRuntimePlan| &solution.workers,
                    |solution: &mut ScalarRuntimePlan| &mut solution.workers,
                ))),
        )
}

fn scalar_runtime_descriptor() -> SolutionDescriptor {
    scalar_runtime_descriptor_with_allows_unassigned(true)
}

fn scalar_runtime_task_count(solution: &ScalarRuntimePlan) -> usize {
    solution.tasks.len()
}

fn scalar_runtime_worker_count(solution: &ScalarRuntimePlan, _provider_index: usize) -> usize {
    solution.workers.len()
}

fn scalar_runtime_worker_get(
    solution: &ScalarRuntimePlan,
    entity_index: usize,
    _variable_index: usize,
) -> Option<usize> {
    solution.tasks[entity_index].worker_idx
}

fn scalar_runtime_worker_set(
    solution: &mut ScalarRuntimePlan,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.tasks[entity_index].worker_idx = value;
}

fn scalar_runtime_value_order_key(
    _solution: &ScalarRuntimePlan,
    _entity_index: usize,
    _variable_index: usize,
    value: usize,
) -> Option<i64> {
    Some(value as i64)
}

fn scalar_runtime_model_with_allows_unassigned(
    allows_unassigned: bool,
) -> ModelContext<ScalarRuntimePlan, usize, DefaultMeter, DefaultMeter> {
    scalar_runtime_model_with_hooks(allows_unassigned, None)
}

fn scalar_runtime_model_with_hooks(
    allows_unassigned: bool,
    value_order_key: Option<fn(&ScalarRuntimePlan, usize, usize, usize) -> Option<i64>>,
) -> ModelContext<ScalarRuntimePlan, usize, DefaultMeter, DefaultMeter> {
    let mut ctx = ScalarVariableContext::new(
        0,
        0,
        "Task",
        scalar_runtime_task_count,
        "worker_idx",
        scalar_runtime_worker_get,
        scalar_runtime_worker_set,
        ValueSource::SolutionCount {
            count_fn: scalar_runtime_worker_count,
            provider_index: 0,
        },
        allows_unassigned,
    );
    if let Some(order_key) = value_order_key {
        ctx = ctx.with_construction_value_order_key(order_key);
    }
    ModelContext::new(vec![VariableContext::Scalar(ctx)])
}

fn scalar_runtime_model() -> ModelContext<ScalarRuntimePlan, usize, DefaultMeter, DefaultMeter> {
    scalar_runtime_model_with_allows_unassigned(true)
}
