use std::fmt;

use super::value_source::ValueSource;

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
