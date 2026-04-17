use super::{list_target_matches, normalize_list_construction_config, ConstructionArgs};
use crate::descriptor_standard::standard_target_matches;
use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, VariableDescriptor, VariableType,
};
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
    assert!(!list_target_matches(&cfg, Some(&list_args())));
}

#[test]
fn list_target_matches_entity_class_only_for_owner() {
    let cfg = config(
        ConstructionHeuristicType::ListCheapestInsertion,
        Some("Route"),
        None,
    );
    assert!(list_target_matches(&cfg, Some(&list_args())));
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
