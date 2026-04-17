use super::*;
use solverforge_config::VariableTargetConfig;
use solverforge_core::score::SoftScore;

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

fn config(kind: ConstructionHeuristicType) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type: kind,
        target: VariableTargetConfig::default(),
        k: 2,
        termination: None,
    }
}

#[test]
fn list_builder_rejects_unnormalized_generic_construction() {
    let panic = std::panic::catch_unwind(|| {
        let args = ConstructionArgs {
            element_count: |_| 0,
            assigned_elements: |_| Vec::new(),
            entity_count: |_| 0,
            list_len: |_, _| 0,
            list_insert: |_, _, _, _| {},
            list_remove: |_, _, _| 0,
            index_to_element: |_, _| 0,
            descriptor_index: 0,
            entity_type_name: "TestEntity",
            variable_name: "list",
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
        };
        let _ = build_list_construction::<TestSolution, usize>(
            Some(&config(ConstructionHeuristicType::FirstFit)),
            &args,
        );
    })
    .expect_err("unnormalized generic construction should panic");

    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&'static str>().copied())
        .unwrap_or("");
    assert!(message.contains("must be normalized before list construction"));
}
