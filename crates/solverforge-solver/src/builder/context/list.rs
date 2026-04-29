use std::fmt;
use std::marker::PhantomData;

use crate::heuristic::selector::k_opt::ListPositionDistanceMeter;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

pub struct IntraDistanceAdapter<T>(pub T);

impl<S, T: CrossEntityDistanceMeter<S>> ListPositionDistanceMeter<S> for IntraDistanceAdapter<T> {
    fn distance(&self, solution: &S, entity_idx: usize, pos_a: usize, pos_b: usize) -> f64 {
        self.0
            .distance(solution, entity_idx, pos_a, entity_idx, pos_b)
    }
}

pub struct ListVariableContext<S, V, DM, IDM> {
    pub entity_type_name: &'static str,
    pub element_count: fn(&S) -> usize,
    pub assigned_elements: fn(&S) -> Vec<V>,
    pub list_len: fn(&S, usize) -> usize,
    pub list_remove: fn(&mut S, usize, usize) -> Option<V>,
    pub construction_list_remove: fn(&mut S, usize, usize) -> V,
    pub list_insert: fn(&mut S, usize, usize, V),
    pub list_get: fn(&S, usize, usize) -> Option<V>,
    pub list_set: fn(&mut S, usize, usize, V),
    pub list_reverse: fn(&mut S, usize, usize, usize),
    pub sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    pub sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    pub ruin_remove: fn(&mut S, usize, usize) -> V,
    pub ruin_insert: fn(&mut S, usize, usize, V),
    pub index_to_element: fn(&S, usize) -> V,
    pub entity_count: fn(&S) -> usize,
    pub cross_distance_meter: DM,
    pub intra_distance_meter: IDM,
    pub variable_name: &'static str,
    pub descriptor_index: usize,
    pub merge_feasible_fn: Option<fn(&S, &[usize]) -> bool>,
    pub cw_depot_fn: Option<fn(&S) -> usize>,
    pub cw_distance_fn: Option<fn(&S, usize, usize) -> i64>,
    pub cw_element_load_fn: Option<fn(&S, usize) -> i64>,
    pub cw_capacity_fn: Option<fn(&S) -> i64>,
    pub cw_assign_route_fn: Option<fn(&mut S, usize, Vec<V>)>,
    pub k_opt_get_route: Option<fn(&S, usize) -> Vec<usize>>,
    pub k_opt_set_route: Option<fn(&mut S, usize, Vec<usize>)>,
    pub k_opt_depot_fn: Option<fn(&S, usize) -> usize>,
    pub k_opt_distance_fn: Option<fn(&S, usize, usize) -> i64>,
    pub k_opt_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, DM: Clone, IDM: Clone> Clone for ListVariableContext<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        Self {
            entity_type_name: self.entity_type_name,
            element_count: self.element_count,
            assigned_elements: self.assigned_elements,
            list_len: self.list_len,
            list_remove: self.list_remove,
            construction_list_remove: self.construction_list_remove,
            list_insert: self.list_insert,
            list_get: self.list_get,
            list_set: self.list_set,
            list_reverse: self.list_reverse,
            sublist_remove: self.sublist_remove,
            sublist_insert: self.sublist_insert,
            ruin_remove: self.ruin_remove,
            ruin_insert: self.ruin_insert,
            index_to_element: self.index_to_element,
            entity_count: self.entity_count,
            cross_distance_meter: self.cross_distance_meter.clone(),
            intra_distance_meter: self.intra_distance_meter.clone(),
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            merge_feasible_fn: self.merge_feasible_fn,
            cw_depot_fn: self.cw_depot_fn,
            cw_distance_fn: self.cw_distance_fn,
            cw_element_load_fn: self.cw_element_load_fn,
            cw_capacity_fn: self.cw_capacity_fn,
            cw_assign_route_fn: self.cw_assign_route_fn,
            k_opt_get_route: self.k_opt_get_route,
            k_opt_set_route: self.k_opt_set_route,
            k_opt_depot_fn: self.k_opt_depot_fn,
            k_opt_distance_fn: self.k_opt_distance_fn,
            k_opt_feasible_fn: self.k_opt_feasible_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, DM, IDM> ListVariableContext<S, V, DM, IDM> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_type_name: &'static str,
        element_count: fn(&S) -> usize,
        assigned_elements: fn(&S) -> Vec<V>,
        list_len: fn(&S, usize) -> usize,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        construction_list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        list_reverse: fn(&mut S, usize, usize, usize),
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        ruin_remove: fn(&mut S, usize, usize) -> V,
        ruin_insert: fn(&mut S, usize, usize, V),
        index_to_element: fn(&S, usize) -> V,
        entity_count: fn(&S) -> usize,
        cross_distance_meter: DM,
        intra_distance_meter: IDM,
        variable_name: &'static str,
        descriptor_index: usize,
        merge_feasible_fn: Option<fn(&S, &[usize]) -> bool>,
        cw_depot_fn: Option<fn(&S) -> usize>,
        cw_distance_fn: Option<fn(&S, usize, usize) -> i64>,
        cw_element_load_fn: Option<fn(&S, usize) -> i64>,
        cw_capacity_fn: Option<fn(&S) -> i64>,
        cw_assign_route_fn: Option<fn(&mut S, usize, Vec<V>)>,
        k_opt_get_route: Option<fn(&S, usize) -> Vec<usize>>,
        k_opt_set_route: Option<fn(&mut S, usize, Vec<usize>)>,
        k_opt_depot_fn: Option<fn(&S, usize) -> usize>,
        k_opt_distance_fn: Option<fn(&S, usize, usize) -> i64>,
        k_opt_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    ) -> Self {
        Self {
            entity_type_name,
            element_count,
            assigned_elements,
            list_len,
            list_remove,
            construction_list_remove,
            list_insert,
            list_get,
            list_set,
            list_reverse,
            sublist_remove,
            sublist_insert,
            ruin_remove,
            ruin_insert,
            index_to_element,
            entity_count,
            cross_distance_meter,
            intra_distance_meter,
            variable_name,
            descriptor_index,
            merge_feasible_fn,
            cw_depot_fn,
            cw_distance_fn,
            cw_element_load_fn,
            cw_capacity_fn,
            cw_assign_route_fn,
            k_opt_get_route,
            k_opt_set_route,
            k_opt_depot_fn,
            k_opt_distance_fn,
            k_opt_feasible_fn,
            _phantom: PhantomData,
        }
    }

    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        entity_class.is_none_or(|name| name == self.entity_type_name)
            && variable_name.is_none_or(|name| name == self.variable_name)
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for ListVariableContext<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListVariableContext")
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}
