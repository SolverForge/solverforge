use super::Construction;
use crate::builder::{ListVariableContext, ModelContext, VariableContext};
use crate::phase::Phase;
use crate::scope::SolverScope;
use crate::DefaultCrossEntityDistanceMeter;
use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;
use std::any::TypeId;

type DefaultMeter = DefaultCrossEntityDistanceMeter;

#[derive(Clone, Debug)]
struct GenericListPlan {
    score: Option<SoftScore>,
    routes: Vec<Vec<usize>>,
    route_pool: Vec<usize>,
}

impl PlanningSolution for GenericListPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct GenericListDirector {
    working_solution: GenericListPlan,
    descriptor: SolutionDescriptor,
}

impl Director<GenericListPlan> for GenericListDirector {
    fn working_solution(&self) -> &GenericListPlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut GenericListPlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = match self.working_solution.routes.as_slice() {
            [left, right] if left.is_empty() && right.is_empty() => SoftScore::of(0),
            [left, right] if !left.is_empty() && right.is_empty() => SoftScore::of(-1),
            [left, right] if left.is_empty() && !right.is_empty() => SoftScore::of(5),
            _ => SoftScore::of(0),
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> GenericListPlan {
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

fn config(kind: ConstructionHeuristicType) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        value_candidate_limit: None,
        construction_heuristic_type: kind,
        construction_obligation: Default::default(),
        target: VariableTargetConfig::default(),
        k: 2,
        termination: None,
    }
}

fn generic_list_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("GenericListPlan", TypeId::of::<GenericListPlan>()).with_entity(
        EntityDescriptor::new("Route", TypeId::of::<Vec<usize>>(), "routes").with_extractor(
            Box::new(EntityCollectionExtractor::new(
                "Route",
                "routes",
                |solution: &GenericListPlan| &solution.routes,
                |solution: &mut GenericListPlan| &mut solution.routes,
            )),
        ),
    )
}

fn route_count(solution: &GenericListPlan) -> usize {
    solution.routes.len()
}

fn route_element_count(solution: &GenericListPlan) -> usize {
    solution.route_pool.len()
}

fn assigned_route_elements(solution: &GenericListPlan) -> Vec<usize> {
    solution
        .routes
        .iter()
        .flat_map(|route| route.iter().copied())
        .collect()
}

fn route_len(solution: &GenericListPlan, entity_index: usize) -> usize {
    solution.routes[entity_index].len()
}

fn route_remove(solution: &mut GenericListPlan, entity_index: usize, pos: usize) -> Option<usize> {
    let route = solution.routes.get_mut(entity_index)?;
    (pos < route.len()).then(|| route.remove(pos))
}

fn route_remove_for_construction(
    solution: &mut GenericListPlan,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].remove(pos)
}

fn route_insert(solution: &mut GenericListPlan, entity_index: usize, pos: usize, value: usize) {
    solution.routes[entity_index].insert(pos, value);
}

fn route_get(solution: &GenericListPlan, entity_index: usize, pos: usize) -> Option<usize> {
    solution.routes[entity_index].get(pos).copied()
}

fn route_set(solution: &mut GenericListPlan, entity_index: usize, pos: usize, value: usize) {
    solution.routes[entity_index][pos] = value;
}

fn route_reverse(solution: &mut GenericListPlan, entity_index: usize, start: usize, end: usize) {
    solution.routes[entity_index][start..end].reverse();
}

fn route_sublist_remove(
    solution: &mut GenericListPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) -> Vec<usize> {
    solution.routes[entity_index].drain(start..end).collect()
}

fn route_sublist_insert(
    solution: &mut GenericListPlan,
    entity_index: usize,
    pos: usize,
    values: Vec<usize>,
) {
    solution.routes[entity_index].splice(pos..pos, values);
}

fn route_ruin_remove(solution: &mut GenericListPlan, entity_index: usize, pos: usize) -> usize {
    solution.routes[entity_index].remove(pos)
}

fn route_ruin_insert(
    solution: &mut GenericListPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].insert(pos, value);
}

fn route_index_to_element(solution: &GenericListPlan, idx: usize) -> usize {
    solution.route_pool[idx]
}

fn generic_list_model() -> ModelContext<GenericListPlan, usize, DefaultMeter, DefaultMeter> {
    ModelContext::new(vec![VariableContext::List(ListVariableContext::new(
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
        route_count,
        DefaultMeter::default(),
        DefaultMeter::default(),
        "visits",
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
    ))])
}

fn solve_generic_list(kind: ConstructionHeuristicType) -> GenericListPlan {
    let descriptor = generic_list_descriptor();
    let plan = GenericListPlan {
        score: None,
        routes: vec![Vec::new(), Vec::new()],
        route_pool: vec![10],
    };
    let director = GenericListDirector {
        working_solution: plan,
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut phase = Construction::new(Some(config(kind)), descriptor, generic_list_model());
    phase.solve(&mut solver_scope);
    solver_scope.working_solution().clone()
}

#[test]
fn generic_list_only_first_fit_uses_canonical_order() {
    let solution = solve_generic_list(ConstructionHeuristicType::FirstFit);

    assert_eq!(solution.routes, vec![vec![10], Vec::<usize>::new()]);
}

#[test]
fn generic_list_only_cheapest_insertion_uses_global_best_score() {
    let solution = solve_generic_list(ConstructionHeuristicType::CheapestInsertion);

    assert_eq!(solution.routes, vec![Vec::<usize>::new(), vec![10]]);
}
