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
pub trait ValueSelector<S: PlanningSolution, V>: Send + Debug {
    // Returns an iterator over values for the given entity.
    fn iter_typed<'a, D: Director<S>>(
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

    // Returns true if this selector may return the same value multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

/// A value selector with a static list of values.
pub struct StaticValueSelector<S, V> {
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
    pub fn new(values: Vec<V>) -> Self {
        Self {
            values,
            _phantom: PhantomData,
        }
    }

    pub fn values(&self) -> &[V] {
        &self.values
    }
}

impl<S, V> ValueSelector<S, V> for StaticValueSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Debug + 'static,
{
    fn iter_typed<'a, D: Director<S>>(
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

/// A value selector that extracts values from the solution using a function pointer.
pub struct FromSolutionValueSelector<S, V> {
    extractor: fn(&S) -> Vec<V>,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V> Debug for FromSolutionValueSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FromSolutionValueSelector").finish()
    }
}

impl<S, V> FromSolutionValueSelector<S, V> {
    pub fn new(extractor: fn(&S) -> Vec<V>) -> Self {
        Self {
            extractor,
            _phantom: PhantomData,
        }
    }
}

pub struct PerEntityValueSelector<S, V> {
    extractor: fn(&S, usize) -> Vec<V>,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V> Debug for PerEntityValueSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerEntityValueSelector").finish()
    }
}

impl<S, V> PerEntityValueSelector<S, V> {
    pub fn new(extractor: fn(&S, usize) -> Vec<V>) -> Self {
        Self {
            extractor,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> ValueSelector<S, V> for PerEntityValueSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Debug + 'static,
{
    fn iter_typed<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        _descriptor_index: usize,
        entity_index: usize,
    ) -> impl Iterator<Item = V> + 'a {
        (self.extractor)(score_director.working_solution(), entity_index).into_iter()
    }

    fn size<D: Director<S>>(
        &self,
        score_director: &D,
        _descriptor_index: usize,
        entity_index: usize,
    ) -> usize {
        (self.extractor)(score_director.working_solution(), entity_index).len()
    }
}

pub struct PerEntitySliceValueSelector<S, V> {
    extractor: for<'a> fn(&'a S, usize) -> &'a [V],
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V> Debug for PerEntitySliceValueSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerEntitySliceValueSelector").finish()
    }
}

impl<S, V> PerEntitySliceValueSelector<S, V> {
    pub fn new(extractor: for<'a> fn(&'a S, usize) -> &'a [V]) -> Self {
        Self {
            extractor,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> ValueSelector<S, V> for PerEntitySliceValueSelector<S, V>
where
    S: PlanningSolution,
    V: Copy + Send + Debug + 'static,
{
    fn iter_typed<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        _descriptor_index: usize,
        entity_index: usize,
    ) -> impl Iterator<Item = V> + 'a {
        (self.extractor)(score_director.working_solution(), entity_index)
            .to_vec()
            .into_iter()
    }

    fn size<D: Director<S>>(
        &self,
        score_director: &D,
        _descriptor_index: usize,
        entity_index: usize,
    ) -> usize {
        (self.extractor)(score_director.working_solution(), entity_index).len()
    }
}

impl<S, V> ValueSelector<S, V> for FromSolutionValueSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Debug + 'static,
{
    fn iter_typed<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> impl Iterator<Item = V> + 'a {
        let values = (self.extractor)(score_director.working_solution());
        values.into_iter()
    }

    fn size<D: Director<S>>(
        &self,
        score_director: &D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize {
        (self.extractor)(score_director.working_solution()).len()
    }
}

/// A value selector that generates a range of usize values 0..count.
///
/// Uses a function pointer to get the count from the solution.
pub struct RangeValueSelector<S> {
    count_fn: fn(&S) -> usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for RangeValueSelector<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RangeValueSelector").finish()
    }
}

impl<S> RangeValueSelector<S> {
    pub fn new(count_fn: fn(&S) -> usize) -> Self {
        Self {
            count_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S> ValueSelector<S, usize> for RangeValueSelector<S>
where
    S: PlanningSolution,
{
    fn iter_typed<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        let count = (self.count_fn)(score_director.working_solution());
        0..count
    }

    fn size<D: Director<S>>(
        &self,
        score_director: &D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize {
        (self.count_fn)(score_director.working_solution())
    }
}

#[cfg(test)]
#[path = "value_selector_tests.rs"]
mod tests;
