use std::fmt;
use std::marker::PhantomData;

use crate::heuristic::selector::k_opt::ListPositionDistanceMeter;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

pub enum ValueSource<S> {
    Empty,
    CountableRange {
        from: usize,
        to: usize,
    },
    SolutionCount {
        count_fn: fn(&S, usize) -> usize,
        provider_index: usize,
    },
    EntitySlice {
        values_for_entity: for<'a> fn(&'a S, usize, usize) -> &'a [usize],
    },
}

impl<S> Clone for ValueSource<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ValueSource<S> {}

impl<S> fmt::Debug for ValueSource<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "ValueSource::Empty"),
            Self::CountableRange { from, to } => {
                write!(f, "ValueSource::CountableRange({from}..{to})")
            }
            Self::SolutionCount { provider_index, .. } => {
                write!(f, "ValueSource::SolutionCount(provider={provider_index})")
            }
            Self::EntitySlice { .. } => write!(f, "ValueSource::EntitySlice(..)"),
        }
    }
}

pub type ScalarGetter<S> = fn(&S, usize, usize) -> Option<usize>;
pub type ScalarSetter<S> = fn(&mut S, usize, usize, Option<usize>);
pub type ScalarCandidateValues<S> = for<'a> fn(&'a S, usize, usize) -> &'a [usize];
pub type NearbyValueDistanceMeter<S> = fn(&S, usize, usize, usize) -> Option<f64>;
pub type NearbyEntityDistanceMeter<S> = fn(&S, usize, usize, usize) -> Option<f64>;
pub type ConstructionEntityOrderKey<S> = fn(&S, usize, usize) -> Option<i64>;
pub type ConstructionValueOrderKey<S> = fn(&S, usize, usize, usize) -> Option<i64>;

pub struct ScalarVariableContext<S> {
    pub descriptor_index: usize,
    pub variable_index: usize,
    pub entity_type_name: &'static str,
    pub entity_count: fn(&S) -> usize,
    pub variable_name: &'static str,
    pub getter: ScalarGetter<S>,
    pub setter: ScalarSetter<S>,
    pub value_source: ValueSource<S>,
    pub allows_unassigned: bool,
    pub candidate_values: Option<ScalarCandidateValues<S>>,
    pub nearby_value_candidates: Option<ScalarCandidateValues<S>>,
    pub nearby_entity_candidates: Option<ScalarCandidateValues<S>>,
    pub nearby_value_distance_meter: Option<NearbyValueDistanceMeter<S>>,
    pub nearby_entity_distance_meter: Option<NearbyEntityDistanceMeter<S>>,
    pub construction_entity_order_key: Option<ConstructionEntityOrderKey<S>>,
    pub construction_value_order_key: Option<ConstructionValueOrderKey<S>>,
}

impl<S> Clone for ScalarVariableContext<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarVariableContext<S> {}

impl<S> ScalarVariableContext<S> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        descriptor_index: usize,
        variable_index: usize,
        entity_type_name: &'static str,
        entity_count: fn(&S) -> usize,
        variable_name: &'static str,
        getter: ScalarGetter<S>,
        setter: ScalarSetter<S>,
        value_source: ValueSource<S>,
        allows_unassigned: bool,
    ) -> Self {
        Self {
            descriptor_index,
            variable_index,
            entity_type_name,
            entity_count,
            variable_name,
            getter,
            setter,
            value_source,
            allows_unassigned,
            candidate_values: None,
            nearby_value_candidates: None,
            nearby_entity_candidates: None,
            nearby_value_distance_meter: None,
            nearby_entity_distance_meter: None,
            construction_entity_order_key: None,
            construction_value_order_key: None,
        }
    }

    pub fn with_nearby_value_distance_meter(mut self, meter: NearbyValueDistanceMeter<S>) -> Self {
        self.nearby_value_distance_meter = Some(meter);
        self
    }

    pub fn with_candidate_values(mut self, provider: ScalarCandidateValues<S>) -> Self {
        self.candidate_values = Some(provider);
        self
    }

    pub fn with_nearby_value_candidates(mut self, provider: ScalarCandidateValues<S>) -> Self {
        self.nearby_value_candidates = Some(provider);
        self
    }

    pub fn with_nearby_entity_candidates(mut self, provider: ScalarCandidateValues<S>) -> Self {
        self.nearby_entity_candidates = Some(provider);
        self
    }

    pub fn with_nearby_entity_distance_meter(
        mut self,
        meter: NearbyEntityDistanceMeter<S>,
    ) -> Self {
        self.nearby_entity_distance_meter = Some(meter);
        self
    }

    pub fn with_construction_entity_order_key(
        mut self,
        order_key: ConstructionEntityOrderKey<S>,
    ) -> Self {
        self.construction_entity_order_key = Some(order_key);
        self
    }

    pub fn with_construction_value_order_key(
        mut self,
        order_key: ConstructionValueOrderKey<S>,
    ) -> Self {
        self.construction_value_order_key = Some(order_key);
        self
    }

    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        entity_class.is_none_or(|name| name == self.entity_type_name)
            && variable_name.is_none_or(|name| name == self.variable_name)
    }

    pub fn current_value(&self, solution: &S, entity_index: usize) -> Option<usize> {
        (self.getter)(solution, entity_index, self.variable_index)
    }

    pub fn set_value(&self, solution: &mut S, entity_index: usize, value: Option<usize>) {
        (self.setter)(solution, entity_index, self.variable_index, value);
    }

    pub fn values_for_entity(&self, solution: &S, entity_index: usize) -> Vec<usize> {
        match self.value_source {
            ValueSource::Empty => Vec::new(),
            ValueSource::CountableRange { from, to } => (from..to).collect(),
            ValueSource::SolutionCount {
                count_fn,
                provider_index,
            } => (0..count_fn(solution, provider_index)).collect(),
            ValueSource::EntitySlice { values_for_entity } => {
                values_for_entity(solution, entity_index, self.variable_index).to_vec()
            }
        }
    }

    pub fn candidate_values_for_entity(
        &self,
        solution: &S,
        entity_index: usize,
        value_candidate_limit: Option<usize>,
    ) -> Vec<usize> {
        if let Some(provider) = self.candidate_values {
            let values = provider(solution, entity_index, self.variable_index);
            return match value_candidate_limit {
                Some(limit) => values.iter().copied().take(limit).collect(),
                None => values.to_vec(),
            };
        }

        match self.value_source {
            ValueSource::Empty => Vec::new(),
            ValueSource::CountableRange { from, to } => {
                let end = value_candidate_limit
                    .map(|limit| from.saturating_add(limit).min(to))
                    .unwrap_or(to);
                (from..end).collect()
            }
            ValueSource::SolutionCount {
                count_fn,
                provider_index,
            } => {
                let count = count_fn(solution, provider_index);
                let end = value_candidate_limit
                    .map(|limit| limit.min(count))
                    .unwrap_or(count);
                (0..end).collect()
            }
            ValueSource::EntitySlice { values_for_entity } => {
                let values = values_for_entity(solution, entity_index, self.variable_index);
                match value_candidate_limit {
                    Some(limit) => values.iter().copied().take(limit).collect(),
                    None => values.to_vec(),
                }
            }
        }
    }

    pub fn has_values_for_entity(&self, solution: &S, entity_index: usize) -> bool {
        match self.value_source {
            ValueSource::Empty => false,
            ValueSource::CountableRange { from, to } => from < to,
            ValueSource::SolutionCount {
                count_fn,
                provider_index,
            } => count_fn(solution, provider_index) > 0,
            ValueSource::EntitySlice { values_for_entity } => {
                !values_for_entity(solution, entity_index, self.variable_index).is_empty()
            }
        }
    }

    pub fn value_is_legal(
        &self,
        solution: &S,
        entity_index: usize,
        candidate: Option<usize>,
    ) -> bool {
        let Some(value) = candidate else {
            return self.allows_unassigned;
        };
        match self.value_source {
            ValueSource::Empty => false,
            ValueSource::CountableRange { from, to } => from <= value && value < to,
            ValueSource::SolutionCount {
                count_fn,
                provider_index,
            } => value < count_fn(solution, provider_index),
            ValueSource::EntitySlice { values_for_entity } => {
                values_for_entity(solution, entity_index, self.variable_index).contains(&value)
            }
        }
    }

    pub fn nearby_value_distance(
        &self,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> Option<f64> {
        self.nearby_value_distance_meter
            .and_then(|meter| meter(solution, entity_index, self.variable_index, value))
    }

    pub fn nearby_entity_distance(
        &self,
        solution: &S,
        left_entity_index: usize,
        right_entity_index: usize,
    ) -> Option<f64> {
        self.nearby_entity_distance_meter.and_then(|meter| {
            meter(
                solution,
                left_entity_index,
                right_entity_index,
                self.variable_index,
            )
        })
    }

    pub fn construction_entity_order_key(&self, solution: &S, entity_index: usize) -> Option<i64> {
        self.construction_entity_order_key
            .and_then(|order_key| order_key(solution, entity_index, self.variable_index))
    }

    pub fn construction_value_order_key(
        &self,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> Option<i64> {
        self.construction_value_order_key
            .and_then(|order_key| order_key(solution, entity_index, self.variable_index, value))
    }
}

impl<S> fmt::Debug for ScalarVariableContext<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarVariableContext")
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("value_source", &self.value_source)
            .field("allows_unassigned", &self.allows_unassigned)
            .field("has_candidate_values", &self.candidate_values.is_some())
            .field(
                "has_nearby_value_candidates",
                &self.nearby_value_candidates.is_some(),
            )
            .field(
                "has_nearby_entity_candidates",
                &self.nearby_entity_candidates.is_some(),
            )
            .field(
                "has_nearby_value_distance_meter",
                &self.nearby_value_distance_meter.is_some(),
            )
            .field(
                "has_nearby_entity_distance_meter",
                &self.nearby_entity_distance_meter.is_some(),
            )
            .field(
                "has_construction_entity_order_key",
                &self.construction_entity_order_key.is_some(),
            )
            .field(
                "has_construction_value_order_key",
                &self.construction_value_order_key.is_some(),
            )
            .finish()
    }
}

#[derive(Debug, Clone)]
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

pub enum VariableContext<S, V, DM, IDM> {
    Scalar(ScalarVariableContext<S>),
    List(ListVariableContext<S, V, DM, IDM>),
}

impl<S, V, DM: Clone, IDM: Clone> Clone for VariableContext<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        match self {
            Self::Scalar(variable) => Self::Scalar(*variable),
            Self::List(variable) => Self::List(variable.clone()),
        }
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for VariableContext<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(variable) => variable.fmt(f),
            Self::List(variable) => variable.fmt(f),
        }
    }
}

pub struct ModelContext<S, V, DM, IDM> {
    variables: Vec<VariableContext<S, V, DM, IDM>>,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, DM: Clone, IDM: Clone> Clone for ModelContext<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S, V, DM, IDM> ModelContext<S, V, DM, IDM> {
    pub fn new(variables: Vec<VariableContext<S, V, DM, IDM>>) -> Self {
        Self {
            variables,
            _phantom: PhantomData,
        }
    }

    pub fn variables(&self) -> &[VariableContext<S, V, DM, IDM>] {
        &self.variables
    }

    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }

    pub fn has_list_variables(&self) -> bool {
        self.variables
            .iter()
            .any(|variable| matches!(variable, VariableContext::List(_)))
    }

    pub fn scalar_variables(&self) -> impl Iterator<Item = &ScalarVariableContext<S>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableContext::Scalar(ctx) => Some(ctx),
            VariableContext::List(_) => None,
        })
    }

    pub fn list_variables(&self) -> impl Iterator<Item = &ListVariableContext<S, V, DM, IDM>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableContext::List(ctx) => Some(ctx),
            VariableContext::Scalar(_) => None,
        })
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for ModelContext<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelContext")
            .field("variables", &self.variables)
            .finish()
    }
}
