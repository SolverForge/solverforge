//! Benchmarking framework for SolverForge.
//!
//! This module provides types and utilities for benchmarking solver configurations
//! against problem instances, collecting statistics, and generating reports.
//!
//! # Overview
//!
//! The benchmarking framework allows you to:
//! - Run multiple solver configurations against the same problem
//! - Execute warmup runs before measurement
//! - Collect detailed statistics (time, score progression, moves/second)
//! - Export results to CSV and Markdown
//!
//! # Zero-Erasure Design
//!
//! All benchmark types use monomorphized generics. The solver factory `F` and
//! score director factory `D` are stored as type parameters, not trait objects.
//!
//! # Example
//!
//! ```
//! use solverforge_benchmark::BenchmarkConfig;
//!
//! // Configure a benchmark run
//! let config = BenchmarkConfig::new("NQueens Benchmark")
//!     .with_warmup_count(2)
//!     .with_run_count(5)
//!     .with_csv_output("results.csv")
//!     .with_markdown_output("report.md");
//!
//! assert_eq!(config.name(), "NQueens Benchmark");
//! assert_eq!(config.warmup_count(), 2);
//! assert_eq!(config.run_count(), 5);
//! ```
//!
//! Full benchmark usage with solver and score director factories:
//!
//! ```text
//! let benchmark = Benchmark::new(
//!     config,
//!     "HillClimbing",
//!     "8-Queens",
//!     || create_problem(),
//!     |s| create_score_director(s),
//!     || create_solver(),
//! );
//! let results = benchmark.run();
//! println!("{}", results.to_markdown());
//! ```

mod config;
mod report;
mod result;
mod runner;

pub use config::BenchmarkConfig;
pub use report::{CsvExporter, MarkdownReport};
pub use result::{BenchmarkResult, BenchmarkRun};
pub use runner::{Benchmark, BenchmarkBuilder};
