use std::fmt;

/// Where a scalar variable's legal values come from. Values are indexes.
pub enum ValueSource<S> {
    /// No legal values.
    Empty,
    /// The values `from..to`.
    CountableRange {
        /// First value (inclusive).
        from: usize,
        /// End of the range (exclusive).
        to: usize,
    },
    /// The values `0..count_fn(solution, provider_index)`.
    SolutionCount {
        /// Returns the number of values in the provider collection.
        count_fn: fn(&S, usize) -> usize,
        /// Index of the provider collection passed to `count_fn`.
        provider_index: usize,
    },
    /// A per-entity slice of legal values.
    EntitySlice {
        /// Returns the legal values for `(entity_index, variable_index)`.
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
