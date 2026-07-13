#[cfg(test)]
mod tests {
    use std::sync::Arc;


    use solverforge_core::domain::{
        DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
        DynamicListMetadataCapabilities, DynamicListVariableSlot, EntityClassId,
        PlanningSolution, VariableId,
    };
    use solverforge_core::score::SoftScore;

    use super::{ListAccessCapability, RouteAccess, RouteSequenceAccess, SavingsAccess};

    #[derive(Clone, Debug)]
    struct Plan {
        rows: Vec<Vec<usize>>,
        score: Option<SoftScore>,
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
            solution.rows.len()
        }

        fn element_count(&self, solution: &Plan) -> usize {
            solution
                .rows
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
            solution.rows.iter().flatten().copied().collect()
        }

        fn len(&self, solution: &Plan, row: usize) -> usize {
            solution.rows[row].len()
        }

        fn get(&self, solution: &Plan, row: usize, position: usize) -> Option<usize> {
            solution.rows.get(row)?.get(position).copied()
        }

        fn insert(&self, solution: &mut Plan, row: usize, position: usize, value: usize) {
            solution.rows[row].insert(position, value);
        }

        fn remove(&self, solution: &mut Plan, row: usize, position: usize) -> Option<usize> {
            (position < solution.rows[row].len()).then(|| solution.rows[row].remove(position))
        }

        fn capabilities(&self) -> DynamicListAccessCapabilities {
            DynamicListAccessCapabilities {
                replace: true,
                ..DynamicListAccessCapabilities::default()
            }
        }

        fn replace(&self, solution: &mut Plan, row: usize, route: Vec<usize>) -> bool {
            let Some(target) = solution.rows.get_mut(row) else {
                return false;
            };
            *target = route;
            true
        }
    }

    #[derive(Debug)]
    struct Metadata {
        capabilities: DynamicListMetadataCapabilities,
    }

    impl DynamicListMetadata<Plan> for Metadata {
        fn entity_class(&self) -> EntityClassId {
            EntityClassId(0)
        }

        fn variable(&self) -> VariableId {
            VariableId(0)
        }

        fn capabilities(&self) -> DynamicListMetadataCapabilities {
            self.capabilities
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
            Some(0)
        }

        fn route_distance(
            &self,
            _solution: &Plan,
            _entity: usize,
            from: usize,
            to: usize,
        ) -> Option<i64> {
            Some(from.abs_diff(to) as i64)
        }

        fn route_feasible(
            &self,
            _solution: &Plan,
            _entity: usize,
            _route: &[usize],
        ) -> Option<bool> {
            Some(true)
        }

        fn savings_depot(&self, _solution: &Plan, _entity: usize) -> Option<usize> {
            Some(0)
        }

        fn savings_metric_class(&self, _solution: &Plan, _entity: usize) -> Option<usize> {
            Some(0)
        }

        fn savings_distance(
            &self,
            _solution: &Plan,
            _entity: usize,
            from: usize,
            to: usize,
        ) -> Option<i64> {
            Some(from.abs_diff(to) as i64)
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

    fn slot(capabilities: DynamicListMetadataCapabilities) -> DynamicListVariableSlot<Plan> {
        DynamicListVariableSlot::with_access_and_metadata(
            EntityClassId(0),
            VariableId(0),
            "Vehicle",
            "visits",
            Arc::new(Access),
            Arc::new(Metadata { capabilities }),
        )
        .expect("test access and metadata identities match")
    }

    #[test]
    fn route_and_savings_are_independent_strict_bundles() {
        let route_only = slot(DynamicListMetadataCapabilities {
            route: true,
            ..DynamicListMetadataCapabilities::default()
        });
        assert!(RouteAccess::validate_route(&route_only).is_ok());
        assert_eq!(
            SavingsAccess::validate_savings(&route_only)
                .expect_err("route metadata must not satisfy savings")
                .capability,
            ListAccessCapability::Savings
        );

        let savings_only = slot(DynamicListMetadataCapabilities {
            savings: true,
            ..DynamicListMetadataCapabilities::default()
        });
        assert!(SavingsAccess::validate_savings(&savings_only).is_ok());
        assert_eq!(
            RouteAccess::validate_route(&savings_only)
                .expect_err("savings metadata must not satisfy route")
                .capability,
            ListAccessCapability::Route
        );
    }

    #[test]
    fn dynamic_route_sequence_reads_base_access_and_replaces_once() {
        let slot = slot(DynamicListMetadataCapabilities {
            route: true,
            ..DynamicListMetadataCapabilities::default()
        });
        let mut plan = Plan {
            rows: vec![vec![8, 3, 5]],
            score: None,
        };

        assert_eq!(
            RouteSequenceAccess::route_values(&slot, &plan, 0),
            Ok(vec![8, 3, 5])
        );
        RouteSequenceAccess::replace_route(&slot, &mut plan, 0, vec![2, 9])
            .expect("dynamic route replacement uses the direct replace operation");
        assert_eq!(plan.rows, vec![vec![2, 9]]);
    }
}
