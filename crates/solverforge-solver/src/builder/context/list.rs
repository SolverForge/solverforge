use std::fmt;
use std::marker::PhantomData;

#[cfg(test)]
/// Canonical source key for the built-in `usize` planning-list representation.
/// Generated list models and dynamic bridge lists use their logical element ID
/// as the stable key; the per-run runtime binder maps it to source order.
pub(crate) fn usize_element_source_key<S>(_solution: &S, element: &usize) -> usize {
    *element
}

/// Function-pointer access to one list planning variable of a typed model.
///
/// `S` is the solution, `V` the list element type, and `DM` / `IDM` the
/// cross-entity and intra-entity distance meters nearby list selectors use.
/// Entity arguments are indexes into the owning entity collection and
/// positions are indexes into that entity's list. Generated models build
/// this through `planning_model!`.
pub struct ListVariableSlot<S, V, DM, IDM> {
    /// Type name of the entity that owns the list.
    pub entity_type_name: &'static str,
    /// Number of elements the lists draw from.
    pub element_count: fn(&S) -> usize,
    /// Every element currently assigned to some entity's list.
    pub assigned_elements: fn(&S) -> Vec<V>,
    /// Length of entity `entity`'s list.
    pub list_len: fn(&S, usize) -> usize,
    /// Removes and returns the element at `(entity, position)`; `None` when the
    /// entity does not exist.
    pub list_remove: fn(&mut S, usize, usize) -> Option<V>,
    /// Removes and returns the element at `(entity, position)`. Kept on the
    /// slot; the current solver does not call it.
    pub construction_list_remove: fn(&mut S, usize, usize) -> V,
    /// Inserts an element at `(entity, position)`.
    pub list_insert: fn(&mut S, usize, usize, V),
    /// The element at `(entity, position)`, if any.
    pub list_get: fn(&S, usize, usize) -> Option<V>,
    /// Replaces the element at `(entity, position)`.
    pub list_set: fn(&mut S, usize, usize, V),
    /// Reverses the range `start..end` of entity `entity`'s list.
    pub list_reverse: fn(&mut S, usize, usize, usize),
    /// Removes and returns the range `start..end` of entity `entity`'s list.
    pub sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    /// Inserts elements into entity `entity`'s list starting at a position.
    pub sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    /// Removes and returns the element at `(entity, position)` for ruin moves.
    pub ruin_remove: fn(&mut S, usize, usize) -> V,
    /// Inserts an element at `(entity, position)` for ruin moves.
    pub ruin_insert: fn(&mut S, usize, usize, V),
    /// The element with the given source index (`0..element_count`).
    pub index_to_element: fn(&S, usize) -> V,
    /// Stable identity key for a list element. The per-run runtime binder
    /// resolves these keys to declared source indexes once; it never scans
    /// payloads by `PartialEq` during construction.
    pub element_source_key: fn(&S, &V) -> usize,
    /// Number of entities that own a list.
    pub entity_count: fn(&S) -> usize,
    /// Distance meter between positions of different entities, for nearby list
    /// selection.
    pub cross_distance_meter: DM,
    /// Distance meter between positions within one entity, for nearby list
    /// selection.
    pub intra_distance_meter: IDM,
    /// Name of the list variable.
    pub variable_name: &'static str,
    /// Index of the owning entity's descriptor.
    pub descriptor_index: usize,
    /// Whole-route getter; required with the other route hooks for k-opt.
    pub route_get_fn: Option<fn(&S, usize) -> Vec<usize>>,
    /// Replaces an entity's whole route in one mutation; required by k-opt and
    /// Clarke-Wright construction.
    pub route_set_fn: Option<fn(&mut S, usize, Vec<usize>)>,
    /// Depot of an entity's route, for route-local neighborhoods.
    pub route_depot_fn: Option<fn(&S, usize) -> usize>,
    /// Distance between two elements on an entity's route.
    pub route_distance_fn: Option<fn(&S, usize, usize, usize) -> i64>,
    /// Whether a candidate route for an entity is feasible.
    pub route_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    /// Depot of an entity for Clarke-Wright savings.
    pub savings_depot_fn: Option<fn(&S, usize) -> usize>,
    /// Metric class of an entity; owners in one class share depot and distance
    /// behavior for savings.
    pub savings_metric_class_fn: Option<fn(&S, usize) -> usize>,
    /// Distance between two elements for an entity's savings computation.
    pub savings_distance_fn: Option<fn(&S, usize, usize, usize) -> i64>,
    /// Whether a candidate route for an entity is feasible during savings
    /// construction.
    pub savings_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    /// The entity whose list must hold an element, when the model restricts
    /// ownership; `None` from the hook means any entity.
    pub element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    /// Construction order key of an element; lower keys are placed first.
    pub construction_element_order_key: Option<fn(&S, V) -> i64>,
    /// Duration of an element for precedence-aware list moves.
    pub precedence_duration_fn: Option<fn(&S, V) -> usize>,
    /// Appends the precedence successors of an element to the buffer.
    pub precedence_successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, DM: Clone, IDM: Clone> Clone for ListVariableSlot<S, V, DM, IDM> {
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
            element_source_key: self.element_source_key,
            entity_count: self.entity_count,
            cross_distance_meter: self.cross_distance_meter.clone(),
            intra_distance_meter: self.intra_distance_meter.clone(),
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            route_get_fn: self.route_get_fn,
            route_set_fn: self.route_set_fn,
            route_depot_fn: self.route_depot_fn,
            route_distance_fn: self.route_distance_fn,
            route_feasible_fn: self.route_feasible_fn,
            savings_depot_fn: self.savings_depot_fn,
            savings_metric_class_fn: self.savings_metric_class_fn,
            savings_distance_fn: self.savings_distance_fn,
            savings_feasible_fn: self.savings_feasible_fn,
            element_owner_fn: self.element_owner_fn,
            construction_element_order_key: self.construction_element_order_key,
            precedence_duration_fn: self.precedence_duration_fn,
            precedence_successors_fn: self.precedence_successors_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, DM, IDM> ListVariableSlot<S, V, DM, IDM> {
    /// Builds a slot from the required list operations and the optional route
    /// and savings hooks; ownership, construction-order and precedence hooks
    /// start unset.
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
        element_source_key: fn(&S, &V) -> usize,
        entity_count: fn(&S) -> usize,
        cross_distance_meter: DM,
        intra_distance_meter: IDM,
        variable_name: &'static str,
        descriptor_index: usize,
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
            element_source_key,
            entity_count,
            cross_distance_meter,
            intra_distance_meter,
            variable_name,
            descriptor_index,
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
            construction_element_order_key: None,
            precedence_duration_fn: None,
            precedence_successors_fn: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the element ownership hook.
    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }

    /// Sets the construction order key hook.
    pub fn with_construction_element_order_key(
        mut self,
        construction_element_order_key: Option<fn(&S, V) -> i64>,
    ) -> Self {
        self.construction_element_order_key = construction_element_order_key;
        self
    }

    /// Sets both precedence hooks.
    pub fn with_precedence_hooks(
        mut self,
        duration_fn: Option<fn(&S, V) -> usize>,
        successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
    ) -> Self {
        self.precedence_duration_fn = duration_fn;
        self.precedence_successors_fn = successors_fn;
        self
    }

    /// Whether this slot matches the given entity class and variable name;
    /// `None` matches any.
    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        entity_class.is_none_or(|name| name == self.entity_type_name)
            && variable_name.is_none_or(|name| name == self.variable_name)
    }

    /// Whether fewer elements are assigned than exist.
    pub fn has_unassigned_elements(&self, solution: &S) -> bool {
        (self.assigned_elements)(solution).len() < (self.element_count)(solution)
    }

    /// Whether any entity's list is non-empty.
    pub fn has_list_content(&self, solution: &S) -> bool {
        (0..(self.entity_count)(solution))
            .any(|entity_index| (self.list_len)(solution, entity_index) > 0)
    }

    /// Whether the route-replacement and savings depot, distance and
    /// feasibility hooks needed by Clarke-Wright construction are present.
    pub fn supports_clarke_wright(&self) -> bool {
        self.route_set_fn.is_some()
            && self.savings_depot_fn.is_some()
            && self.savings_distance_fn.is_some()
            && self.savings_feasible_fn.is_some()
    }

    /// Whether the route getter, setter, depot and distance hooks needed by
    /// k-opt are present.
    pub fn supports_k_opt(&self) -> bool {
        self.route_get_fn.is_some()
            && self.route_set_fn.is_some()
            && self.route_depot_fn.is_some()
            && self.route_distance_fn.is_some()
    }

    /// Whether ruin moves are available; always `true`.
    pub fn supports_ruin(&self) -> bool {
        true
    }

    /// Whether both precedence hooks are present.
    pub fn supports_precedence_moves(&self) -> bool {
        self.precedence_duration_fn.is_some() && self.precedence_successors_fn.is_some()
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for ListVariableSlot<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListVariableSlot")
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}
