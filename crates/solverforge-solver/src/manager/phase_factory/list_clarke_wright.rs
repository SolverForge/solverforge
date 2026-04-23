/* Clarke-Wright savings construction phase for list-variable problems.

Builds routes by computing savings for every element pair, then greedily
merging singleton routes in descending savings order subject to capacity.

Public list hooks operate in actual stored list values. This phase may use
collection indices internally for dense bookkeeping, but it must normalize
them before calling any domain callback.
*/

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

/// List construction phase using Clarke-Wright savings algorithm.
///
/// Builds routes by computing a savings value for every pair of elements,
/// then greedily merges singleton routes in descending savings order,
/// subject to a capacity constraint. All domain knowledge is supplied
/// by the caller via function pointers — no time-window feasibility or
/// post-processing is performed inside the phase.
///
/// # Algorithm
///
/// ```text
/// 1. For every pair (i, j) where i < j (depot excluded):
///        savings(i, j) = dist(depot, i) + dist(depot, j) - dist(i, j)
/// 2. Sort savings descending.
/// 3. Start with each element in its own singleton route.
/// 4. For each (i, j) in savings order:
///        - Skip if i or j are in the same route.
///        - Skip if merged load exceeds capacity.
///        - Skip if i is not an endpoint of its route, or j is not.
///        - Orient and merge the two routes.
/// 5. Assign each non-empty route to an empty entity via assign_route.
/// ```
///
/// # Example
///
/// ```
/// use solverforge_solver::ListClarkeWrightPhase;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone)]
/// struct Vehicle { route: Vec<usize> }
///
/// #[derive(Clone)]
/// struct Plan {
///     vehicles: Vec<Vehicle>,
///     n_stops: usize,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for Plan {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// let phase = ListClarkeWrightPhase::<Plan, usize>::new(
///     |p| p.n_stops,
///     |p| p.vehicles.iter().flat_map(|v| v.route.iter().copied()).collect(),
///     |p| p.vehicles.len(),
///     |p, entity_idx| p.vehicles.get(entity_idx).map_or(0, |v| v.route.len()),
///     |p, entity_idx, route| {
///         if let Some(v) = p.vehicles.get_mut(entity_idx) {
///             v.route = route;
///         }
///     },
///     |_p, idx| idx,
///     |_p| 0,            // depot value
///     |_p, i, j| (i as i64 - j as i64).abs(),  // distance between actual values
///     |_p, elem| elem as i64,  // load by actual value
///     |_p| 10,           // capacity per route
///     None,              // merge_feasible_fn: no extra feasibility check
///     0,
/// );
/// ```
pub struct ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Into<usize> + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    route_len: fn(&S, usize) -> usize,
    assign_route: fn(&mut S, usize, Vec<E>),
    index_to_element: fn(&S, usize) -> E,
    depot_fn: fn(&S) -> usize,
    distance_fn: fn(&S, usize, usize) -> i64,
    element_load_fn: fn(&S, usize) -> i64,
    capacity_fn: fn(&S) -> i64,
    merge_feasible_fn: Option<fn(&S, &[usize]) -> bool>,
    descriptor_index: usize,
    _marker: PhantomData<fn() -> (S, E)>,
}

impl<S, E> std::fmt::Debug for ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Into<usize> + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListClarkeWrightPhase").finish()
    }
}

impl<S, E> ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Into<usize> + Send + Sync + 'static,
{
    /* Creates a new Clarke-Wright savings construction phase.

    # Arguments

    * `element_count` — Total number of elements (stops) to assign
    * `get_assigned` — Returns currently assigned elements
    * `entity_count` — Number of entities (vehicles/routes)
    * `route_len` — Current list length for an entity; used to preserve preassigned routes
    * `assign_route` — Assigns a complete route `Vec<E>` to entity at index
    * `index_to_element` — Converts an element index to its domain value
    * `depot_fn` — Returns the depot value (excluded from savings pairs)
    * `distance_fn` — Distance between two actual element values
    * `element_load_fn` — Load contributed by an actual element value
    * `capacity_fn` — Maximum load per route
    * `merge_feasible_fn` — Optional feasibility gate called after capacity and endpoint checks.
    Receives the solution and the candidate merged route (as actual element values); return `false`
    to skip the merge.
    * `descriptor_index` — Entity descriptor index for change notification
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        route_len: fn(&S, usize) -> usize,
        assign_route: fn(&mut S, usize, Vec<E>),
        index_to_element: fn(&S, usize) -> E,
        depot_fn: fn(&S) -> usize,
        distance_fn: fn(&S, usize, usize) -> i64,
        element_load_fn: fn(&S, usize) -> i64,
        capacity_fn: fn(&S) -> i64,
        merge_feasible_fn: Option<fn(&S, &[usize]) -> bool>,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            get_assigned,
            entity_count,
            route_len,
            assign_route,
            index_to_element,
            depot_fn,
            distance_fn,
            element_load_fn,
            capacity_fn,
            merge_feasible_fn,
            descriptor_index,
            _marker: PhantomData,
        }
    }
}

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Eq + std::hash::Hash + Into<usize> + Send + Sync + 'static,
    D: Director<S>,
    BestCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let solution = phase_scope.score_director().working_solution();
        let n_elements = (self.element_count)(solution);
        let n_entities = (self.entity_count)(solution);

        if n_entities == 0 || n_elements == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned = (self.get_assigned)(phase_scope.score_director().working_solution());
        if assigned.len() >= n_elements {
            tracing::info!("All elements already assigned, skipping Clarke-Wright construction");
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let solution = phase_scope.score_director().working_solution();
        let depot = (self.depot_fn)(solution);
        let capacity = (self.capacity_fn)(solution);
        let index_to_element = self.index_to_element;
        let available_entity_slots: Vec<usize> = (0..n_entities)
            .filter(|&entity_idx| (self.route_len)(solution, entity_idx) == 0)
            .collect();

        // Collect unassigned collection indices, but compare domain hooks in actual value space.
        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();

        let unassigned_indices: Vec<usize> = (0..n_elements)
            .filter(|&i| {
                let element = index_to_element(solution, i);
                element.into() != depot && !assigned_set.contains(&element)
            })
            .collect();

        let n = unassigned_indices.len();
        if n == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }
        if available_entity_slots.is_empty() {
            tracing::warn!(
                unassigned_elements = n,
                "ListClarkeWright found no empty entity slots for remaining work; leaving preassigned routes untouched"
            );
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        // Initialize singleton routes before savings generation so we can always
        // commit a meaningful partial construction if termination fires mid-phase.
        let solution = phase_scope.score_director().working_solution();
        let element_load_fn = self.element_load_fn;
        let mut routes: Vec<Vec<usize>> = unassigned_indices.iter().map(|&i| vec![i]).collect();
        let mut route_load: Vec<i64> = unassigned_indices
            .iter()
            .map(|&i| element_load_fn(solution, index_to_element(solution, i).into()))
            .collect();

        // Map collection index -> route index (into routes/route_load)
        let mut route_of: Vec<Option<usize>> = vec![None; n_elements];
        for (route_idx, &elem_idx) in unassigned_indices.iter().enumerate() {
            route_of[elem_idx] = Some(route_idx);
        }

        // Compute savings for all pairs (i, j) where i < j in unassigned_indices,
        // calling domain hooks only in actual value space.
        let distance_fn = self.distance_fn;
        let mut savings: Vec<(i64, usize, usize)> = Vec::with_capacity(n * (n - 1) / 2);
        let mut construction_interrupted = false;
        'savings_generation: for a in 0..n {
            if (a & 0x1F) == 0
                && phase_scope
                    .solver_scope_mut()
                    .should_terminate_construction()
            {
                construction_interrupted = true;
                break 'savings_generation;
            }
            for b in (a + 1)..n {
                let i = unassigned_indices[a];
                let j = unassigned_indices[b];
                let solution = phase_scope.score_director().working_solution();
                let i_value = index_to_element(solution, i).into();
                let j_value = index_to_element(solution, j).into();
                let saving = distance_fn(solution, depot, i_value)
                    + distance_fn(solution, depot, j_value)
                    - distance_fn(solution, i_value, j_value);
                savings.push((saving, i, j));
                if (savings.len() & 0x3FF) == 0
                    && phase_scope
                        .solver_scope_mut()
                        .should_terminate_construction()
                {
                    construction_interrupted = true;
                    break 'savings_generation;
                }
            }
        }
        savings.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));

        // Greedy merge
        for (merge_idx, (_, i, j)) in savings.iter().enumerate() {
            if (merge_idx & 0xFF) == 0
                && phase_scope
                    .solver_scope_mut()
                    .should_terminate_construction()
            {
                construction_interrupted = true;
                break;
            }
            let i = *i;
            let j = *j;

            let ri = match route_of[i] {
                Some(r) => r,
                None => continue,
            };
            let rj = match route_of[j] {
                Some(r) => r,
                None => continue,
            };

            if ri == rj {
                continue;
            }

            if route_load[ri] + route_load[rj] > capacity {
                continue;
            }

            // i must be an endpoint of ri
            let i_is_endpoint = routes[ri].first() == Some(&i) || routes[ri].last() == Some(&i);
            if !i_is_endpoint {
                continue;
            }
            // j must be an endpoint of rj
            let j_is_endpoint = routes[rj].first() == Some(&j) || routes[rj].last() == Some(&j);
            if !j_is_endpoint {
                continue;
            }

            // Optional feasibility gate: build candidate using oriented copies BEFORE
            // modifying routes, exactly matching the template's test-then-commit pattern.
            if let Some(feasible) = self.merge_feasible_fn {
                let solution = phase_scope.score_director().working_solution();
                let mut test_ri = routes[ri].clone();
                if routes[ri].first() == Some(&i) {
                    test_ri.reverse();
                }
                let mut test_rj = routes[rj].clone();
                if routes[rj].last() == Some(&j) {
                    test_rj.reverse();
                }
                let candidate_route: Vec<usize> = test_ri
                    .into_iter()
                    .chain(test_rj)
                    .map(|idx| index_to_element(solution, idx).into())
                    .collect();
                if !feasible(solution, &candidate_route) {
                    continue;
                }
            }

            // Orient: i should be at the END of ri (so we can append rj after it)
            if routes[ri].first() == Some(&i) {
                routes[ri].reverse();
            }
            // Orient: j should be at the START of rj (so it connects to i)
            if routes[rj].last() == Some(&j) {
                routes[rj].reverse();
            }

            // Merge rj into ri
            let rj_elements: Vec<usize> = routes[rj].drain(..).collect();
            let new_load = route_load[ri] + route_load[rj];
            route_load[ri] = new_load;
            route_load[rj] = 0;
            for &c in &rj_elements {
                route_of[c] = Some(ri);
            }
            routes[ri].extend(rj_elements);
        }

        // Assign all constructed routes in one step so an interrupted construction
        // still commits a coherent partial solution instead of leaving the working
        // solution empty.
        let non_empty: Vec<Vec<usize>> = routes.into_iter().filter(|r| !r.is_empty()).collect();
        if non_empty.len() > available_entity_slots.len() && !construction_interrupted {
            tracing::warn!(
                constructed_routes = non_empty.len(),
                available_slots = available_entity_slots.len(),
                "ListClarkeWright could not fit remaining work into empty entities without overwriting preassigned routes"
            );
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        if !non_empty.is_empty() {
            let assign_route = self.assign_route;
            let descriptor_index = self.descriptor_index;
            let mut step_scope = StepScope::new(&mut phase_scope);

            step_scope.apply_committed_change(|sd| {
                for (entity_idx, index_route) in available_entity_slots.into_iter().zip(non_empty) {
                    sd.before_variable_changed(descriptor_index, entity_idx);
                    let element_route: Vec<E> = index_route
                        .iter()
                        .map(|&idx| index_to_element(sd.working_solution(), idx))
                        .collect();
                    assign_route(sd.working_solution_mut(), entity_idx, element_route);
                    sd.after_variable_changed(descriptor_index, entity_idx);
                }
            });

            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();
        }

        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ListClarkeWright"
    }
}

#[cfg(test)]
mod tests;
