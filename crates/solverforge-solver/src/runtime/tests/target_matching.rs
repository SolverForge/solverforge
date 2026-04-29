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
        candidate_values: None,
        nearby_value_candidates: None,
        nearby_entity_candidates: None,
        nearby_value_distance_meter: None,
        nearby_entity_distance_meter: None,
        construction_entity_order_key: None,
        construction_value_order_key: None,
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
        value_candidate_limit: Some(usize::MAX),
        construction_heuristic_type,
        construction_obligation: Default::default(),
        target: VariableTargetConfig {
            entity_class: entity_class.map(str::to_owned),
            variable_name: variable_name.map(str::to_owned),
        },
        k: 2,
        group_name: None,
        group_candidate_limit: None,
        termination: None,
    }
}

#[test]
fn scalar_target_matches_entity_class_only_target() {
    let descriptor = descriptor();
    assert!(scalar_target_matches(&descriptor, Some("Route"), None));
}
