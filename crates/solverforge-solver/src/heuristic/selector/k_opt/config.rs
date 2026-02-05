//! K-opt configuration.

/// Configuration for k-opt move generation.
#[derive(Debug, Clone)]
pub struct KOptConfig {
    /// The k value (2-5).
    pub k: usize,
    /// Minimum segment length between cuts (default: 1).
    pub min_segment_len: usize,
    /// Whether to use only a subset of reconnection patterns.
    pub limited_patterns: bool,
}

impl KOptConfig {
    /// Creates a new k-opt configuration.
    ///
    /// # Panics
    ///
    /// Panics if k < 2 or k > 5.
    pub fn new(k: usize) -> Self {
        assert!((2..=5).contains(&k), "k must be between 2 and 5");
        Self {
            k,
            min_segment_len: 1,
            limited_patterns: false,
        }
    }

    /// Sets minimum segment length between cuts.
    pub fn with_min_segment_len(mut self, len: usize) -> Self {
        self.min_segment_len = len;
        self
    }

    /// Enables limited pattern mode (faster but less thorough).
    pub fn with_limited_patterns(mut self, limited: bool) -> Self {
        self.limited_patterns = limited;
        self
    }
}
