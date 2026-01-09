//! List construction phase for zero-erasure fluent API.

use solverforge_core::domain::{ListVariableSolution, PlanningSolution};
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::director::typed::TypedScoreDirector;
use solverforge_scoring::director::ShadowVariableSupport;

/// Assigns all unassigned elements to entities using round-robin distribution.
///
/// This is a simple construction heuristic that distributes elements evenly
/// across entities. For better initial solutions, consider cheapest insertion.
///
/// # Type Parameters
///
/// - `S`: Solution type implementing `ListVariableSolution` and `ShadowVariableSupport`
/// - `C`: Constraint set type (fully typed, no trait objects)
///
/// # Example
///
/// ```ignore
/// use solverforge_solver::phase::fluent::list_construction_phase;
///
/// let mut director = TypedScoreDirector::new(solution, constraints);
/// list_construction_phase(&mut director);
/// // All visits now assigned to vehicles
/// ```
#[inline]
pub fn list_construction_phase<S, C>(director: &mut TypedScoreDirector<S, C>)
where
    S: PlanningSolution + ListVariableSolution + ShadowVariableSupport,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    let n_entities = director.working_solution().entity_count();
    if n_entities == 0 {
        return;
    }

    let unassigned = director.working_solution().unassigned_elements();

    for (i, elem) in unassigned.into_iter().enumerate() {
        let entity_idx = i % n_entities;
        director.do_change_with_shadows(entity_idx, |solution| {
            solution.list_push(entity_idx, elem);
        });
    }

    // Ensure score is calculated after construction
    director.calculate_score();
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::score::SimpleScore;
    use std::collections::HashSet;

    #[derive(Clone)]
    struct Vehicle {
        visits: Vec<usize>,
    }

    #[derive(Clone)]
    struct TestSolution {
        vehicles: Vec<Vehicle>,
        visit_count: usize,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl ShadowVariableSupport for TestSolution {
        fn update_entity_shadows(&mut self, _entity_index: usize) {
            // No shadow variables in test
        }
    }

    impl ListVariableSolution for TestSolution {
        type Element = usize;

        fn entity_count(&self) -> usize {
            self.vehicles.len()
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            self.vehicles[entity_idx].visits.len()
        }

        fn list_get(&self, entity_idx: usize, position: usize) -> Self::Element {
            self.vehicles[entity_idx].visits[position]
        }

        fn list_push(&mut self, entity_idx: usize, elem: Self::Element) {
            self.vehicles[entity_idx].visits.push(elem);
        }

        fn list_insert(&mut self, entity_idx: usize, position: usize, elem: Self::Element) {
            self.vehicles[entity_idx].visits.insert(position, elem);
        }

        fn list_remove(&mut self, entity_idx: usize, position: usize) -> Self::Element {
            self.vehicles[entity_idx].visits.remove(position)
        }

        fn list_reverse(&mut self, entity_idx: usize, start: usize, end: usize) {
            self.vehicles[entity_idx].visits[start..end].reverse();
        }

        fn unassigned_elements(&self) -> Vec<Self::Element> {
            let assigned: HashSet<usize> = self
                .vehicles
                .iter()
                .flat_map(|v| v.visits.iter().copied())
                .collect();
            (0..self.visit_count)
                .filter(|i| !assigned.contains(i))
                .collect()
        }
    }

    #[test]
    fn test_round_robin_assignment() {
        let solution = TestSolution {
            vehicles: vec![
                Vehicle { visits: vec![] },
                Vehicle { visits: vec![] },
                Vehicle { visits: vec![] },
            ],
            visit_count: 6,
            score: None,
        };

        let mut director = TypedScoreDirector::new(solution, ());

        list_construction_phase(&mut director);

        let sol = director.working_solution();
        // 6 visits across 3 vehicles = 2 each
        assert_eq!(sol.vehicles[0].visits.len(), 2);
        assert_eq!(sol.vehicles[1].visits.len(), 2);
        assert_eq!(sol.vehicles[2].visits.len(), 2);

        // All visits assigned
        assert!(sol.unassigned_elements().is_empty());
    }
}
