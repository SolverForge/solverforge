//! Function-pointer adapters for public static range moves.

use std::fmt;

use crate::builder::context::list_access::ListAccessError;

use super::{ListRangeAccess, ListReverseAccess, ListWindowAccess};

#[derive(Clone, Copy)]
pub(crate) struct StaticListReverseAccess<S, V> {
    pub(crate) list_len: fn(&S, usize) -> usize,
    pub(crate) list_get: fn(&S, usize, usize) -> Option<V>,
    pub(crate) list_reverse: fn(&mut S, usize, usize, usize),
    pub(crate) variable_name: &'static str,
    pub(crate) descriptor_index: usize,
}

impl<S, V> fmt::Debug for StaticListReverseAccess<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticListReverseAccess")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> ListRangeAccess<S> for StaticListReverseAccess<S, V>
where
    V: Clone + Send + Sync + fmt::Debug + 'static,
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

impl<S, V> ListReverseAccess<S> for StaticListReverseAccess<S, V>
where
    V: Clone + Send + Sync + fmt::Debug + 'static,
{
    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError> {
        (self.list_reverse)(solution, entity, start, end);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) struct StaticListWindowAccess<S, V> {
    pub(crate) list_len: fn(&S, usize) -> usize,
    pub(crate) list_get: fn(&S, usize, usize) -> Option<V>,
    pub(crate) sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    pub(crate) sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    pub(crate) variable_name: &'static str,
    pub(crate) descriptor_index: usize,
}

impl<S, V> fmt::Debug for StaticListWindowAccess<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticListWindowAccess")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> ListRangeAccess<S> for StaticListWindowAccess<S, V>
where
    V: Clone + Send + Sync + fmt::Debug + 'static,
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

impl<S, V> ListWindowAccess<S> for StaticListWindowAccess<S, V>
where
    V: Clone + Send + Sync + fmt::Debug + 'static,
{
    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError> {
        Ok((self.sublist_remove)(solution, entity, start, end))
    }

    fn sublist_insert(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        values: Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        (self.sublist_insert)(solution, entity, position, values);
        Ok(())
    }
}
