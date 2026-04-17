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
        count_fn: fn(&S) -> usize,
    },
    EntitySlice {
        values_for_entity: for<'a> fn(&'a S, usize) -> &'a [usize],
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
            Self::SolutionCount { .. } => write!(f, "ValueSource::SolutionCount(..)"),
            Self::EntitySlice { .. } => write!(f, "ValueSource::EntitySlice(..)"),
        }
    }
}

pub struct ScalarVariableContext<S> {
    pub descriptor_index: usize,
    pub entity_type_name: &'static str,
    pub entity_count: fn(&S) -> usize,
    pub variable_name: &'static str,
    pub getter: fn(&S, usize) -> Option<usize>,
    pub setter: fn(&mut S, usize, Option<usize>),
    pub value_source: ValueSource<S>,
    pub allows_unassigned: bool,
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
        entity_type_name: &'static str,
        entity_count: fn(&S) -> usize,
        variable_name: &'static str,
        getter: fn(&S, usize) -> Option<usize>,
        setter: fn(&mut S, usize, Option<usize>),
        value_source: ValueSource<S>,
        allows_unassigned: bool,
    ) -> Self {
        Self {
            descriptor_index,
            entity_type_name,
            entity_count,
            variable_name,
            getter,
            setter,
            value_source,
            allows_unassigned,
        }
    }

    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        entity_class.is_none_or(|name| name == self.entity_type_name)
            && variable_name.is_none_or(|name| name == self.variable_name)
    }
}

impl<S> fmt::Debug for ScalarVariableContext<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarVariableContext")
            .field("descriptor_index", &self.descriptor_index)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("value_source", &self.value_source)
            .field("allows_unassigned", &self.allows_unassigned)
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
    pub list_len: fn(&S, usize) -> usize,
    pub list_remove: fn(&mut S, usize, usize) -> Option<V>,
    pub list_insert: fn(&mut S, usize, usize, V),
    pub list_get: fn(&S, usize, usize) -> Option<V>,
    pub list_set: fn(&mut S, usize, usize, V),
    pub list_reverse: fn(&mut S, usize, usize, usize),
    pub sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    pub sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    pub ruin_remove: fn(&mut S, usize, usize) -> V,
    pub ruin_insert: fn(&mut S, usize, usize, V),
    pub entity_count: fn(&S) -> usize,
    pub cross_distance_meter: DM,
    pub intra_distance_meter: IDM,
    pub variable_name: &'static str,
    pub descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, DM: Clone, IDM: Clone> Clone for ListVariableContext<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        Self {
            entity_type_name: self.entity_type_name,
            list_len: self.list_len,
            list_remove: self.list_remove,
            list_insert: self.list_insert,
            list_get: self.list_get,
            list_set: self.list_set,
            list_reverse: self.list_reverse,
            sublist_remove: self.sublist_remove,
            sublist_insert: self.sublist_insert,
            ruin_remove: self.ruin_remove,
            ruin_insert: self.ruin_insert,
            entity_count: self.entity_count,
            cross_distance_meter: self.cross_distance_meter.clone(),
            intra_distance_meter: self.intra_distance_meter.clone(),
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, DM, IDM> ListVariableContext<S, V, DM, IDM> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_type_name: &'static str,
        list_len: fn(&S, usize) -> usize,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        list_reverse: fn(&mut S, usize, usize, usize),
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        ruin_remove: fn(&mut S, usize, usize) -> V,
        ruin_insert: fn(&mut S, usize, usize, V),
        entity_count: fn(&S) -> usize,
        cross_distance_meter: DM,
        intra_distance_meter: IDM,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_type_name,
            list_len,
            list_remove,
            list_insert,
            list_get,
            list_set,
            list_reverse,
            sublist_remove,
            sublist_insert,
            ruin_remove,
            ruin_insert,
            entity_count,
            cross_distance_meter,
            intra_distance_meter,
            variable_name,
            descriptor_index,
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
