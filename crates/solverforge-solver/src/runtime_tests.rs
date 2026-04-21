use super::{
    list_target_matches, matching_list_construction, normalize_list_construction_config,
    Construction, ConstructionArgs,
};
use crate::descriptor_standard::standard_target_matches;
use crate::phase::Phase;
use crate::scope::SolverScope;
use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, ProblemFactDescriptor,
    SolutionDescriptor, VariableDescriptor, VariableType,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct TestSolution {
    score: Option<solverforge_core::score::SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = solverforge_core::score::SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn standard_variable(name: &'static str) -> VariableDescriptor {
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
    }
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(
            EntityDescriptor::new("Route", TypeId::of::<()>(), "routes")
                .with_variable(standard_variable("vehicle_id"))
                .with_variable(VariableDescriptor::list("visits")),
        )
        .with_entity(
            EntityDescriptor::new("Shift", TypeId::of::<u8>(), "shifts")
                .with_variable(standard_variable("employee_id")),
        )
}

fn list_args() -> ConstructionArgs<TestSolution, usize> {
    ConstructionArgs {
        element_count: |_| 0,
        assigned_elements: |_| Vec::new(),
        entity_count: |_| 0,
        list_len: |_, _| 0,
        list_insert: |_, _, _, _| {},
        list_remove: |_, _, _| 0,
        index_to_element: |_, _| 0,
        descriptor_index: 0,
        entity_type_name: "Route",
        variable_name: "visits",
        depot_fn: None,
        distance_fn: None,
        element_load_fn: None,
        capacity_fn: None,
        assign_route_fn: None,
        merge_feasible_fn: None,
        k_opt_get_route: None,
        k_opt_set_route: None,
        k_opt_depot_fn: None,
        k_opt_distance_fn: None,
        k_opt_feasible_fn: None,
    }
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
fn list_target_requires_matching_variable_name() {
    let cfg = config(
        ConstructionHeuristicType::ListCheapestInsertion,
        Some("Shift"),
        Some("employee_id"),
    );
    assert!(!list_target_matches(&cfg, &list_args()));
}

#[test]
fn list_target_matches_entity_class_only_for_owner() {
    let cfg = config(
        ConstructionHeuristicType::ListCheapestInsertion,
        Some("Route"),
        None,
    );
    assert!(list_target_matches(&cfg, &list_args()));
}

#[test]
fn matching_list_construction_returns_all_owners_without_target() {
    let args = vec![
        list_args(),
        ConstructionArgs {
            entity_type_name: "Shift",
            variable_name: "visits",
            ..list_args()
        },
    ];

    let matches = matching_list_construction::<TestSolution, usize>(None, &args);

    assert_eq!(matches.len(), 2);
}

#[test]
fn matching_list_construction_filters_to_targeted_owner() {
    let args = vec![
        list_args(),
        ConstructionArgs {
            entity_type_name: "Shift",
            variable_name: "assignments",
            ..list_args()
        },
    ];
    let cfg = config(
        ConstructionHeuristicType::ListCheapestInsertion,
        Some("Shift"),
        Some("assignments"),
    );

    let matches = matching_list_construction(Some(&cfg), &args);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].entity_type_name, "Shift");
    assert_eq!(matches[0].variable_name, "assignments");
}

#[test]
fn generic_list_dispatch_normalizes_to_list_cheapest_insertion() {
    let cfg = config(
        ConstructionHeuristicType::FirstFit,
        Some("Route"),
        Some("visits"),
    );
    let normalized = normalize_list_construction_config(Some(&cfg))
        .expect("generic list config should normalize");
    assert_eq!(
        normalized.construction_heuristic_type,
        ConstructionHeuristicType::ListCheapestInsertion
    );
}

#[test]
fn standard_target_matches_entity_class_only_target() {
    let descriptor = descriptor();
    assert!(standard_target_matches(&descriptor, Some("Route"), None));
}

#[derive(Clone, Debug)]
struct StandardRuntimeWorker;

#[derive(Clone, Debug)]
struct StandardRuntimeTask {
    worker_idx: Option<usize>,
}

#[derive(Clone, Debug)]
struct StandardRuntimePlan {
    score: Option<SoftScore>,
    workers: Vec<StandardRuntimeWorker>,
    tasks: Vec<StandardRuntimeTask>,
}

#[derive(Clone, Debug)]
struct StandardRuntimeDirector {
    working_solution: StandardRuntimePlan,
    descriptor: SolutionDescriptor,
}

impl PlanningSolution for StandardRuntimePlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl solverforge_scoring::Director<StandardRuntimePlan> for StandardRuntimeDirector {
    fn working_solution(&self) -> &StandardRuntimePlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut StandardRuntimePlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = if self.working_solution.tasks[0].worker_idx.is_none() {
            SoftScore::of(0)
        } else {
            SoftScore::of(-1)
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> StandardRuntimePlan {
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
        .downcast_ref::<StandardRuntimeTask>()
        .expect("task expected")
        .worker_idx
}

fn set_runtime_worker_idx(entity: &mut dyn std::any::Any, value: Option<usize>) {
    entity
        .downcast_mut::<StandardRuntimeTask>()
        .expect("task expected")
        .worker_idx = value;
}

fn standard_runtime_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("StandardRuntimePlan", TypeId::of::<StandardRuntimePlan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<StandardRuntimeTask>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |solution: &StandardRuntimePlan| &solution.tasks,
                    |solution: &mut StandardRuntimePlan| &mut solution.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_value_range("workers")
                        .with_usize_accessors(get_runtime_worker_idx, set_runtime_worker_idx),
                ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<StandardRuntimeWorker>(), "workers")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |solution: &StandardRuntimePlan| &solution.workers,
                    |solution: &mut StandardRuntimePlan| &mut solution.workers,
                ))),
        )
}

fn standard_runtime_list_args() -> ConstructionArgs<StandardRuntimePlan, usize> {
    ConstructionArgs {
        element_count: |_| 0,
        assigned_elements: |_| Vec::new(),
        entity_count: |_| 0,
        list_len: |_, _| 0,
        list_insert: |_, _, _, _| {},
        list_remove: |_, _, _| 0,
        index_to_element: |_, _| 0,
        descriptor_index: 0,
        entity_type_name: "Route",
        variable_name: "visits",
        depot_fn: None,
        distance_fn: None,
        element_load_fn: None,
        capacity_fn: None,
        assign_route_fn: None,
        merge_feasible_fn: None,
        k_opt_get_route: None,
        k_opt_set_route: None,
        k_opt_depot_fn: None,
        k_opt_distance_fn: None,
        k_opt_feasible_fn: None,
    }
}

#[test]
fn standard_runtime_frontier_marks_kept_optional_none_as_complete() {
    let descriptor = standard_runtime_descriptor();
    let plan = StandardRuntimePlan {
        score: None,
        workers: vec![StandardRuntimeWorker],
        tasks: vec![StandardRuntimeTask { worker_idx: None }],
    };
    let director = StandardRuntimeDirector {
        working_solution: plan,
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut targeted_phase = Construction::new(
        Some(config(
            ConstructionHeuristicType::CheapestInsertion,
            Some("Task"),
            Some("worker_idx"),
        )),
        descriptor.clone(),
        vec![standard_runtime_list_args()],
    );
    targeted_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(solver_scope.stats().step_count, 1);
    assert!(
        !super::standard_work_remaining_with_frontier(
            &descriptor,
            solver_scope.standard_construction_frontier(),
            solver_scope.solution_revision(),
            None,
            None,
            solver_scope.working_solution(),
        ),
        "completed optional None should not be treated as remaining standard work",
    );

    let mut untargeted_phase =
        Construction::new(None, descriptor, vec![standard_runtime_list_args()]);
    untargeted_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(solver_scope.stats().step_count, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 0);
}

#[test]
fn no_op_runtime_construction_still_seeds_score_and_best_solution() {
    let descriptor = standard_runtime_descriptor();
    let plan = StandardRuntimePlan {
        score: None,
        workers: vec![StandardRuntimeWorker],
        tasks: vec![StandardRuntimeTask {
            worker_idx: Some(0),
        }],
    };
    let director = StandardRuntimeDirector {
        working_solution: plan,
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(None, descriptor, vec![standard_runtime_list_args()]);
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(-1))
    );
    assert_eq!(solver_scope.best_score().copied(), Some(SoftScore::of(-1)));
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

impl solverforge_scoring::Director<RevisionPlan> for RevisionDirector {
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
            _ => None,
        }
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.tasks.len())
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

fn revision_list_args() -> ConstructionArgs<RevisionPlan, usize> {
    ConstructionArgs {
        element_count: |solution| solution.route_pool.len(),
        assigned_elements: |solution| {
            solution
                .routes
                .iter()
                .flat_map(|route| route.iter().copied())
                .collect()
        },
        entity_count: |solution| solution.routes.len(),
        list_len: |solution, entity_idx| solution.routes[entity_idx].len(),
        list_insert: |solution, entity_idx, pos, value| {
            solution.routes[entity_idx].insert(pos, value);
        },
        list_remove: |solution, entity_idx, pos| solution.routes[entity_idx].remove(pos),
        index_to_element: |solution, idx| solution.route_pool[idx],
        descriptor_index: 1,
        entity_type_name: "Route",
        variable_name: "visits",
        depot_fn: None,
        distance_fn: None,
        element_load_fn: None,
        capacity_fn: None,
        assign_route_fn: None,
        merge_feasible_fn: None,
        k_opt_get_route: None,
        k_opt_set_route: None,
        k_opt_depot_fn: None,
        k_opt_distance_fn: None,
        k_opt_feasible_fn: None,
    }
}

#[test]
fn later_construction_revisits_optional_none_after_unrelated_list_commit() {
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

    let mut standard_phase = Construction::new(
        Some(config(
            ConstructionHeuristicType::CheapestInsertion,
            Some("Task"),
            Some("worker_idx"),
        )),
        descriptor.clone(),
        vec![revision_list_args()],
    );
    standard_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(
        solver_scope.working_solution().routes[0],
        Vec::<usize>::new()
    );

    let mut list_phase = Construction::new(
        Some(config(
            ConstructionHeuristicType::ListRoundRobin,
            Some("Route"),
            Some("visits"),
        )),
        descriptor.clone(),
        vec![revision_list_args()],
    );
    list_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().routes[0], vec![10]);

    let mut reconstruct = Construction::new(None, descriptor, vec![revision_list_args()]);
    reconstruct.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
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
}

fn route_args() -> ConstructionArgs<MultiOwnerSolution, usize> {
    ConstructionArgs {
        element_count: |solution| solution.route_pool.len(),
        assigned_elements: |solution| {
            solution
                .routes
                .iter()
                .flat_map(|route| route.iter().copied())
                .collect()
        },
        entity_count: |solution| solution.routes.len(),
        list_len: |solution, entity_idx| solution.routes[entity_idx].len(),
        list_insert: |solution, entity_idx, pos, value| {
            solution.log.push("Route");
            solution.routes[entity_idx].insert(pos, value);
        },
        list_remove: |solution, entity_idx, pos| solution.routes[entity_idx].remove(pos),
        index_to_element: |solution, idx| solution.route_pool[idx],
        descriptor_index: 0,
        entity_type_name: "Route",
        variable_name: "tasks",
        depot_fn: None,
        distance_fn: None,
        element_load_fn: None,
        capacity_fn: None,
        assign_route_fn: None,
        merge_feasible_fn: None,
        k_opt_get_route: None,
        k_opt_set_route: None,
        k_opt_depot_fn: None,
        k_opt_distance_fn: None,
        k_opt_feasible_fn: None,
    }
}

fn shift_args() -> ConstructionArgs<MultiOwnerSolution, usize> {
    ConstructionArgs {
        element_count: |solution| solution.shift_pool.len(),
        assigned_elements: |solution| {
            solution
                .shifts
                .iter()
                .flat_map(|shift| shift.iter().copied())
                .collect()
        },
        entity_count: |solution| solution.shifts.len(),
        list_len: |solution, entity_idx| solution.shifts[entity_idx].len(),
        list_insert: |solution, entity_idx, pos, value| {
            solution.log.push("Shift");
            solution.shifts[entity_idx].insert(pos, value);
        },
        list_remove: |solution, entity_idx, pos| solution.shifts[entity_idx].remove(pos),
        index_to_element: |solution, idx| solution.shift_pool[idx],
        descriptor_index: 1,
        entity_type_name: "Shift",
        variable_name: "tasks",
        depot_fn: None,
        distance_fn: None,
        element_load_fn: None,
        capacity_fn: None,
        assign_route_fn: None,
        merge_feasible_fn: None,
        k_opt_get_route: None,
        k_opt_set_route: None,
        k_opt_depot_fn: None,
        k_opt_distance_fn: None,
        k_opt_feasible_fn: None,
    }
}

fn multi_owner_scope(
    solution: MultiOwnerSolution,
) -> SolverScope<'static, MultiOwnerSolution, ScoreDirector<MultiOwnerSolution, ()>> {
    let director = ScoreDirector::simple(solution, multi_owner_descriptor(), |_, _| 0);
    SolverScope::new(director)
}

fn solve_multi_owner_construction(config: ConstructionHeuristicConfig) -> MultiOwnerSolution {
    let mut phase = Construction::new(
        Some(config),
        multi_owner_descriptor(),
        vec![route_args(), shift_args()],
    );
    let mut solver_scope = multi_owner_scope(multi_owner_solution());
    phase.solve(&mut solver_scope);
    solver_scope.working_solution().clone()
}

#[test]
fn untargeted_multi_owner_list_construction_runs_all_owners_in_declaration_order() {
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
fn targeted_multi_owner_list_construction_runs_only_matching_owner() {
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
fn targeted_multi_owner_list_construction_runs_all_matching_owners() {
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
fn targeted_multi_owner_list_construction_panics_when_no_owner_matches() {
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
    assert!(message.contains("matched no planning variables"));
}
