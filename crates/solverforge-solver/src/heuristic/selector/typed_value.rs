//! Typed value selectors for high-performance value iteration.
//!
//! Unlike the type-erased `ValueSelector` that yields `Arc<dyn Any>`,
//! typed value selectors yield `V` directly with no heap allocation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

/// A typed value selector that yields values of type `V` directly.
///
/// Unlike `ValueSelector` which returns `Arc<dyn Any>`, this trait
/// returns `V` inline, eliminating heap allocation per value.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The value type
pub trait TypedValueSelector<S, V>: Send + Debug
where
    S: PlanningSolution,
    S::Score: Score,
{
    /// Returns an iterator over typed values for the given entity.
    fn iter_typed<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Box<dyn Iterator<Item = V> + 'a>
    where
        C: ConstraintSet<S, S::Score>;

    /// Returns the number of values.
    fn size<C>(
        &self,
        score_director: &ScoreDirector<S, C>,
        descriptor_index: usize,
        entity_index: usize,
    ) -> usize
    where
        C: ConstraintSet<S, S::Score>;

    /// Returns true if this selector may return the same value multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

/// A typed value selector with a static list of values.
#[derive(Clone)]
pub struct StaticTypedValueSelector<S, V> {
    values: Vec<V>,
    _phantom: PhantomData<S>,
}

impl<S, V: Debug> Debug for StaticTypedValueSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticTypedValueSelector")
            .field("values", &self.values)
            .finish()
    }
}

impl<S, V: Clone> StaticTypedValueSelector<S, V> {
    /// Creates a new static value selector with the given values.
    pub fn new(values: Vec<V>) -> Self {
        Self {
            values,
            _phantom: PhantomData,
        }
    }

    /// Returns the values.
    pub fn values(&self) -> &[V] {
        &self.values
    }
}

impl<S, V> TypedValueSelector<S, V> for StaticTypedValueSelector<S, V>
where
    S: PlanningSolution,
    S::Score: Score,
    V: Clone + Send + Debug + 'static,
{
    fn iter_typed<'a, C>(
        &'a self,
        _score_director: &'a ScoreDirector<S, C>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> Box<dyn Iterator<Item = V> + 'a>
    where
        C: ConstraintSet<S, S::Score>,
    {
        Box::new(self.values.iter().cloned())
    }

    fn size<C>(
        &self,
        _score_director: &ScoreDirector<S, C>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        self.values.len()
    }
}

/// A typed value selector that extracts values from the solution using a function pointer.
pub struct FromSolutionTypedValueSelector<S, V> {
    extractor: fn(&S) -> Vec<V>,
    _phantom: PhantomData<(S, V)>,
}

impl<S, V> Debug for FromSolutionTypedValueSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FromSolutionTypedValueSelector").finish()
    }
}

impl<S, V> FromSolutionTypedValueSelector<S, V> {
    /// Creates a new selector with the given extractor function pointer.
    pub fn new(extractor: fn(&S) -> Vec<V>) -> Self {
        Self {
            extractor,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> TypedValueSelector<S, V> for FromSolutionTypedValueSelector<S, V>
where
    S: PlanningSolution,
    S::Score: Score,
    V: Clone + Send + Debug + 'static,
{
    fn iter_typed<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> Box<dyn Iterator<Item = V> + 'a>
    where
        C: ConstraintSet<S, S::Score>,
    {
        let values = (self.extractor)(score_director.working_solution());
        Box::new(values.into_iter())
    }

    fn size<C>(
        &self,
        score_director: &ScoreDirector<S, C>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        (self.extractor)(score_director.working_solution()).len()
    }
}

/// A typed value selector that generates a range of usize values 0..count.
///
/// Uses a function pointer to get the count from the solution.
pub struct RangeValueSelector<S> {
    count_fn: fn(&S) -> usize,
    _phantom: PhantomData<S>,
}

impl<S> Debug for RangeValueSelector<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RangeValueSelector").finish()
    }
}

impl<S> RangeValueSelector<S> {
    /// Creates a new range value selector with the given count function.
    pub fn new(count_fn: fn(&S) -> usize) -> Self {
        Self {
            count_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S> TypedValueSelector<S, usize> for RangeValueSelector<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    fn iter_typed<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> Box<dyn Iterator<Item = usize> + 'a>
    where
        C: ConstraintSet<S, S::Score>,
    {
        let count = (self.count_fn)(score_director.working_solution());
        Box::new(0..count)
    }

    fn size<C>(
        &self,
        score_director: &ScoreDirector<S, C>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        (self.count_fn)(score_director.working_solution())
    }
}
