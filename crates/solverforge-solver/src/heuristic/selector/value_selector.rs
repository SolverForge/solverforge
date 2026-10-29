/* Value selectors for high-performance value iteration.

Unlike the type-erased `ValueSelector` that yields `Arc<dyn Any>`,
these selectors yield `V` directly with no heap allocation.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

/// A value selector that yields values of type `V` directly.
///
/// Unlike `ValueSelector` which returns `Arc<dyn Any>`, this trait
/// returns `V` inline, eliminating heap allocation per value.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The value type
pub(crate) trait ValueSelector<S: PlanningSolution, V>: Send + Debug {
    // Returns an iterator over values for the given entity.
    fn iter<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        descriptor_index: usize,
        entity_index: usize,
    ) -> impl Iterator<Item = V> + 'a;

    fn size<D: Director<S>>(
        &self,
        score_director: &D,
        descriptor_index: usize,
        entity_index: usize,
    ) -> usize;
}

/// A value selector with a static list of values.
pub(crate) struct StaticValueSelector<S, V> {
    values: Vec<V>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, V: Clone> Clone for StaticValueSelector<S, V> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S, V: Debug> Debug for StaticValueSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticValueSelector")
            .field("values", &self.values)
            .finish()
    }
}

impl<S, V: Clone> StaticValueSelector<S, V> {
    pub(crate) fn new(values: Vec<V>) -> Self {
        Self {
            values,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> ValueSelector<S, V> for StaticValueSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Debug + 'static,
{
    fn iter<'a, D: Director<S>>(
        &'a self,
        _score_director: &D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> impl Iterator<Item = V> + 'a {
        self.values.iter().cloned()
    }

    fn size<D: Director<S>>(
        &self,
        _score_director: &D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize {
        self.values.len()
    }
}

#[cfg(test)]
#[path = "value_selector_tests.rs"]
mod tests;
