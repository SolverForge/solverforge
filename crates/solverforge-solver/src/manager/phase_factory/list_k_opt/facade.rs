//! Function-pointer adapter that drives the live kernel in tests.

use super::*;
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use std::marker::PhantomData;

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
pub(crate) struct ListKOptPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    k: usize,
    entity_count: fn(&S) -> usize,
    get_route: fn(&S, usize) -> Vec<usize>,
    set_route: fn(&mut S, usize, Vec<usize>),
    depot_fn: fn(&S, usize) -> usize,
    distance_fn: fn(&S, usize, usize, usize) -> i64,
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
    /// Creates a new k-opt polishing phase.
    ///
    /// # Arguments
    ///
    /// * `k` — k value; only k=2 (2-opt) is implemented; k>2 logs a warning and is a no-op
    /// * `entity_count` — number of entities (routes) in the solution
    /// * `get_route` — returns the route for entity at given index as element indices
    /// * `set_route` — replaces the route for entity at given index
    /// * `depot_fn` — returns the depot element index for a given entity
    /// * `distance_fn` — distance between two element indices for the given entity
    /// * `feasible_fn` — optional feasibility gate; receives solution, entity index, and
    ///   candidate route after tentative reversal; return `false` to reject the move
    /// * `descriptor_index` — entity descriptor index for change notification
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        k: usize,
        entity_count: fn(&S) -> usize,
        get_route: fn(&S, usize) -> Vec<usize>,
        set_route: fn(&mut S, usize, Vec<usize>),
        depot_fn: fn(&S, usize) -> usize,
        distance_fn: fn(&S, usize, usize, usize) -> i64,
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

impl<S, E> ListKOptAccess<S> for ListKOptPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_count(&self, solution: &S) -> usize {
        (self.entity_count)(solution)
    }

    fn route_values(&self, solution: &S, entity_index: usize) -> Vec<usize> {
        (self.get_route)(solution, entity_index)
    }

    fn replace_route(&self, solution: &mut S, entity_index: usize, route: Vec<usize>) {
        (self.set_route)(solution, entity_index, route);
    }

    fn route_depot(&self, solution: &S, entity_index: usize) -> usize {
        (self.depot_fn)(solution, entity_index)
    }

    fn route_distance(&self, solution: &S, entity_index: usize, from: usize, to: usize) -> i64 {
        (self.distance_fn)(solution, entity_index, from, to)
    }

    fn route_feasible(&self, solution: &S, entity_index: usize, route: &[usize]) -> bool {
        self.feasible_fn
            .is_none_or(|feasible| feasible(solution, entity_index, route))
    }
}

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListKOptPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
        let k = self.k;
        run_list_k_opt(
            self,
            k,
            StepControlPolicy::ObserveConfigLimits,
            solver_scope,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ListKOpt"
    }
}
