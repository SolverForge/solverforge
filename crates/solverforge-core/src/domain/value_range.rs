//! Value range providers for planning variables.
//!
//! Value range providers define the possible values that can be assigned to
//! planning variables. They can be static (fixed list) or dynamic (computed
//! from the solution state).

/// Provides values for a planning variable.
///
/// This trait is implemented for types that can produce a list of valid values
/// for a planning variable. The values can be static or computed dynamically
/// based on the solution state.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `V` - The value type (must match the planning variable's type)
///
/// # Example
///
/// ```
/// use solverforge_core::domain::ValueRangeProvider;
///
/// // Define a solution with a size field
/// struct NQueensSolution {
///     n: i32,
/// }
///
/// // Implement a value range provider that computes row values
/// struct RowRangeProvider;
///
/// impl ValueRangeProvider<NQueensSolution, i32> for RowRangeProvider {
///     fn get_values(&self, solution: &NQueensSolution) -> Vec<i32> {
///         (0..solution.n).collect()
///     }
/// }
///
/// let solution = NQueensSolution { n: 8 };
/// let provider = RowRangeProvider;
/// assert_eq!(provider.get_values(&solution), vec![0, 1, 2, 3, 4, 5, 6, 7]);
/// assert_eq!(provider.value_count(&solution), 8);
/// ```
pub trait ValueRangeProvider<S, V>: Send + Sync {
    /// Returns all possible values for the variable.
    ///
    /// This method is called during move generation to determine which
    /// values can be assigned to a planning variable.
    fn get_values(&self, solution: &S) -> Vec<V>;

    /// Returns the number of possible values.
    ///
    /// The default implementation calls `get_values` and returns the length,
    /// but implementations may override this for efficiency if the count
    /// can be computed without materializing the values.
    fn value_count(&self, solution: &S) -> usize {
        self.get_values(solution).len()
    }

    /// Returns whether the value range is empty.
    fn is_empty(&self, solution: &S) -> bool {
        self.value_count(solution) == 0
    }
}

/// A value range provider backed by a field in the solution.
///
/// This is the most common case: a `Vec<V>` field that contains the possible values.
pub struct FieldValueRangeProvider<S, V, F>
where
    F: Fn(&S) -> &Vec<V> + Send + Sync,
{
    getter: F,
    _marker: std::marker::PhantomData<(S, V)>,
}

impl<S, V, F> FieldValueRangeProvider<S, V, F>
where
    F: Fn(&S) -> &Vec<V> + Send + Sync,
{
    /// Creates a new field-based value range provider.
    pub fn new(getter: F) -> Self {
        Self {
            getter,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, V, F> ValueRangeProvider<S, V> for FieldValueRangeProvider<S, V, F>
where
    S: Send + Sync,
    V: Clone + Send + Sync,
    F: Fn(&S) -> &Vec<V> + Send + Sync,
{
    fn get_values(&self, solution: &S) -> Vec<V> {
        (self.getter)(solution).clone()
    }

    fn value_count(&self, solution: &S) -> usize {
        (self.getter)(solution).len()
    }
}

/// A value range provider that computes values dynamically.
///
/// Use this when values are computed from solution state rather than
/// stored in a field.
pub struct ComputedValueRangeProvider<S, V, F>
where
    F: Fn(&S) -> Vec<V> + Send + Sync,
{
    compute: F,
    _marker: std::marker::PhantomData<(S, V)>,
}

impl<S, V, F> ComputedValueRangeProvider<S, V, F>
where
    F: Fn(&S) -> Vec<V> + Send + Sync,
{
    /// Creates a new computed value range provider.
    pub fn new(compute: F) -> Self {
        Self {
            compute,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns [`ValueRangeType::EntityDependent`] since computed ranges
    /// derive their values from solution state at runtime.
    pub fn value_range_type() -> ValueRangeType {
        ValueRangeType::EntityDependent
    }
}

impl<S, V, F> ValueRangeProvider<S, V> for ComputedValueRangeProvider<S, V, F>
where
    S: Send + Sync,
    V: Send + Sync,
    F: Fn(&S) -> Vec<V> + Send + Sync,
{
    fn get_values(&self, solution: &S) -> Vec<V> {
        (self.compute)(solution)
    }
}

/// A static value range with a fixed set of values.
///
/// Use this when the possible values don't depend on solution state.
pub struct StaticValueRange<V> {
    values: Vec<V>,
}

impl<V> StaticValueRange<V> {
    /// Creates a new static value range.
    pub fn new(values: Vec<V>) -> Self {
        Self { values }
    }
}

impl<S, V> ValueRangeProvider<S, V> for StaticValueRange<V>
where
    S: Send + Sync,
    V: Clone + Send + Sync,
{
    fn get_values(&self, _solution: &S) -> Vec<V> {
        self.values.clone()
    }

    fn value_count(&self, _solution: &S) -> usize {
        self.values.len()
    }
}

/// An integer range value provider.
///
/// Efficiently provides a contiguous range of integers without storing them.
pub struct IntegerRange {
    start: i64,
    end: i64,
}

use super::variable::ValueRangeType;

impl IntegerRange {
    /// Creates a new integer range [start, end).
    pub fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }

    /// Creates a range from 0 to n (exclusive).
    pub fn from_zero(n: i64) -> Self {
        Self::new(0, n)
    }

    /// Returns the [`ValueRangeType`] describing this range.
    ///
    /// An `IntegerRange` is a countable range with known bounds.
    pub fn value_range_type(&self) -> ValueRangeType {
        ValueRangeType::CountableRange {
            from: self.start,
            to: self.end,
        }
    }
}

impl<S> ValueRangeProvider<S, i64> for IntegerRange
where
    S: Send + Sync,
{
    fn get_values(&self, _solution: &S) -> Vec<i64> {
        (self.start..self.end).collect()
    }

    fn value_count(&self, _solution: &S) -> usize {
        let count = (self.end - self.start).max(0);
        usize::try_from(count).expect("IntegerRange count overflows usize")
    }
}

impl<S> ValueRangeProvider<S, i32> for IntegerRange
where
    S: Send + Sync,
{
    fn get_values(&self, _solution: &S) -> Vec<i32> {
        let start_i32 =
            i32::try_from(self.start).expect("IntegerRange start overflows i32 for i32 provider");
        let end_i32 =
            i32::try_from(self.end).expect("IntegerRange end overflows i32 for i32 provider");
        (start_i32..end_i32).collect()
    }

    fn value_count(&self, _solution: &S) -> usize {
        let count = (self.end - self.start).max(0);
        usize::try_from(count).expect("IntegerRange count overflows usize")
    }
}
