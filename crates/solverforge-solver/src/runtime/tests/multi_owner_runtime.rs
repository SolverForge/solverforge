
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

fn multi_owner_model() -> RuntimeModel<MultiOwnerSolution, usize, DefaultMeter, DefaultMeter> {
    RuntimeModel::new(vec![
        VariableSlot::List(ListVariableSlot::new(
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
        )),
        VariableSlot::List(ListVariableSlot::new(
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
    assert!(message.contains("matched no planning variables"));
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
