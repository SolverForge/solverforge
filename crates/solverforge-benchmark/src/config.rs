//! Benchmark configuration.

/// Configuration for a benchmark run.
///
/// Controls warmup iterations, measurement runs, and optional output paths.
///
/// # Example
///
/// ```
/// use solverforge_benchmark::BenchmarkConfig;
///
/// let config = BenchmarkConfig::new("My Benchmark")
///     .with_warmup_count(3)
///     .with_run_count(10);
///
/// assert_eq!(config.name(), "My Benchmark");
/// assert_eq!(config.warmup_count(), 3);
/// assert_eq!(config.run_count(), 10);
/// ```
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    name: String,
    warmup_count: usize,
    run_count: usize,
    csv_output_path: Option<String>,
    markdown_output_path: Option<String>,
}

impl BenchmarkConfig {
    /// Creates a new benchmark configuration with the given name.
    ///
    /// Defaults:
    /// - warmup_count: 1
    /// - run_count: 3
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::BenchmarkConfig;
    ///
    /// let config = BenchmarkConfig::new("Test Benchmark");
    /// assert_eq!(config.warmup_count(), 1);
    /// assert_eq!(config.run_count(), 3);
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            warmup_count: 1,
            run_count: 3,
            csv_output_path: None,
            markdown_output_path: None,
        }
    }

    /// Sets the number of warmup iterations (not measured).
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::BenchmarkConfig;
    ///
    /// let config = BenchmarkConfig::new("Test").with_warmup_count(5);
    /// assert_eq!(config.warmup_count(), 5);
    /// ```
    pub fn with_warmup_count(mut self, count: usize) -> Self {
        self.warmup_count = count;
        self
    }

    /// Sets the number of measurement runs.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::BenchmarkConfig;
    ///
    /// let config = BenchmarkConfig::new("Test").with_run_count(10);
    /// assert_eq!(config.run_count(), 10);
    /// ```
    pub fn with_run_count(mut self, count: usize) -> Self {
        self.run_count = count;
        self
    }

    /// Sets the output path for CSV export.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::BenchmarkConfig;
    ///
    /// let config = BenchmarkConfig::new("Test")
    ///     .with_csv_output("results.csv");
    /// assert_eq!(config.csv_output_path(), Some("results.csv"));
    /// ```
    pub fn with_csv_output(mut self, path: impl Into<String>) -> Self {
        self.csv_output_path = Some(path.into());
        self
    }

    /// Sets the output path for Markdown report.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::BenchmarkConfig;
    ///
    /// let config = BenchmarkConfig::new("Test")
    ///     .with_markdown_output("report.md");
    /// assert_eq!(config.markdown_output_path(), Some("report.md"));
    /// ```
    pub fn with_markdown_output(mut self, path: impl Into<String>) -> Self {
        self.markdown_output_path = Some(path.into());
        self
    }

    /// Returns the benchmark name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the number of warmup iterations.
    pub fn warmup_count(&self) -> usize {
        self.warmup_count
    }

    /// Returns the number of measurement runs.
    pub fn run_count(&self) -> usize {
        self.run_count
    }

    /// Returns the CSV output path, if set.
    pub fn csv_output_path(&self) -> Option<&str> {
        self.csv_output_path.as_deref()
    }

    /// Returns the Markdown output path, if set.
    pub fn markdown_output_path(&self) -> Option<&str> {
        self.markdown_output_path.as_deref()
    }
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self::new("Benchmark")
    }
}
