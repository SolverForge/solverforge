#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use solverforge_core::domain::{
        DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
        DynamicListMetadataCapabilities, EntityClassId, PlanningSolution, VariableId,
    };
    use solverforge_core::score::SoftScore;

    use crate::builder::context::list_access::{RouteAccess, SavingsAccess};
    use crate::builder::context::{RuntimeListSlot, SavingsMetricClassPolicy};
    use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;

    #[derive(Clone, Debug)]
    struct Plan {
        score: Option<SoftScore>,
        routes: Vec<Vec<usize>>,
    }

    impl PlanningSolution for Plan {
        type Score = SoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[derive(Debug)]
    struct Access;

    impl DynamicListAccess<Plan> for Access {
        fn entity_class(&self) -> EntityClassId {
            EntityClassId(0)
        }

        fn variable(&self) -> VariableId {
            VariableId(0)
        }

        fn entity_count(&self, solution: &Plan) -> usize {
            solution.routes.len()
        }

        fn element_count(&self, solution: &Plan) -> usize {
            solution
                .routes
                .iter()
                .flatten()
                .copied()
                .max()
                .map_or(0, |value| value + 1)
        }

        fn element(&self, _solution: &Plan, element_index: usize) -> Option<usize> {
            Some(element_index)
        }

        fn assigned_elements(&self, solution: &Plan) -> Vec<usize> {
            solution.routes.iter().flatten().copied().collect()
        }

        fn len(&self, solution: &Plan, row: usize) -> usize {
            solution.routes[row].len()
        }

        fn get(&self, solution: &Plan, row: usize, position: usize) -> Option<usize> {
            solution.routes.get(row)?.get(position).copied()
        }

        fn insert(&self, solution: &mut Plan, row: usize, position: usize, value: usize) {
            solution.routes[row].insert(position, value);
        }

        fn remove(&self, solution: &mut Plan, row: usize, position: usize) -> Option<usize> {
            (position < solution.routes[row].len()).then(|| solution.routes[row].remove(position))
        }

        fn capabilities(&self) -> DynamicListAccessCapabilities {
            DynamicListAccessCapabilities {
                replace: true,
                ..DynamicListAccessCapabilities::default()
            }
        }

        fn replace(&self, solution: &mut Plan, row: usize, values: Vec<usize>) -> bool {
            let Some(route) = solution.routes.get_mut(row) else {
                return false;
            };
            *route = values;
            true
        }
    }

    #[derive(Debug)]
    struct Metadata;

    impl DynamicListMetadata<Plan> for Metadata {
        fn entity_class(&self) -> EntityClassId {
            EntityClassId(0)
        }

        fn variable(&self) -> VariableId {
            VariableId(0)
        }

        fn capabilities(&self) -> DynamicListMetadataCapabilities {
            DynamicListMetadataCapabilities {
                route: true,
                savings: true,
                ..DynamicListMetadataCapabilities::default()
            }
        }

        fn element_owner(&self, _solution: &Plan, _element: usize) -> Option<usize> {
            None
        }

        fn construction_order_key(&self, _solution: &Plan, _element: usize) -> Option<i64> {
            None
        }

        fn precedence_duration(&self, _solution: &Plan, _element: usize) -> Option<usize> {
            None
        }

        fn extend_precedence_successors(
            &self,
            _solution: &Plan,
            _element: usize,
            _successors: &mut Vec<usize>,
        ) -> bool {
            false
        }

        fn cross_position_distance(
            &self,
            _solution: &Plan,
            _from_entity: usize,
            _from_position: usize,
            _to_entity: usize,
            _to_position: usize,
        ) -> Option<f64> {
            None
        }

        fn intra_position_distance(
            &self,
            _solution: &Plan,
            _entity: usize,
            _from_position: usize,
            _to_position: usize,
        ) -> Option<f64> {
            None
        }

        fn route_depot(&self, _solution: &Plan, _entity: usize) -> Option<usize> {
            Some(7)
        }

        fn route_distance(
            &self,
            _solution: &Plan,
            _entity: usize,
            _from: usize,
            _to: usize,
        ) -> Option<i64> {
            Some(101)
        }

        fn route_feasible(
            &self,
            _solution: &Plan,
            _entity: usize,
            _route: &[usize],
        ) -> Option<bool> {
            Some(false)
        }

        fn savings_depot(&self, _solution: &Plan, _entity: usize) -> Option<usize> {
            Some(3)
        }

        fn savings_metric_class(&self, _solution: &Plan, _entity: usize) -> Option<usize> {
            Some(11)
        }

        fn savings_distance(
            &self,
            _solution: &Plan,
            _entity: usize,
            _from: usize,
            _to: usize,
        ) -> Option<i64> {
            Some(202)
        }

        fn savings_feasible(
            &self,
            _solution: &Plan,
            _entity: usize,
            _route: &[usize],
        ) -> Option<bool> {
            Some(true)
        }
    }

    fn slot() -> RuntimeListSlot<
        Plan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > {
        RuntimeListSlot::from_dynamic(
            solverforge_core::domain::DynamicListVariableSlot::with_access_and_metadata(
                EntityClassId(0),
                VariableId(0),
                "Vehicle",
                "visits",
                Arc::new(Access),
                Arc::new(Metadata),
            )
            .expect("access and metadata identities match"),
        )
    }

    #[test]
    fn route_and_savings_bundles_remain_independent_for_runtime_slots() {
        let slot = slot();
        let plan = Plan {
            score: None,
            routes: vec![vec![4, 9]],
        };

        assert_eq!(RouteAccess::route_depot(&slot, &plan, 0), Ok(7));
        assert_eq!(RouteAccess::route_distance(&slot, &plan, 0, 4, 9), Ok(101));
        assert_eq!(
            RouteAccess::route_feasible(&slot, &plan, 0, &[4, 9]),
            Ok(false)
        );

        assert_eq!(SavingsAccess::savings_depot(&slot, &plan, 0), Ok(3));
        assert_eq!(
            slot.savings_metric_class_policy(),
            SavingsMetricClassPolicy::ExplicitDynamicProvider
        );
        assert_eq!(SavingsAccess::savings_metric_class(&slot, &plan, 0), Ok(11));
        assert_eq!(
            SavingsAccess::savings_distance(&slot, &plan, 0, 4, 9),
            Ok(202)
        );
        assert_eq!(
            SavingsAccess::savings_feasible(&slot, &plan, 0, &[4, 9]),
            Ok(true)
        );
    }
}
