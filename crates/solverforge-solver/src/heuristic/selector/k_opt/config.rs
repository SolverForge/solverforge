// Configuration for k-opt move generation.

// Configuration for k-opt move generation.
#[derive(Debug, Clone)]
pub struct KOptConfig {
    // The k value (2-5).
    pub k: usize,
    // Minimum segment length between cuts (default: 1).
    pub min_segment_len: usize,
    // Whether to use only a subset of reconnection patterns.
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
}
