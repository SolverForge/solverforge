/* Per-route k-opt polishing phase for list-variable problems.

Applies 2-opt local search to each entity's route independently,
improving solution quality after construction without modifying the
overall assignment structure.
*/

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

/// Per-route k-opt polishing phase for list variable problems.
///
/// Runs 2-opt local search on each entity's route to local optimum after
/// construction. All domain knowledge is supplied via function pointers.
///
/// # Algorithm (k=2, 2-opt)
///
/// For each entity:
/// 1. Read the current route as a sequence of element indices.
/// 2. Try every segment reversal `[i..=j]`.
/// 3. Accept if distance improves AND `feasible_fn` passes (if provided).
/// 4. Repeat until no improving reversal exists.
///
/// # Example
///
/// ```
/// use solverforge_solver::ListKOptPhase;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone)]
/// struct Vehicle { route: Vec<usize> }
///
/// #[derive(Clone)]
/// struct Plan {
///     vehicles: Vec<Vehicle>,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for Plan {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// let phase = ListKOptPhase::<Plan, usize>::new(
///     2,
///     |p| p.vehicles.len(),
///     |p, entity_idx| p.vehicles[entity_idx].route.clone(),
///     |p, entity_idx, route| { p.vehicles[entity_idx].route = route; },
///     |_p, entity_idx| 0,   // depot index
///     |_p, i, j| (i as i64 - j as i64).abs(),
///     None,                  // no extra feasibility check
///     0,
/// );
/// ```
pub struct ListKOptPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    k: usize,
    entity_count: fn(&S) -> usize,
    get_route: fn(&S, usize) -> Vec<usize>,
    set_route: fn(&mut S, usize, Vec<usize>),
    depot_fn: fn(&S, usize) -> usize,
    distance_fn: fn(&S, usize, usize) -> i64,
    feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    descriptor_index: usize,
    _marker: PhantomData<fn() -> (S, E)>,
}

impl<S, E> std::fmt::Debug for ListKOptPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListKOptPhase").field("k", &self.k).finish()
    }
}

impl<S, E> ListKOptPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    /* Creates a new k-opt polishing phase.

    # Arguments

    * `k` — k value; only k=2 (2-opt) is implemented; k>2 logs a warning and is a no-op
    * `entity_count` — number of entities (routes) in the solution
    * `get_route` — returns the route for entity at given index as element indices
    * `set_route` — replaces the route for entity at given index
    * `depot_fn` — returns the depot element index for a given entity
    * `distance_fn` — distance between two element indices
    * `feasible_fn` — optional feasibility gate; receives solution, entity index, and
    candidate route after tentative reversal; return `false` to reject the move
    * `descriptor_index` — entity descriptor index for change notification
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        k: usize,
        entity_count: fn(&S) -> usize,
        get_route: fn(&S, usize) -> Vec<usize>,
        set_route: fn(&mut S, usize, Vec<usize>),
        depot_fn: fn(&S, usize) -> usize,
        distance_fn: fn(&S, usize, usize) -> i64,
        feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
        descriptor_index: usize,
    ) -> Self {
        Self {
            k,
            entity_count,
            get_route,
            set_route,
            depot_fn,
            distance_fn,
            feasible_fn,
            descriptor_index,
            _marker: PhantomData,
        }
    }
}

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListKOptPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
    D: Director<S>,
    BestCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        if self.k != 2 {
            tracing::warn!(
                k = self.k,
                "ListKOptPhase: only k=2 is implemented; skipping k-opt polishing"
            );
            let mut phase_scope = PhaseScope::new(solver_scope, 0);
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let n_entities = {
            let solution = phase_scope.score_director().working_solution();
            (self.entity_count)(solution)
        };

        if n_entities == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let distance_fn = self.distance_fn;
        let feasible_fn = self.feasible_fn;
        let depot_fn = self.depot_fn;
        let descriptor_index = self.descriptor_index;

        for entity_idx in 0..n_entities {
            let (depot, mut route) = {
                let solution = phase_scope.score_director().working_solution();
                let depot = depot_fn(solution, entity_idx);
                let route = (self.get_route)(solution, entity_idx);
                (depot, route)
            };

            let n = route.len();
            if n < 4 {
                continue;
            }

            let mut changed = false;

            // 2-opt: try all (i, j) segment reversals
            loop {
                let mut improved = false;
                for i in 0..n - 1 {
                    let a = if i == 0 { depot } else { route[i - 1] };
                    let b = route[i];
                    for j in i + 1..n {
                        let c = route[j];
                        let e = if j + 1 < n { route[j + 1] } else { depot };
                        // Accept if reversing [i..=j] reduces distance
                        let solution = phase_scope.score_director().working_solution();
                        if distance_fn(solution, a, c) + distance_fn(solution, b, e)
                            < distance_fn(solution, a, b) + distance_fn(solution, c, e)
                        {
                            route[i..=j].reverse();
                            // Check optional feasibility gate; revert if infeasible
                            if let Some(f) = feasible_fn {
                                let solution = phase_scope.score_director().working_solution();
                                if !f(solution, entity_idx, &route) {
                                    route[i..=j].reverse();
                                    continue;
                                }
                            }
                            improved = true;
                            changed = true;
                        }
                    }
                }
                if !improved {
                    break;
                }
            }

            if changed {
                let mut step_scope = StepScope::new(&mut phase_scope);
                {
                    let sd = step_scope.score_director_mut();
                    sd.before_variable_changed(descriptor_index, entity_idx);
                    (self.set_route)(sd.working_solution_mut(), entity_idx, route);
                    sd.after_variable_changed(descriptor_index, entity_idx);
                }
                let step_score = step_scope.calculate_score();
                step_scope.set_step_score(step_score);
                step_scope.complete();
            }
        }

        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ListKOpt"
    }
}
