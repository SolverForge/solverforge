//! Local search phases for zero-erasure fluent API.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use solverforge_core::domain::{ListVariableSolution, PlanningSolution};
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::director::typed::TypedScoreDirector;
use solverforge_scoring::director::ShadowVariableSupport;

/// 2-opt local search phase with late acceptance.
///
/// Iteratively applies 2-opt moves (segment reversals) to each entity's list,
/// accepting moves that improve the score or match a late acceptance criterion.
///
/// # Arguments
///
/// - `director`: The typed score director managing the solution
/// - `time_limit`: Maximum duration for this phase
/// - `terminate`: External termination flag (for async cancellation)
///
/// # Algorithm
///
/// For each entity's list, tries all 2-opt moves (reversing segment [i+1..j]).
/// Uses late acceptance: accepts moves that improve over the score from N steps ago.
///
/// # Example
///
/// ```ignore
/// use solverforge_solver::phase::fluent::two_opt_phase;
/// use std::sync::atomic::AtomicBool;
/// use std::time::Duration;
///
/// let terminate = AtomicBool::new(false);
/// two_opt_phase(&mut director, Duration::from_secs(30), &terminate);
/// ```
#[inline]
pub fn two_opt_phase<S, C>(
    director: &mut TypedScoreDirector<S, C>,
    time_limit: Duration,
    terminate: &AtomicBool,
) where
    S: PlanningSolution + ListVariableSolution + ShadowVariableSupport,
    S::Score: Score + Ord,
    C: ConstraintSet<S, S::Score>,
{
    const LATE_ACCEPTANCE_SIZE: usize = 400;

    let start = Instant::now();

    // Initialize score and late acceptance buffer
    let mut current_score = director.calculate_score();
    let mut late_scores = vec![current_score.clone(); LATE_ACCEPTANCE_SIZE];
    let mut best_score = current_score.clone();
    let mut step = 0usize;

    while !terminate.load(Ordering::Relaxed) && start.elapsed() < time_limit {
        let n_entities = director.working_solution().entity_count();
        let mut improved_this_round = false;

        for entity_idx in 0..n_entities {
            let list_len = director.working_solution().list_len(entity_idx);
            if list_len < 4 {
                continue;
            }

            for i in 0..list_len.saturating_sub(2) {
                for j in (i + 2)..list_len {
                    if terminate.load(Ordering::Relaxed) || start.elapsed() >= time_limit {
                        return;
                    }

                    // Apply 2-opt: reverse segment [i+1..j+1)
                    let new_score = director.do_change_with_shadows(entity_idx, |solution| {
                        solution.list_reverse(entity_idx, i + 1, j + 1);
                    });

                    let late_idx = step % LATE_ACCEPTANCE_SIZE;
                    let late_score = &late_scores[late_idx];

                    // Accept if better than late score OR better than best
                    if new_score >= *late_score || new_score > best_score {
                        // Accept move
                        late_scores[late_idx] = new_score.clone();
                        if new_score > best_score {
                            best_score = new_score.clone();
                            improved_this_round = true;
                        }
                        current_score = new_score;
                        step += 1;
                    } else {
                        // Reject - undo the move
                        director.do_change_with_shadows(entity_idx, |solution| {
                            solution.list_reverse(entity_idx, i + 1, j + 1);
                        });
                    }
                }
            }
        }

        // If no improvement in a full round, we might be stuck
        // Continue anyway - late acceptance allows escaping local optima
        if !improved_this_round && step > LATE_ACCEPTANCE_SIZE * 2 {
            // Could add diversification here
        }
    }
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
        fn update_entity_shadows(&mut self, _entity_index: usize) {}
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
    fn test_two_opt_terminates() {
        let solution = TestSolution {
            vehicles: vec![Vehicle {
                visits: vec![0, 1, 2, 3, 4, 5],
            }],
            visit_count: 6,
            score: None,
        };

        let mut director = TypedScoreDirector::new(solution, ());
        let terminate = AtomicBool::new(false);

        // Should terminate within time limit
        two_opt_phase(&mut director, Duration::from_millis(100), &terminate);

        // Solution should still be valid
        let sol = director.working_solution();
        assert_eq!(sol.vehicles[0].visits.len(), 6);
    }

    #[test]
    fn test_two_opt_respects_terminate_flag() {
        let solution = TestSolution {
            vehicles: vec![Vehicle {
                visits: vec![0, 1, 2, 3, 4, 5],
            }],
            visit_count: 6,
            score: None,
        };

        let mut director = TypedScoreDirector::new(solution, ());
        let terminate = AtomicBool::new(true); // Already terminated

        let start = Instant::now();
        two_opt_phase(&mut director, Duration::from_secs(10), &terminate);

        // Should return immediately
        assert!(start.elapsed() < Duration::from_millis(100));
    }
}
