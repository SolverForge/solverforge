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
    EntityDescriptor, PlanningSolution, SolutionDescriptor, VariableDescriptor, VariableType,
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

fn multi_owner_scope(solution: MultiOwnerSolution) -> SolverScope<'static, MultiOwnerSolution, ScoreDirector<MultiOwnerSolution, ()>> {
    let director = ScoreDirector::simple(solution, multi_owner_descriptor(), |_, _| 0);
    SolverScope::new(director)
}

fn solve_multi_owner_construction(
    config: ConstructionHeuristicConfig,
) -> MultiOwnerSolution {
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
