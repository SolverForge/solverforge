use super::*;

#[derive(Debug)]
struct DynamicAccess {
    profile: DynamicListProfile,
}

impl DynamicListAccess<Plan> for DynamicAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, solution: &Plan) -> usize {
        entity_count(solution)
    }

    fn element_count(&self, solution: &Plan) -> usize {
        element_count(solution)
    }

    fn element(&self, solution: &Plan, element_index: usize) -> Option<usize> {
        solution.elements.get(element_index).copied()
    }

    fn assigned_elements(&self, solution: &Plan) -> Vec<usize> {
        assigned_elements(solution)
    }

    fn len(&self, solution: &Plan, entity: usize) -> usize {
        list_len(solution, entity)
    }

    fn get(&self, solution: &Plan, entity: usize, position: usize) -> Option<usize> {
        list_get(solution, entity, position)
    }

    fn insert(&self, solution: &mut Plan, entity: usize, position: usize, value: usize) {
        list_insert(solution, entity, position, value);
    }

    fn remove(&self, solution: &mut Plan, entity: usize, position: usize) -> Option<usize> {
        list_remove(solution, entity, position)
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        match self.profile {
            DynamicListProfile::Reduced => DynamicListAccessCapabilities {
                replace: true,
                ..DynamicListAccessCapabilities::default()
            },
            DynamicListProfile::SetWithoutMetric => DynamicListAccessCapabilities {
                set: true,
                ..DynamicListAccessCapabilities::default()
            },
            DynamicListProfile::Equivalent => DynamicListAccessCapabilities {
                set: true,
                replace: true,
                reverse: true,
                sublist: true,
            },
        }
    }

    fn set(&self, solution: &mut Plan, entity: usize, position: usize, value: usize) -> bool {
        list_set(solution, entity, position, value);
        true
    }

    fn replace(&self, solution: &mut Plan, entity: usize, values: Vec<usize>) -> bool {
        replace_route(solution, entity, values);
        true
    }

    fn reverse(&self, solution: &mut Plan, entity: usize, start: usize, end: usize) -> bool {
        list_reverse(solution, entity, start, end);
        true
    }

    fn sublist_remove(
        &self,
        solution: &mut Plan,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Option<Vec<usize>> {
        Some(sublist_remove(solution, entity, start, end))
    }

    fn sublist_insert(
        &self,
        solution: &mut Plan,
        entity: usize,
        position: usize,
        values: Vec<usize>,
    ) -> bool {
        sublist_insert(solution, entity, position, values);
        true
    }
}

#[derive(Debug)]
struct DynamicMetadata {
    scenario: Scenario,
    profile: DynamicListProfile,
}

impl DynamicListMetadata<Plan> for DynamicMetadata {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn capabilities(&self) -> DynamicListMetadataCapabilities {
        match self.scenario {
            Scenario::Cheapest => DynamicListMetadataCapabilities::default(),
            Scenario::Jssp => DynamicListMetadataCapabilities {
                element_owner: true,
                construction_order_key: true,
                precedence_duration: true,
                precedence_successors: true,
                ..DynamicListMetadataCapabilities::default()
            },
            Scenario::Cvrp => DynamicListMetadataCapabilities {
                cross_position_distance: matches!(self.profile, DynamicListProfile::Equivalent),
                intra_position_distance: matches!(self.profile, DynamicListProfile::Equivalent),
                route: true,
                savings: true,
                ..DynamicListMetadataCapabilities::default()
            },
        }
    }

    fn element_owner(&self, solution: &Plan, element: usize) -> Option<usize> {
        owner(solution, &element)
    }

    fn construction_order_key(&self, solution: &Plan, element: usize) -> Option<i64> {
        matches!(self.scenario, Scenario::Jssp).then(|| order(solution, element))
    }

    fn precedence_duration(&self, solution: &Plan, element: usize) -> Option<usize> {
        matches!(self.scenario, Scenario::Jssp).then(|| duration(solution, element))
    }

    fn extend_precedence_successors(
        &self,
        solution: &Plan,
        element: usize,
        successors_out: &mut Vec<usize>,
    ) -> bool {
        if !matches!(self.scenario, Scenario::Jssp) {
            return false;
        }
        successors(solution, element, successors_out);
        true
    }

    fn cross_position_distance(
        &self,
        _: &Plan,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
    ) -> Option<f64> {
        matches!(self.profile, DynamicListProfile::Equivalent).then_some(0.0)
    }

    fn intra_position_distance(&self, _: &Plan, _: usize, _: usize, _: usize) -> Option<f64> {
        matches!(self.profile, DynamicListProfile::Equivalent).then_some(0.0)
    }

    fn route_depot(&self, solution: &Plan, entity: usize) -> Option<usize> {
        matches!(self.scenario, Scenario::Cvrp).then(|| depot(solution, entity))
    }

    fn route_distance(
        &self,
        solution: &Plan,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Option<i64> {
        matches!(self.scenario, Scenario::Cvrp).then(|| route_distance(solution, entity, from, to))
    }

    fn route_feasible(&self, solution: &Plan, entity: usize, route: &[usize]) -> Option<bool> {
        matches!(self.scenario, Scenario::Cvrp).then(|| feasible(solution, entity, route))
    }

    fn savings_depot(&self, solution: &Plan, entity: usize) -> Option<usize> {
        matches!(self.scenario, Scenario::Cvrp).then(|| depot(solution, entity))
    }

    fn savings_metric_class(&self, _: &Plan, entity: usize) -> Option<usize> {
        matches!(self.scenario, Scenario::Cvrp).then_some(entity)
    }

    fn savings_distance(
        &self,
        solution: &Plan,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Option<i64> {
        matches!(self.scenario, Scenario::Cvrp)
            .then(|| savings_distance(solution, entity, from, to))
    }

    fn savings_feasible(&self, solution: &Plan, entity: usize, route: &[usize]) -> Option<bool> {
        matches!(self.scenario, Scenario::Cvrp).then(|| feasible(solution, entity, route))
    }
}

pub(super) fn dynamic_model(scenario: Scenario) -> Model {
    dynamic_model_with_profile(scenario, DynamicListProfile::Reduced)
}

pub(super) fn dynamic_model_with_profile(scenario: Scenario, profile: DynamicListProfile) -> Model {
    let slot = DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(0),
        VariableId(0),
        "Vehicle",
        "visits",
        Arc::new(DynamicAccess { profile }),
        Arc::new(DynamicMetadata { scenario, profile }),
    )
    .expect("test access and metadata identities match");
    RuntimeModel::new(vec![VariableSlot::DynamicList(slot)])
}
