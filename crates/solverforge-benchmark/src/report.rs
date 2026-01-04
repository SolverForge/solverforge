//! Report generation for benchmark results.

use std::fmt::{Display, Write as _};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use solverforge_core::score::Score;

use crate::result::BenchmarkResult;

/// CSV exporter for benchmark results.
///
/// Exports benchmark results to CSV format with columns for run index,
/// solve time, final score, moves per second, and acceptance rate.
///
/// # Example
///
/// ```
/// use solverforge_benchmark::{BenchmarkResult, CsvExporter};
/// use solverforge_core::score::SimpleScore;
///
/// let result = BenchmarkResult::<SimpleScore>::new("Test", "HC", "Problem");
/// let csv = CsvExporter::to_string(&result);
/// assert!(csv.contains("run_index,solve_time_ms"));
/// ```
pub struct CsvExporter;

impl CsvExporter {
    /// Exports benchmark result to CSV string.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::{BenchmarkResult, BenchmarkRun, CsvExporter};
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// let mut result = BenchmarkResult::<SimpleScore>::new("Test", "HC", "Problem");
    /// result.add_run(BenchmarkRun {
    ///     run_index: 0,
    ///     solve_time: Duration::from_millis(100),
    ///     final_score: SimpleScore::of(0),
    ///     score_history: vec![],
    ///     moves_evaluated: 1000,
    ///     moves_accepted: 500,
    ///     score_calculations: 1000,
    /// });
    ///
    /// let csv = CsvExporter::to_string(&result);
    /// assert!(csv.contains("0,100,"));
    /// ```
    pub fn to_string<Sc: Score + Display>(result: &BenchmarkResult<Sc>) -> String {
        let mut output = String::new();

        // Header
        writeln!(
            output,
            "run_index,solve_time_ms,final_score,moves_evaluated,moves_accepted,moves_per_second,acceptance_rate"
        )
        .unwrap();

        // Data rows
        for run in &result.runs {
            writeln!(
                output,
                "{},{},{},{},{},{:.2},{:.4}",
                run.run_index,
                run.solve_time.as_millis(),
                run.final_score,
                run.moves_evaluated,
                run.moves_accepted,
                run.moves_per_second(),
                run.acceptance_rate(),
            )
            .unwrap();
        }

        output
    }

    /// Exports benchmark result to a CSV file.
    pub fn to_file<Sc: Score + Display>(
        result: &BenchmarkResult<Sc>,
        path: impl AsRef<Path>,
    ) -> io::Result<()> {
        let csv = Self::to_string(result);
        fs::write(path, csv)
    }

    /// Writes benchmark result as CSV to a writer.
    pub fn write<Sc: Score + Display, W: Write>(
        result: &BenchmarkResult<Sc>,
        mut writer: W,
    ) -> io::Result<()> {
        let csv = Self::to_string(result);
        writer.write_all(csv.as_bytes())
    }
}

/// Markdown report generator.
///
/// Generates human-readable Markdown reports from benchmark results,
/// including summary statistics and a table of individual runs.
///
/// # Example
///
/// ```
/// use solverforge_benchmark::{BenchmarkResult, MarkdownReport};
/// use solverforge_core::score::SimpleScore;
///
/// let result = BenchmarkResult::<SimpleScore>::new("Test", "HC", "Problem");
/// let md = MarkdownReport::to_string(&result);
/// assert!(md.contains("# Benchmark: Test"));
/// ```
pub struct MarkdownReport;

impl MarkdownReport {
    /// Generates a Markdown report string.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::{BenchmarkResult, BenchmarkRun, MarkdownReport};
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// let mut result = BenchmarkResult::<SimpleScore>::new("Test", "HC", "Problem");
    /// result.add_run(BenchmarkRun {
    ///     run_index: 0,
    ///     solve_time: Duration::from_millis(100),
    ///     final_score: SimpleScore::of(0),
    ///     score_history: vec![],
    ///     moves_evaluated: 1000,
    ///     moves_accepted: 500,
    ///     score_calculations: 1000,
    /// });
    ///
    /// let md = MarkdownReport::to_string(&result);
    /// assert!(md.contains("## Summary"));
    /// assert!(md.contains("| Run | Time (ms) |"));
    /// ```
    pub fn to_string<Sc: Score + Display>(result: &BenchmarkResult<Sc>) -> String {
        let mut output = String::new();

        // Title
        writeln!(output, "# Benchmark: {}", result.name).unwrap();
        writeln!(output).unwrap();

        // Metadata
        writeln!(output, "- **Solver**: {}", result.solver_name).unwrap();
        writeln!(output, "- **Problem**: {}", result.problem_name).unwrap();
        writeln!(output, "- **Runs**: {}", result.run_count()).unwrap();
        writeln!(output).unwrap();

        // Summary
        writeln!(output, "## Summary").unwrap();
        writeln!(output).unwrap();

        if let Some(best) = result.best_score() {
            writeln!(output, "| Metric | Value |").unwrap();
            writeln!(output, "|--------|-------|").unwrap();
            writeln!(output, "| Best Score | {} |", best).unwrap();
            if let Some(worst) = result.worst_score() {
                writeln!(output, "| Worst Score | {} |", worst).unwrap();
            }
            writeln!(
                output,
                "| Avg Time | {:.2} ms |",
                result.avg_solve_time().as_secs_f64() * 1000.0
            )
            .unwrap();
            writeln!(
                output,
                "| Min Time | {:.2} ms |",
                result.min_solve_time().as_secs_f64() * 1000.0
            )
            .unwrap();
            writeln!(
                output,
                "| Max Time | {:.2} ms |",
                result.max_solve_time().as_secs_f64() * 1000.0
            )
            .unwrap();
            writeln!(
                output,
                "| Avg Moves/sec | {:.0} |",
                result.avg_moves_per_second()
            )
            .unwrap();
            writeln!(
                output,
                "| Avg Acceptance | {:.2}% |",
                result.avg_acceptance_rate() * 100.0
            )
            .unwrap();
        } else {
            writeln!(output, "*No runs completed.*").unwrap();
        }
        writeln!(output).unwrap();

        // Detailed results
        if !result.runs.is_empty() {
            writeln!(output, "## Run Details").unwrap();
            writeln!(output).unwrap();
            writeln!(output, "| Run | Time (ms) | Score | Moves/sec | Accept % |").unwrap();
            writeln!(output, "|-----|-----------|-------|-----------|----------|").unwrap();

            for run in &result.runs {
                writeln!(
                    output,
                    "| {} | {:.2} | {} | {:.0} | {:.2}% |",
                    run.run_index,
                    run.solve_time.as_secs_f64() * 1000.0,
                    run.final_score,
                    run.moves_per_second(),
                    run.acceptance_rate() * 100.0,
                )
                .unwrap();
            }
        }

        output
    }

    /// Writes Markdown report to a file.
    pub fn to_file<Sc: Score + Display>(
        result: &BenchmarkResult<Sc>,
        path: impl AsRef<Path>,
    ) -> io::Result<()> {
        let md = Self::to_string(result);
        fs::write(path, md)
    }

    /// Writes Markdown report to a writer.
    pub fn write<Sc: Score + Display, W: Write>(
        result: &BenchmarkResult<Sc>,
        mut writer: W,
    ) -> io::Result<()> {
        let md = Self::to_string(result);
        writer.write_all(md.as_bytes())
    }
}

/// Generates a comparison report for multiple benchmark results.
///
/// # Example
///
/// ```
/// use solverforge_benchmark::{BenchmarkResult, MarkdownReport};
/// use solverforge_core::score::SimpleScore;
///
/// let result1 = BenchmarkResult::<SimpleScore>::new("Test", "HC", "Problem");
/// let result2 = BenchmarkResult::<SimpleScore>::new("Test", "Tabu", "Problem");
///
/// let comparison = MarkdownReport::comparison(&[&result1, &result2]);
/// assert!(comparison.contains("## Comparison"));
/// ```
impl MarkdownReport {
    /// Generates a comparison table for multiple results.
    pub fn comparison<Sc: Score + Display>(results: &[&BenchmarkResult<Sc>]) -> String {
        let mut output = String::new();

        writeln!(output, "## Comparison").unwrap();
        writeln!(output).unwrap();
        writeln!(
            output,
            "| Solver | Problem | Best Score | Avg Time (ms) | Moves/sec |"
        )
        .unwrap();
        writeln!(
            output,
            "|--------|---------|------------|---------------|-----------|"
        )
        .unwrap();

        for result in results {
            let best = result
                .best_score()
                .map(|s| format!("{}", s))
                .unwrap_or_else(|| "N/A".to_string());

            writeln!(
                output,
                "| {} | {} | {} | {:.2} | {:.0} |",
                result.solver_name,
                result.problem_name,
                best,
                result.avg_solve_time().as_secs_f64() * 1000.0,
                result.avg_moves_per_second(),
            )
            .unwrap();
        }

        output
    }
}
