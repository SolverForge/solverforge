use std::fmt;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::scope::{ProgressCallback, SolverScope};

// The immutable recursive graph compiler is the sole runtime construction
// entrypoint for typed and dynamic models.
#[path = "runtime/compiler/mod.rs"]
pub(crate) mod compiler;
#[path = "runtime/provider_cursor.rs"]
pub(crate) mod provider_cursor;

#[cfg(test)]
#[path = "runtime/provider_cursor_tests.rs"]
mod provider_cursor_tests;

pub struct ListVariableMetadata<S, DM, IDM> {
    pub cross_distance_meter: DM,
    pub intra_distance_meter: IDM,
    pub route_get_fn: Option<fn(&S, usize) -> Vec<usize>>,
    pub route_set_fn: Option<fn(&mut S, usize, Vec<usize>)>,
    pub route_depot_fn: Option<fn(&S, usize) -> usize>,
    pub route_distance_fn: Option<fn(&S, usize, usize, usize) -> i64>,
    pub route_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    pub savings_depot_fn: Option<fn(&S, usize) -> usize>,
    pub savings_metric_class_fn: Option<fn(&S, usize) -> usize>,
    pub savings_distance_fn: Option<fn(&S, usize, usize, usize) -> i64>,
    pub savings_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    pub element_owner_fn: Option<fn(&S, &usize) -> Option<usize>>,
    _phantom: PhantomData<fn() -> S>,
}

pub trait ListVariableEntity<S> {
    type CrossDistanceMeter: CrossEntityDistanceMeter<S> + Clone + fmt::Debug;
    type IntraDistanceMeter: CrossEntityDistanceMeter<S> + Clone + fmt::Debug + 'static;

    const HAS_LIST_VARIABLE: bool;
    const LIST_VARIABLE_NAME: &'static str;
    const LIST_ELEMENT_SOURCE: Option<&'static str>;

    fn list_field(entity: &Self) -> &[usize];
    fn list_field_mut(entity: &mut Self) -> &mut Vec<usize>;
    fn list_metadata() -> ListVariableMetadata<S, Self::CrossDistanceMeter, Self::IntraDistanceMeter>;
}

impl<S, DM, IDM> ListVariableMetadata<S, DM, IDM> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cross_distance_meter: DM,
        intra_distance_meter: IDM,
        route_get_fn: Option<fn(&S, usize) -> Vec<usize>>,
        route_set_fn: Option<fn(&mut S, usize, Vec<usize>)>,
        route_depot_fn: Option<fn(&S, usize) -> usize>,
        route_distance_fn: Option<fn(&S, usize, usize, usize) -> i64>,
        route_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
        savings_depot_fn: Option<fn(&S, usize) -> usize>,
        savings_metric_class_fn: Option<fn(&S, usize) -> usize>,
        savings_distance_fn: Option<fn(&S, usize, usize, usize) -> i64>,
        savings_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    ) -> Self {
        Self {
            cross_distance_meter,
            intra_distance_meter,
            route_get_fn,
            route_set_fn,
            route_depot_fn,
            route_distance_fn,
            route_feasible_fn,
            savings_depot_fn,
            savings_metric_class_fn,
            savings_distance_fn,
            savings_feasible_fn,
            element_owner_fn: None,
            _phantom: PhantomData,
        }
    }

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &usize) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }
}

pub(crate) fn finalize_noop_construction<S, D, ProgressCb>(
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let had_best = solver_scope.best_score().is_some();
    solver_scope.update_best_solution();
    if had_best {
        solver_scope.promote_current_solution_on_score_tie();
    }
}
