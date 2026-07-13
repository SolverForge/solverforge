//! Narrow physical access traits for shared list move primitives.
//!
//! They intentionally expose only the operations a move family consumes. A
//! direct public selector never receives a fabricated callback for an
//! unrelated list operation, while RuntimeListSlot participates through its
//! complete ListAccess implementation.

use std::fmt;

use solverforge_core::domain::DynamicListVariableSlot;

use crate::builder::context::{
    list_access::{ListAccess, ListAccessError},
    RuntimeListSlot,
};
use crate::builder::ListVariableSlot;
use crate::heuristic::r#move::metadata::{
    encode_option_debug, encode_runtime_dynamic_list_source, NONE_ID,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

pub(crate) trait ListMoveAccess<S> {
    type Element: Clone + PartialEq + Send + Sync + fmt::Debug + 'static;

    fn descriptor_index(&self) -> usize;
    fn variable_name(&self) -> &'static str;
    fn list_len(&self, solution: &S, entity: usize) -> usize;
    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element>;

    /// Canonical value identity for tabu metadata.
    ///
    /// Static facades retain their historical debug-derived identity. The
    /// compiled runtime carrier overrides this for dynamic values so the
    /// `RuntimeListElement` wrapper never leaks into public search behavior.
    fn tabu_value_id(&self, _solution: &S, value: Option<&Self::Element>) -> u64 {
        encode_option_debug(value)
    }
}

pub(crate) trait ListChangeAccess<S>: ListMoveAccess<S> {
    fn list_remove(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
    ) -> Option<Self::Element>;
    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element);
}

pub(crate) trait ListSwapAccess<S>: ListMoveAccess<S> {
    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError>;
}

/// One source-aware identity boundary for the tagged runtime list carrier.
///
/// The static branch retains the historical value-debug identity. The dynamic
/// branch derives its token from `ListAccess::element_source_key`, so it
/// follows the bound declaration's logical source stream rather than the
/// internal `RuntimeListElement` debug wrapper.
pub(crate) fn runtime_list_tabu_value_id<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    solution: &S,
    value: Option<&crate::builder::context::RuntimeListElement<V>>,
) -> u64
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let Some(value) = value else {
        return NONE_ID;
    };
    match (slot, value) {
        (
            RuntimeListSlot::Static { .. },
            crate::builder::context::RuntimeListElement::Static(value),
        ) => encode_option_debug(Some(value)),
        (RuntimeListSlot::Dynamic(_), crate::builder::context::RuntimeListElement::Dynamic(_)) => {
            encode_runtime_dynamic_list_source(ListAccess::element_source_key(
                slot, solution, value,
            ))
        }
        _ => panic!("runtime list tabu value does not belong to its selected list slot"),
    }
}

impl<S, V, DM, IDM> ListMoveAccess<S> for ListVariableSlot<S, V, DM, IDM>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Element = V;

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn variable_name(&self) -> &'static str {
        ListAccess::variable_name(self)
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        ListAccess::list_len(self, solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        ListAccess::list_get(self, solution, entity, position)
    }
}

impl<S, V, DM, IDM> ListChangeAccess<S> for ListVariableSlot<S, V, DM, IDM>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn list_remove(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
    ) -> Option<Self::Element> {
        ListAccess::list_remove(self, solution, entity, position)
    }

    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element) {
        ListAccess::list_insert(self, solution, entity, position, value);
    }
}

impl<S, V, DM, IDM> ListSwapAccess<S> for ListVariableSlot<S, V, DM, IDM>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError> {
        ListAccess::list_set(self, solution, entity, position, value)
    }
}

impl<S> ListMoveAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    type Element = usize;

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn variable_name(&self) -> &'static str {
        ListAccess::variable_name(self)
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        ListAccess::list_len(self, solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        ListAccess::list_get(self, solution, entity, position)
    }
}

impl<S> ListChangeAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn list_remove(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
    ) -> Option<Self::Element> {
        ListAccess::list_remove(self, solution, entity, position)
    }

    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element) {
        ListAccess::list_insert(self, solution, entity, position, value);
    }
}

impl<S> ListSwapAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError> {
        ListAccess::list_set(self, solution, entity, position, value)
    }
}

impl<S, V, DM, IDM> ListMoveAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Element = crate::builder::context::RuntimeListElement<V>;

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn variable_name(&self) -> &'static str {
        ListAccess::variable_name(self)
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        ListAccess::list_len(self, solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        ListAccess::list_get(self, solution, entity, position)
    }

    fn tabu_value_id(&self, solution: &S, value: Option<&Self::Element>) -> u64 {
        runtime_list_tabu_value_id(self, solution, value)
    }
}

impl<S, V, DM, IDM> ListChangeAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn list_remove(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
    ) -> Option<Self::Element> {
        ListAccess::list_remove(self, solution, entity, position)
    }

    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element) {
        ListAccess::list_insert(self, solution, entity, position, value);
    }
}

impl<S, V, DM, IDM> ListSwapAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError> {
        ListAccess::list_set(self, solution, entity, position, value)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct StaticListChangeAccess<S, V> {
    pub(crate) list_len: fn(&S, usize) -> usize,
    pub(crate) list_get: fn(&S, usize, usize) -> Option<V>,
    pub(crate) list_remove: fn(&mut S, usize, usize) -> Option<V>,
    pub(crate) list_insert: fn(&mut S, usize, usize, V),
    pub(crate) variable_name: &'static str,
    pub(crate) descriptor_index: usize,
}

impl<S, V> fmt::Debug for StaticListChangeAccess<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticListChangeAccess")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> ListMoveAccess<S> for StaticListChangeAccess<S, V>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    type Element = V;

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn variable_name(&self) -> &'static str {
        self.variable_name
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        (self.list_len)(solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        (self.list_get)(solution, entity, position)
    }
}

impl<S, V> ListChangeAccess<S> for StaticListChangeAccess<S, V>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    fn list_remove(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
    ) -> Option<Self::Element> {
        (self.list_remove)(solution, entity, position)
    }

    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element) {
        (self.list_insert)(solution, entity, position, value);
    }
}

pub(crate) struct StaticListSwapAccess<S, V> {
    pub(crate) list_len: fn(&S, usize) -> usize,
    pub(crate) list_get: fn(&S, usize, usize) -> Option<V>,
    pub(crate) list_set: fn(&mut S, usize, usize, V),
    pub(crate) variable_name: &'static str,
    pub(crate) descriptor_index: usize,
}

impl<S, V> Copy for StaticListSwapAccess<S, V> {}

impl<S, V> Clone for StaticListSwapAccess<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> fmt::Debug for StaticListSwapAccess<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticListSwapAccess")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> ListMoveAccess<S> for StaticListSwapAccess<S, V>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    type Element = V;

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn variable_name(&self) -> &'static str {
        self.variable_name
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        (self.list_len)(solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        (self.list_get)(solution, entity, position)
    }
}

impl<S, V> ListSwapAccess<S> for StaticListSwapAccess<S, V>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError> {
        (self.list_set)(solution, entity, position, value);
        Ok(())
    }
}
