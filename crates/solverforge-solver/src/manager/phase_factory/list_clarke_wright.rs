/* Clarke-Wright savings construction phase for list-variable problems.

Builds routes by computing savings for every element pair, then greedily
merging singleton routes in descending savings order subject to owner-aware
route feasibility.

Public list hooks operate in actual stored list values. This phase may use
collection indices internally for dense bookkeeping, but it must normalize
them before calling any domain callback.
*/

use std::collections::HashSet;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

mod owner_assignment;
mod savings;

use owner_assignment::{feasible_owners, match_route_owners};
use savings::{sort_savings, SavingsEntry};

/// List construction phase using Clarke-Wright savings algorithm.
///
/// Builds routes by computing a savings value for every pair of elements,
/// then greedily merges singleton routes in descending savings order,
/// subject to owner-aware route feasibility. All domain knowledge is supplied
/// by the caller via function pointers.
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
///        - Skip if no route owner can accept the merged route.
///        - Skip if i is not an endpoint of its route, or j is not.
///        - Orient and merge the two routes.
/// 5. Assign non-empty routes to empty entities through deterministic matching.
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
///     |_p, _entity_idx| 0,            // depot value
///     |_p, _entity_idx, i, j| (i as i64 - j as i64).abs(),
///     |_p, _entity_idx, route| route.iter().sum::<usize>() <= 10,
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
    assign_route: fn(&mut S, usize, Vec<usize>),
    index_to_element: fn(&S, usize) -> E,
    depot_fn: fn(&S, usize) -> usize,
    distance_fn: fn(&S, usize, usize, usize) -> i64,
    feasible_fn: fn(&S, usize, &[usize]) -> bool,
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
    * `assign_route` — Assigns a complete route of element values to entity at index
    * `index_to_element` — Converts an element index to its domain value
    * `depot_fn` — Returns the depot value for the route owner
    * `distance_fn` — Distance between two actual element values for the route owner
    * `feasible_fn` — Owner-aware hard feasibility gate. Receives the solution,
    route owner, and candidate route as actual element values; return `false`
    to skip the route-owner assignment.
    * `descriptor_index` — Entity descriptor index for change notification
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        route_len: fn(&S, usize) -> usize,
        assign_route: fn(&mut S, usize, Vec<usize>),
        index_to_element: fn(&S, usize) -> E,
        depot_fn: fn(&S, usize) -> usize,
        distance_fn: fn(&S, usize, usize, usize) -> i64,
        feasible_fn: fn(&S, usize, &[usize]) -> bool,
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
            feasible_fn,
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
        let index_to_element = self.index_to_element;
        let available_entity_slots: Vec<usize> = (0..n_entities)
            .filter(|&entity_idx| (self.route_len)(solution, entity_idx) == 0)
            .collect();

        if available_entity_slots.is_empty() {
            tracing::warn!(
                unassigned_elements = n_elements.saturating_sub(assigned.len()),
                "ListClarkeWright found no empty entity slots for remaining work; leaving preassigned routes untouched"
            );
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let depot_values: HashSet<usize> = available_entity_slots
            .iter()
            .map(|&entity_idx| (self.depot_fn)(solution, entity_idx))
            .collect();
        let assigned_set: HashSet<E> = assigned.into_iter().collect();
        let unassigned_indices: Vec<usize> = (0..n_elements)
            .filter(|&i| {
                let element = index_to_element(solution, i);
                !depot_values.contains(&element.into()) && !assigned_set.contains(&element)
            })
            .collect();

        let n = unassigned_indices.len();
        if n == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        // Initialize singleton routes before savings generation so we can always
        // commit a meaningful partial construction if termination fires mid-phase.
        let mut routes: Vec<Vec<usize>> = unassigned_indices.iter().map(|&i| vec![i]).collect();

        // Map collection index -> route index.
        let mut route_of: Vec<Option<usize>> = vec![None; n_elements];
        for (route_idx, &elem_idx) in unassigned_indices.iter().enumerate() {
            route_of[elem_idx] = Some(route_idx);
        }

        // Compute savings for all pairs (i, j) where i < j in unassigned_indices,
        // calling domain hooks only in actual value space.
        let distance_fn = self.distance_fn;
        let mut savings: Vec<SavingsEntry> = Vec::with_capacity(
            n.saturating_mul(n.saturating_sub(1))
                .saturating_div(2)
                .saturating_mul(available_entity_slots.len()),
        );
        let mut construction_interrupted = false;
        'savings_generation: for &owner_idx in &available_entity_slots {
            for a in 0..n {
                if (a & 0x1F) == 0
                    && phase_scope
                        .solver_scope_mut()
                        .should_terminate_construction()
                {
                    construction_interrupted = true;
                    break 'savings_generation;
                }
                for b in (a + 1)..n {
                    let left_idx = unassigned_indices[a];
                    let right_idx = unassigned_indices[b];
                    let solution = phase_scope.score_director().working_solution();
                    let depot = (self.depot_fn)(solution, owner_idx);
                    let left_value = index_to_element(solution, left_idx).into();
                    let right_value = index_to_element(solution, right_idx).into();
                    let saving = distance_fn(solution, owner_idx, depot, left_value)
                        + distance_fn(solution, owner_idx, depot, right_value)
                        - distance_fn(solution, owner_idx, left_value, right_value);
                    savings.push(SavingsEntry {
                        saving,
                        owner_idx,
                        left_idx,
                        right_idx,
                    });
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
        }
        sort_savings(&mut savings);

        // Greedy merge
        for (merge_idx, entry) in savings.iter().enumerate() {
            if (merge_idx & 0xFF) == 0
                && phase_scope
                    .solver_scope_mut()
                    .should_terminate_construction()
            {
                construction_interrupted = true;
                break;
            }
            let i = entry.left_idx;
            let j = entry.right_idx;

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

            let mut test_ri = routes[ri].clone();
            if test_ri.first() == Some(&i) {
                test_ri.reverse();
            }
            let mut test_rj = routes[rj].clone();
            if test_rj.last() == Some(&j) {
                test_rj.reverse();
            }

            let candidate_indices: Vec<usize> = test_ri.iter().chain(&test_rj).copied().collect();
            let solution = phase_scope.score_director().working_solution();
            let candidate_route = route_values(solution, index_to_element, &candidate_indices);
            if feasible_owners(
                solution,
                &available_entity_slots,
                &candidate_route,
                self.feasible_fn,
            )
            .is_empty()
            {
                continue;
            }

            routes[ri] = test_ri;
            routes[rj].clear();
            for &c in &test_rj {
                route_of[c] = Some(ri);
            }
            routes[ri].extend(test_rj);
        }

        // Assign all constructed routes in one step so an interrupted construction
        // still commits a coherent partial solution instead of leaving the working
        // solution empty.
        let non_empty: Vec<Vec<usize>> = routes.into_iter().filter(|r| !r.is_empty()).collect();
        let solution = phase_scope.score_director().working_solution();
        let feasible_sets: Vec<Vec<usize>> = non_empty
            .iter()
            .map(|route| {
                let values = route_values(solution, index_to_element, route);
                feasible_owners(solution, &available_entity_slots, &values, self.feasible_fn)
            })
            .collect();
        let route_to_owner = match_route_owners(&feasible_sets);
        let matched_count = route_to_owner
            .iter()
            .filter(|owner| owner.is_some())
            .count();

        if matched_count < non_empty.len() && !construction_interrupted {
            tracing::warn!(
                constructed_routes = non_empty.len(),
                available_slots = available_entity_slots.len(),
                matched_routes = matched_count,
                "ListClarkeWright could not match every constructed route to a feasible empty entity"
            );
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        if matched_count > 0 {
            let assign_route = self.assign_route;
            let descriptor_index = self.descriptor_index;
            let mut step_scope = StepScope::new(&mut phase_scope);

            step_scope.apply_committed_change(|sd| {
                for (index_route, entity_idx) in non_empty.into_iter().zip(route_to_owner) {
                    let Some(entity_idx) = entity_idx else {
                        continue;
                    };
                    sd.before_variable_changed(descriptor_index, entity_idx);
                    let route = route_values(sd.working_solution(), index_to_element, &index_route);
                    assign_route(sd.working_solution_mut(), entity_idx, route);
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

fn route_values<S, E>(
    solution: &S,
    index_to_element: fn(&S, usize) -> E,
    route: &[usize],
) -> Vec<usize>
where
    E: Copy + Into<usize>,
{
    route
        .iter()
        .map(|&idx| index_to_element(solution, idx).into())
        .collect()
}

#[cfg(test)]
mod tests;
