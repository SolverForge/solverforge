use super::Construction;
use crate::builder::{
    ListVariableContext, ModelContext, ScalarVariableContext, ValueSource, VariableContext,
};
use crate::descriptor_scalar::{scalar_target_matches, scalar_work_remaining_with_frontier};
use crate::phase::Phase;
use crate::scope::SolverScope;
use crate::DefaultCrossEntityDistanceMeter;
use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, ProblemFactDescriptor,
    SolutionDescriptor, VariableDescriptor, VariableType,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, ScoreDirector};
use std::any::TypeId;

type DefaultMeter = DefaultCrossEntityDistanceMeter;

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

fn scalar_variable(name: &'static str) -> VariableDescriptor {
    VariableDescriptor {
        name,
        variable_type: VariableType::Genuine,
        allows_unassigned: true,
        value_range_provider: Some("values"),
        value_range_type: solverforge_core::domain::ValueRangeType::Collection,
        source_variable: None,
        source_entity: None,
        usize_getter: Some(|_| None),
        usize_setter: Some(|_, _| {}),
        entity_value_provider: Some(|_| vec![1]),
        nearby_value_distance_meter: None,
        nearby_entity_distance_meter: None,
    }
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(
            EntityDescriptor::new("Route", TypeId::of::<()>(), "routes")
                .with_variable(scalar_variable("vehicle_id"))
                .with_variable(VariableDescriptor::list("visits")),
        )
        .with_entity(
            EntityDescriptor::new("Shift", TypeId::of::<u8>(), "shifts")
                .with_variable(scalar_variable("employee_id")),
        )
}

fn config(
    construction_heuristic_type: ConstructionHeuristicType,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type,
        target: VariableTargetConfig {
            entity_class: entity_class.map(str::to_owned),
            variable_name: variable_name.map(str::to_owned),
        },
        k: 2,
        termination: None,
    }
}

#[test]
fn scalar_target_matches_entity_class_only_target() {
    let descriptor = descriptor();
    assert!(scalar_target_matches(&descriptor, Some("Route"), None));
}

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

fn scalar_runtime_worker_count(solution: &ScalarRuntimePlan) -> usize {
    solution.workers.len()
}

fn scalar_runtime_worker_get(solution: &ScalarRuntimePlan, entity_index: usize) -> Option<usize> {
    solution.tasks[entity_index].worker_idx
}

fn scalar_runtime_worker_set(
    solution: &mut ScalarRuntimePlan,
    entity_index: usize,
    value: Option<usize>,
) {
    solution.tasks[entity_index].worker_idx = value;
}

fn scalar_runtime_model_with_allows_unassigned(
    allows_unassigned: bool,
) -> ModelContext<ScalarRuntimePlan, usize, DefaultMeter, DefaultMeter> {
    ModelContext::new(vec![VariableContext::Scalar(ScalarVariableContext::new(
        0,
        "Task",
        scalar_runtime_task_count,
        "worker_idx",
        scalar_runtime_worker_get,
        scalar_runtime_worker_set,
        ValueSource::SolutionCount {
            count_fn: scalar_runtime_worker_count,
        },
        allows_unassigned,
    ))])
}

fn scalar_runtime_model() -> ModelContext<ScalarRuntimePlan, usize, DefaultMeter, DefaultMeter> {
    scalar_runtime_model_with_allows_unassigned(true)
}

#[test]
fn scalar_runtime_frontier_marks_kept_optional_none_as_complete() {
    let descriptor = scalar_runtime_descriptor();
    let plan = ScalarRuntimePlan {
        score: None,
        workers: vec![ScalarRuntimeWorker],
        tasks: vec![ScalarRuntimeTask { worker_idx: None }],
    };
    let director = ScalarRuntimeDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut targeted_phase = Construction::new(
        Some(config(
            ConstructionHeuristicType::CheapestInsertion,
            Some("Task"),
            Some("worker_idx"),
        )),
        descriptor.clone(),
        scalar_runtime_model(),
    );
    targeted_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(solver_scope.stats().step_count, 1);
    assert!(
        !scalar_work_remaining_with_frontier(
            &descriptor,
            solver_scope.construction_frontier(),
            solver_scope.solution_revision(),
            None,
            None,
            solver_scope.working_solution(),
        ),
        "completed optional None should not be treated as remaining scalar work",
    );

    let mut untargeted_phase = Construction::new(None, descriptor, scalar_runtime_model());
    untargeted_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(solver_scope.stats().step_count, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 0);
}

#[test]
fn no_op_runtime_construction_still_seeds_score_and_best_solution() {
    let descriptor = scalar_runtime_descriptor();
    let plan = ScalarRuntimePlan {
        score: None,
        workers: vec![ScalarRuntimeWorker],
        tasks: vec![ScalarRuntimeTask {
            worker_idx: Some(0),
        }],
    };
    let director = ScalarRuntimeDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(None, descriptor, scalar_runtime_model());
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(-1))
    );
    assert_eq!(solver_scope.best_score().copied(), Some(SoftScore::of(-1)));
}

#[test]
fn scalar_runtime_first_fit_keeps_none_when_optional_baseline_is_not_beaten() {
    let descriptor = scalar_runtime_descriptor();
    let plan = ScalarRuntimePlan {
        score: None,
        workers: vec![
            ScalarRuntimeWorker,
            ScalarRuntimeWorker,
            ScalarRuntimeWorker,
        ],
        tasks: vec![ScalarRuntimeTask { worker_idx: None }],
    };
    let director = ScalarRuntimeDirector::with_score_mode(
        plan,
        descriptor.clone(),
        ScalarRuntimeScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [-5, -1, -2],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(None, descriptor, scalar_runtime_model());
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(0))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 0);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn scalar_runtime_first_fit_skips_worse_candidate_for_later_improvement() {
    let descriptor = scalar_runtime_descriptor();
    let plan = ScalarRuntimePlan {
        score: None,
        workers: vec![
            ScalarRuntimeWorker,
            ScalarRuntimeWorker,
            ScalarRuntimeWorker,
        ],
        tasks: vec![ScalarRuntimeTask { worker_idx: None }],
    };
    let director = ScalarRuntimeDirector::with_score_mode(
        plan,
        descriptor.clone(),
        ScalarRuntimeScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [-5, 7, -1],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(None, descriptor, scalar_runtime_model());
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(7))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn scalar_runtime_first_fit_takes_first_improving_candidate() {
    let descriptor = scalar_runtime_descriptor();
    let plan = ScalarRuntimePlan {
        score: None,
        workers: vec![
            ScalarRuntimeWorker,
            ScalarRuntimeWorker,
            ScalarRuntimeWorker,
        ],
        tasks: vec![ScalarRuntimeTask { worker_idx: None }],
    };
    let director = ScalarRuntimeDirector::with_score_mode(
        plan,
        descriptor.clone(),
        ScalarRuntimeScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [7, -5, 3],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(None, descriptor, scalar_runtime_model());
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(7))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn scalar_runtime_first_fit_required_slot_still_assigns_first_doable() {
    let descriptor = scalar_runtime_descriptor_with_allows_unassigned(false);
    let plan = ScalarRuntimePlan {
        score: None,
        workers: vec![
            ScalarRuntimeWorker,
            ScalarRuntimeWorker,
            ScalarRuntimeWorker,
        ],
        tasks: vec![ScalarRuntimeTask { worker_idx: None }],
    };
    let director = ScalarRuntimeDirector::with_score_mode(
        plan,
        descriptor.clone(),
        ScalarRuntimeScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [-5, -1, -2],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        None,
        descriptor,
        scalar_runtime_model_with_allows_unassigned(false),
    );
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[derive(Clone, Debug)]
struct RevisionWorker;

#[derive(Clone, Debug)]
struct RevisionTask {
    worker_idx: Option<usize>,
}

#[derive(Clone, Debug)]
struct RevisionPlan {
    score: Option<SoftScore>,
    workers: Vec<RevisionWorker>,
    tasks: Vec<RevisionTask>,
    routes: Vec<Vec<usize>>,
    route_pool: Vec<usize>,
}

#[derive(Clone, Debug)]
struct RevisionDirector {
    working_solution: RevisionPlan,
    descriptor: SolutionDescriptor,
}

impl PlanningSolution for RevisionPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Director<RevisionPlan> for RevisionDirector {
    fn working_solution(&self) -> &RevisionPlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut RevisionPlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let route_ready = !self.working_solution.routes[0].is_empty();
        let assigned = self.working_solution.tasks[0].worker_idx.is_some();
        let score = match (route_ready, assigned) {
            (false, false) => SoftScore::of(0),
            (false, true) => SoftScore::of(-1),
            (true, false) => SoftScore::of(0),
            (true, true) => SoftScore::of(10),
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> RevisionPlan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        match descriptor_index {
            0 => Some(self.working_solution.tasks.len()),
            1 => Some(self.working_solution.routes.len()),
            _ => None,
        }
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.tasks.len() + self.working_solution.routes.len())
    }
}

fn revision_task_getter(entity: &dyn std::any::Any) -> Option<usize> {
    entity
        .downcast_ref::<RevisionTask>()
        .expect("task expected")
        .worker_idx
}

fn revision_task_setter(entity: &mut dyn std::any::Any, value: Option<usize>) {
    entity
        .downcast_mut::<RevisionTask>()
        .expect("task expected")
        .worker_idx = value;
}

fn revision_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("RevisionPlan", TypeId::of::<RevisionPlan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<RevisionTask>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |solution: &RevisionPlan| &solution.tasks,
                    |solution: &mut RevisionPlan| &mut solution.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_value_range("workers")
                        .with_usize_accessors(revision_task_getter, revision_task_setter),
                ),
        )
        .with_entity(
            EntityDescriptor::new("Route", TypeId::of::<Vec<usize>>(), "routes").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Route",
                    "routes",
                    |solution: &RevisionPlan| &solution.routes,
                    |solution: &mut RevisionPlan| &mut solution.routes,
                )),
            ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<RevisionWorker>(), "workers")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |solution: &RevisionPlan| &solution.workers,
                    |solution: &mut RevisionPlan| &mut solution.workers,
                ))),
        )
}

fn revision_task_count(solution: &RevisionPlan) -> usize {
    solution.tasks.len()
}

fn revision_worker_count(solution: &RevisionPlan) -> usize {
    solution.workers.len()
}

fn revision_worker_get(solution: &RevisionPlan, entity_index: usize) -> Option<usize> {
    solution.tasks[entity_index].worker_idx
}

fn revision_worker_set(solution: &mut RevisionPlan, entity_index: usize, value: Option<usize>) {
    solution.tasks[entity_index].worker_idx = value;
}

fn revision_route_count(solution: &RevisionPlan) -> usize {
    solution.routes.len()
}

fn revision_route_element_count(solution: &RevisionPlan) -> usize {
    solution.route_pool.len()
}

fn revision_assigned_route_elements(solution: &RevisionPlan) -> Vec<usize> {
    solution
        .routes
        .iter()
        .flat_map(|route| route.iter().copied())
        .collect()
}

fn revision_route_len(solution: &RevisionPlan, entity_index: usize) -> usize {
    solution.routes[entity_index].len()
}

fn revision_route_remove(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
) -> Option<usize> {
    let route = solution.routes.get_mut(entity_index)?;
    (pos < route.len()).then(|| route.remove(pos))
}

fn revision_route_remove_for_construction(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].remove(pos)
}

fn revision_route_insert(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].insert(pos, value);
}

fn revision_route_get(solution: &RevisionPlan, entity_index: usize, pos: usize) -> Option<usize> {
    solution.routes[entity_index].get(pos).copied()
}

fn revision_route_set(solution: &mut RevisionPlan, entity_index: usize, pos: usize, value: usize) {
    solution.routes[entity_index][pos] = value;
}

fn revision_route_reverse(
    solution: &mut RevisionPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) {
    solution.routes[entity_index][start..end].reverse();
}

fn revision_route_sublist_remove(
    solution: &mut RevisionPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) -> Vec<usize> {
    solution.routes[entity_index].drain(start..end).collect()
}

fn revision_route_sublist_insert(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
    values: Vec<usize>,
) {
    solution.routes[entity_index].splice(pos..pos, values);
}

fn revision_route_ruin_remove(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].remove(pos)
}

fn revision_route_ruin_insert(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].insert(pos, value);
}

fn revision_route_index_to_element(solution: &RevisionPlan, idx: usize) -> usize {
    solution.route_pool[idx]
}

fn revision_model() -> ModelContext<RevisionPlan, usize, DefaultMeter, DefaultMeter> {
    ModelContext::new(vec![
        VariableContext::Scalar(ScalarVariableContext::new(
            0,
            "Task",
            revision_task_count,
            "worker_idx",
            revision_worker_get,
            revision_worker_set,
            ValueSource::SolutionCount {
                count_fn: revision_worker_count,
            },
            true,
        )),
        VariableContext::List(ListVariableContext::new(
            "Route",
            revision_route_element_count,
            revision_assigned_route_elements,
            revision_route_len,
            revision_route_remove,
            revision_route_remove_for_construction,
            revision_route_insert,
            revision_route_get,
            revision_route_set,
            revision_route_reverse,
            revision_route_sublist_remove,
            revision_route_sublist_insert,
            revision_route_ruin_remove,
            revision_route_ruin_insert,
            revision_route_index_to_element,
            revision_route_count,
            DefaultMeter::default(),
            DefaultMeter::default(),
            "visits",
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )),
    ])
}

#[test]
fn generic_mixed_phase_reopens_optional_none_after_list_commit() {
    let descriptor = revision_descriptor();
    let plan = RevisionPlan {
        score: None,
        workers: vec![RevisionWorker],
        tasks: vec![RevisionTask { worker_idx: None }],
        routes: vec![Vec::new()],
        route_pool: vec![10],
    };
    let director = RevisionDirector {
        working_solution: plan,
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(None, descriptor, revision_model());
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().routes[0], vec![10]);
    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
    assert_eq!(solver_scope.stats().moves_accepted, 2);
}

#[derive(Clone, Debug, Default)]
struct MultiOwnerSolution {
    score: Option<SoftScore>,
    routes: Vec<Vec<usize>>,
    shifts: Vec<Vec<usize>>,
    route_pool: Vec<usize>,
    shift_pool: Vec<usize>,
    log: Vec<&'static str>,
}

impl PlanningSolution for MultiOwnerSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn multi_owner_solution() -> MultiOwnerSolution {
    MultiOwnerSolution {
        score: None,
        routes: vec![Vec::new()],
        shifts: vec![Vec::new()],
        route_pool: vec![10, 11],
        shift_pool: vec![20, 21],
        log: Vec::new(),
    }
}

fn multi_owner_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("MultiOwnerSolution", TypeId::of::<MultiOwnerSolution>())
        .with_entity(
            EntityDescriptor::new("Route", TypeId::of::<Vec<usize>>(), "routes").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Route",
                    "routes",
                    |solution: &MultiOwnerSolution| &solution.routes,
                    |solution: &mut MultiOwnerSolution| &mut solution.routes,
                )),
            ),
        )
        .with_entity(
            EntityDescriptor::new("Shift", TypeId::of::<Vec<usize>>(), "shifts").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Shift",
                    "shifts",
                    |solution: &MultiOwnerSolution| &solution.shifts,
                    |solution: &mut MultiOwnerSolution| &mut solution.shifts,
                )),
            ),
        )
}

fn route_entity_count(solution: &MultiOwnerSolution) -> usize {
    solution.routes.len()
}

fn route_element_count(solution: &MultiOwnerSolution) -> usize {
    solution.route_pool.len()
}

fn assigned_route_elements(solution: &MultiOwnerSolution) -> Vec<usize> {
    solution
        .routes
        .iter()
        .flat_map(|route| route.iter().copied())
        .collect()
}

fn route_len(solution: &MultiOwnerSolution, entity_index: usize) -> usize {
    solution.routes[entity_index].len()
}

fn route_remove(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    pos: usize,
) -> Option<usize> {
    let route = solution.routes.get_mut(entity_index)?;
    (pos < route.len()).then(|| route.remove(pos))
}

fn route_remove_for_construction(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].remove(pos)
}

fn route_insert(solution: &mut MultiOwnerSolution, entity_index: usize, pos: usize, value: usize) {
    solution.log.push("Route");
    solution.routes[entity_index].insert(pos, value);
}

fn route_get(solution: &MultiOwnerSolution, entity_index: usize, pos: usize) -> Option<usize> {
    solution.routes[entity_index].get(pos).copied()
}

fn route_set(solution: &mut MultiOwnerSolution, entity_index: usize, pos: usize, value: usize) {
    solution.routes[entity_index][pos] = value;
}

fn route_reverse(solution: &mut MultiOwnerSolution, entity_index: usize, start: usize, end: usize) {
    solution.routes[entity_index][start..end].reverse();
}

fn route_sublist_remove(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    start: usize,
    end: usize,
) -> Vec<usize> {
    solution.routes[entity_index].drain(start..end).collect()
}

fn route_sublist_insert(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    pos: usize,
    values: Vec<usize>,
) {
    solution.routes[entity_index].splice(pos..pos, values);
}

fn route_ruin_remove(solution: &mut MultiOwnerSolution, entity_index: usize, pos: usize) -> usize {
    solution.routes[entity_index].remove(pos)
}

fn route_ruin_insert(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].insert(pos, value);
}

fn route_index_to_element(solution: &MultiOwnerSolution, idx: usize) -> usize {
    solution.route_pool[idx]
}

fn shift_entity_count(solution: &MultiOwnerSolution) -> usize {
    solution.shifts.len()
}

fn shift_element_count(solution: &MultiOwnerSolution) -> usize {
    solution.shift_pool.len()
}

fn assigned_shift_elements(solution: &MultiOwnerSolution) -> Vec<usize> {
    solution
        .shifts
        .iter()
        .flat_map(|shift| shift.iter().copied())
        .collect()
}

fn shift_len(solution: &MultiOwnerSolution, entity_index: usize) -> usize {
    solution.shifts[entity_index].len()
}

fn shift_remove(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    pos: usize,
) -> Option<usize> {
    let shift = solution.shifts.get_mut(entity_index)?;
    (pos < shift.len()).then(|| shift.remove(pos))
}

fn shift_remove_for_construction(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.shifts[entity_index].remove(pos)
}

fn shift_insert(solution: &mut MultiOwnerSolution, entity_index: usize, pos: usize, value: usize) {
    solution.log.push("Shift");
    solution.shifts[entity_index].insert(pos, value);
}

fn shift_get(solution: &MultiOwnerSolution, entity_index: usize, pos: usize) -> Option<usize> {
    solution.shifts[entity_index].get(pos).copied()
}

fn shift_set(solution: &mut MultiOwnerSolution, entity_index: usize, pos: usize, value: usize) {
    solution.shifts[entity_index][pos] = value;
}

fn shift_reverse(solution: &mut MultiOwnerSolution, entity_index: usize, start: usize, end: usize) {
    solution.shifts[entity_index][start..end].reverse();
}

fn shift_sublist_remove(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    start: usize,
    end: usize,
) -> Vec<usize> {
    solution.shifts[entity_index].drain(start..end).collect()
}

fn shift_sublist_insert(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    pos: usize,
    values: Vec<usize>,
) {
    solution.shifts[entity_index].splice(pos..pos, values);
}

fn shift_ruin_remove(solution: &mut MultiOwnerSolution, entity_index: usize, pos: usize) -> usize {
    solution.shifts[entity_index].remove(pos)
}

fn shift_ruin_insert(
    solution: &mut MultiOwnerSolution,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.shifts[entity_index].insert(pos, value);
}

fn shift_index_to_element(solution: &MultiOwnerSolution, idx: usize) -> usize {
    solution.shift_pool[idx]
}

fn multi_owner_model() -> ModelContext<MultiOwnerSolution, usize, DefaultMeter, DefaultMeter> {
    ModelContext::new(vec![
        VariableContext::List(ListVariableContext::new(
            "Route",
            route_element_count,
            assigned_route_elements,
            route_len,
            route_remove,
            route_remove_for_construction,
            route_insert,
            route_get,
            route_set,
            route_reverse,
            route_sublist_remove,
            route_sublist_insert,
            route_ruin_remove,
            route_ruin_insert,
            route_index_to_element,
            route_entity_count,
            DefaultMeter::default(),
            DefaultMeter::default(),
            "tasks",
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )),
        VariableContext::List(ListVariableContext::new(
            "Shift",
            shift_element_count,
            assigned_shift_elements,
            shift_len,
            shift_remove,
            shift_remove_for_construction,
            shift_insert,
            shift_get,
            shift_set,
            shift_reverse,
            shift_sublist_remove,
            shift_sublist_insert,
            shift_ruin_remove,
            shift_ruin_insert,
            shift_index_to_element,
            shift_entity_count,
            DefaultMeter::default(),
            DefaultMeter::default(),
            "tasks",
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )),
    ])
}

fn multi_owner_scope(
    solution: MultiOwnerSolution,
) -> SolverScope<'static, MultiOwnerSolution, ScoreDirector<MultiOwnerSolution, ()>> {
    let director = ScoreDirector::simple(
        solution,
        multi_owner_descriptor(),
        |solution, descriptor_index| match descriptor_index {
            0 => solution.routes.len(),
            1 => solution.shifts.len(),
            _ => 0,
        },
    );
    SolverScope::new(director)
}

fn solve_multi_owner_construction(config: ConstructionHeuristicConfig) -> MultiOwnerSolution {
    solve_multi_owner_construction_with_solution(multi_owner_solution(), config)
}

fn solve_multi_owner_construction_with_solution(
    solution: MultiOwnerSolution,
    config: ConstructionHeuristicConfig,
) -> MultiOwnerSolution {
    let mut phase = Construction::new(Some(config), multi_owner_descriptor(), multi_owner_model());
    let mut solver_scope = multi_owner_scope(solution);
    phase.solve(&mut solver_scope);
    solver_scope.working_solution().clone()
}

#[test]
fn list_target_matches_entity_class_only() {
    let solution = solve_multi_owner_construction(config(
        ConstructionHeuristicType::FirstFit,
        Some("Route"),
        None,
    ));

    assert_eq!(solution.routes, vec![vec![11, 10]]);
    assert_eq!(solution.shifts, vec![Vec::<usize>::new()]);
}

#[test]
fn list_target_matches_variable_name_across_all_owners() {
    let solution = solve_multi_owner_construction(config(
        ConstructionHeuristicType::FirstFit,
        None,
        Some("tasks"),
    ));

    assert_eq!(solution.routes, vec![vec![11, 10]]);
    assert_eq!(solution.shifts, vec![vec![21, 20]]);
}

#[test]
fn construction_target_panics_when_no_variable_matches() {
    let panic = std::panic::catch_unwind(|| {
        let _ = solve_multi_owner_construction(config(
            ConstructionHeuristicType::FirstFit,
            Some("Worker"),
            Some("tasks"),
        ));
    })
    .expect_err("missing generic target should panic");

    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&'static str>().copied())
        .unwrap_or("");
    assert!(message.contains("matched no planning variables"));
}

#[test]
fn untargeted_multi_owner_list_round_robin_runs_all_owners_in_declaration_order() {
    let solution = solve_multi_owner_construction(config(
        ConstructionHeuristicType::ListRoundRobin,
        None,
        None,
    ));

    assert_eq!(solution.routes, vec![vec![10, 11]]);
    assert_eq!(solution.shifts, vec![vec![20, 21]]);
    assert_eq!(solution.log, vec!["Route", "Route", "Shift", "Shift"]);
}

#[test]
fn targeted_multi_owner_list_round_robin_runs_only_matching_owner() {
    let solution = solve_multi_owner_construction(config(
        ConstructionHeuristicType::ListRoundRobin,
        Some("Shift"),
        None,
    ));

    assert_eq!(solution.routes, vec![Vec::<usize>::new()]);
    assert_eq!(solution.shifts, vec![vec![20, 21]]);
    assert_eq!(solution.log, vec!["Shift", "Shift"]);
}

#[test]
fn targeted_multi_owner_list_round_robin_runs_all_matching_owners() {
    let solution = solve_multi_owner_construction(config(
        ConstructionHeuristicType::ListRoundRobin,
        None,
        Some("tasks"),
    ));

    assert_eq!(solution.routes, vec![vec![10, 11]]);
    assert_eq!(solution.shifts, vec![vec![20, 21]]);
    assert_eq!(solution.log, vec!["Route", "Route", "Shift", "Shift"]);
}

#[test]
fn targeted_multi_owner_list_round_robin_panics_when_no_owner_matches() {
    let panic = std::panic::catch_unwind(|| {
        let _ = solve_multi_owner_construction(config(
            ConstructionHeuristicType::ListRoundRobin,
            Some("Worker"),
            Some("tasks"),
        ));
    })
    .expect_err("missing list target should panic");

    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&'static str>().copied())
        .unwrap_or("");
    assert!(message.contains("does not match the targeted planning list variable"));
}

#[test]
fn list_round_robin_runtime_appends_after_existing_elements() {
    let mut solution = multi_owner_solution();
    solution.routes[0] = vec![99];
    solution.shifts[0] = vec![88];

    let solution = solve_multi_owner_construction_with_solution(
        solution,
        config(ConstructionHeuristicType::ListRoundRobin, None, None),
    );

    assert_eq!(solution.routes[0], vec![99, 10, 11]);
    assert_eq!(solution.shifts[0], vec![88, 20, 21]);
    assert_eq!(solution.log, vec!["Route", "Route", "Shift", "Shift"]);
}

#[derive(Clone, Debug)]
struct MixedTargetWorker;

#[derive(Clone, Debug)]
struct MixedTargetRoute {
    worker_idx: Option<usize>,
    tasks: Vec<usize>,
}

#[derive(Clone, Debug)]
struct MixedTargetPlan {
    score: Option<SoftScore>,
    workers: Vec<MixedTargetWorker>,
    routes: Vec<MixedTargetRoute>,
    task_pool: Vec<usize>,
}

impl PlanningSolution for MixedTargetPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct MixedTargetDirector {
    working_solution: MixedTargetPlan,
    descriptor: SolutionDescriptor,
    score_mode: MixedTargetScoreMode,
}

#[derive(Clone, Copy, Debug)]
enum MixedTargetScoreMode {
    Flat,
    PreferAssignedWorker,
}

impl Director<MixedTargetPlan> for MixedTargetDirector {
    fn working_solution(&self) -> &MixedTargetPlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut MixedTargetPlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = match self.score_mode {
            MixedTargetScoreMode::Flat => SoftScore::of(0),
            MixedTargetScoreMode::PreferAssignedWorker => SoftScore::of(
                self.working_solution.routes[0]
                    .worker_idx
                    .map(|_| 1)
                    .unwrap_or(0),
            ),
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> MixedTargetPlan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.routes.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.routes.len())
    }
}

fn mixed_target_plan() -> MixedTargetPlan {
    MixedTargetPlan {
        score: None,
        workers: vec![MixedTargetWorker],
        routes: vec![MixedTargetRoute {
            worker_idx: None,
            tasks: Vec::new(),
        }],
        task_pool: vec![10],
    }
}

fn mixed_target_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("MixedTargetPlan", TypeId::of::<MixedTargetPlan>())
        .with_entity(
            EntityDescriptor::new("Route", TypeId::of::<MixedTargetRoute>(), "routes")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Route",
                    "routes",
                    |solution: &MixedTargetPlan| &solution.routes,
                    |solution: &mut MixedTargetPlan| &mut solution.routes,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_value_range("workers")
                        .with_usize_accessors(
                            mixed_target_worker_get_any,
                            mixed_target_worker_set_any,
                        ),
                )
                .with_variable(VariableDescriptor::list("tasks")),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<MixedTargetWorker>(), "workers")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |solution: &MixedTargetPlan| &solution.workers,
                    |solution: &mut MixedTargetPlan| &mut solution.workers,
                ))),
        )
}

fn mixed_target_route_count(solution: &MixedTargetPlan) -> usize {
    solution.routes.len()
}

fn mixed_target_worker_get(solution: &MixedTargetPlan, entity_index: usize) -> Option<usize> {
    solution.routes[entity_index].worker_idx
}

fn mixed_target_worker_set(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    value: Option<usize>,
) {
    solution.routes[entity_index].worker_idx = value;
}

fn mixed_target_worker_get_any(entity: &dyn std::any::Any) -> Option<usize> {
    entity
        .downcast_ref::<MixedTargetRoute>()
        .expect("route expected")
        .worker_idx
}

fn mixed_target_worker_set_any(entity: &mut dyn std::any::Any, value: Option<usize>) {
    entity
        .downcast_mut::<MixedTargetRoute>()
        .expect("route expected")
        .worker_idx = value;
}

fn mixed_target_element_count(solution: &MixedTargetPlan) -> usize {
    solution.task_pool.len()
}

fn mixed_target_assigned_elements(solution: &MixedTargetPlan) -> Vec<usize> {
    solution
        .routes
        .iter()
        .flat_map(|route| route.tasks.iter().copied())
        .collect()
}

fn mixed_target_list_len(solution: &MixedTargetPlan, entity_index: usize) -> usize {
    solution.routes[entity_index].tasks.len()
}

fn mixed_target_list_remove(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
) -> Option<usize> {
    let route = solution.routes.get_mut(entity_index)?;
    (pos < route.tasks.len()).then(|| route.tasks.remove(pos))
}

fn mixed_target_construction_list_remove(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].tasks.remove(pos)
}

fn mixed_target_list_insert(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].tasks.insert(pos, value);
}

fn mixed_target_list_get(
    solution: &MixedTargetPlan,
    entity_index: usize,
    pos: usize,
) -> Option<usize> {
    solution.routes[entity_index].tasks.get(pos).copied()
}

fn mixed_target_list_set(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].tasks[pos] = value;
}

fn mixed_target_list_reverse(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) {
    solution.routes[entity_index].tasks[start..end].reverse();
}

fn mixed_target_sublist_remove(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) -> Vec<usize> {
    solution.routes[entity_index]
        .tasks
        .drain(start..end)
        .collect()
}

fn mixed_target_sublist_insert(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
    values: Vec<usize>,
) {
    solution.routes[entity_index].tasks.splice(pos..pos, values);
}

fn mixed_target_ruin_remove(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].tasks.remove(pos)
}

fn mixed_target_ruin_insert(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].tasks.insert(pos, value);
}

fn mixed_target_index_to_element(solution: &MixedTargetPlan, idx: usize) -> usize {
    solution.task_pool[idx]
}

static MIXED_TARGET_AVAILABLE_WORKERS: [usize; 1] = [0];

fn mixed_target_available_workers(solution: &MixedTargetPlan, entity_index: usize) -> &[usize] {
    if solution.routes[entity_index].tasks.is_empty() {
        &MIXED_TARGET_AVAILABLE_WORKERS
    } else {
        &[]
    }
}

fn mixed_target_model() -> ModelContext<MixedTargetPlan, usize, DefaultMeter, DefaultMeter> {
    ModelContext::new(vec![
        VariableContext::Scalar(ScalarVariableContext::new(
            0,
            "Route",
            mixed_target_route_count,
            "worker_idx",
            mixed_target_worker_get,
            mixed_target_worker_set,
            ValueSource::EntitySlice {
                values_for_entity: mixed_target_available_workers,
            },
            true,
        )),
        VariableContext::List(ListVariableContext::new(
            "Route",
            mixed_target_element_count,
            mixed_target_assigned_elements,
            mixed_target_list_len,
            mixed_target_list_remove,
            mixed_target_construction_list_remove,
            mixed_target_list_insert,
            mixed_target_list_get,
            mixed_target_list_set,
            mixed_target_list_reverse,
            mixed_target_sublist_remove,
            mixed_target_sublist_insert,
            mixed_target_ruin_remove,
            mixed_target_ruin_insert,
            mixed_target_index_to_element,
            mixed_target_route_count,
            DefaultMeter::default(),
            DefaultMeter::default(),
            "tasks",
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )),
    ])
}

fn solve_mixed_target_construction(
    kind: ConstructionHeuristicType,
    entity_class: Option<&str>,
    score_mode: MixedTargetScoreMode,
) -> MixedTargetPlan {
    let descriptor = mixed_target_descriptor();
    let director = MixedTargetDirector {
        working_solution: mixed_target_plan(),
        descriptor: descriptor.clone(),
        score_mode,
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(config(kind, entity_class, None)),
        descriptor,
        mixed_target_model(),
    );
    phase.solve(&mut solver_scope);
    solver_scope.working_solution().clone()
}

#[test]
fn entity_class_target_matches_scalar_and_list_in_same_owner() {
    let solution = solve_mixed_target_construction(
        ConstructionHeuristicType::FirstFit,
        Some("Route"),
        MixedTargetScoreMode::PreferAssignedWorker,
    );

    assert_eq!(solution.routes[0].worker_idx, Some(0));
    assert_eq!(solution.routes[0].tasks, vec![10]);
}

#[test]
fn mixed_cheapest_insertion_breaks_equal_scores_by_canonical_order() {
    let solution = solve_mixed_target_construction(
        ConstructionHeuristicType::CheapestInsertion,
        Some("Route"),
        MixedTargetScoreMode::Flat,
    );

    assert_eq!(solution.routes[0].worker_idx, Some(0));
    assert_eq!(solution.routes[0].tasks, vec![10]);
}
