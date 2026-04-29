use std::fmt;

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
