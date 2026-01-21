//! SolutionManager for stateless score analysis.
//!
//! This follows Timefold's `SolutionManager` pattern - stateless operations
//! on solutions without job tracking:
//! - Analyzing solutions for constraint violations
//! - Score calculation and breakdown
//!
//! For async job management, see `SolverManager`.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

/// Analysis of a single constraint's contribution to the score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintAnalysis<Sc> {
    /// Name of the constraint.
    pub name: String,
    /// Weight of the constraint.
    pub weight: Sc,
    /// Score contribution from this constraint.
    pub score: Sc,
    /// Number of matches (violations or rewards).
    pub match_count: usize,
}

/// Result of analyzing a solution's constraints.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreAnalysis<Sc> {
    /// The total score.
    pub score: Sc,
    /// Analysis of each constraint.
    pub constraints: Vec<ConstraintAnalysis<Sc>>,
}

/// Trait for solutions that can be analyzed for constraint violations.
///
/// This trait is implemented by the `#[planning_solution]` macro when
/// `constraints` is specified. It provides constraint analysis without
/// knowing the concrete solution type.
///
/// # Example
///
/// ```
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_solver::manager::{Analyzable, ScoreAnalysis, ConstraintAnalysis};
///
/// #[derive(Clone)]
/// struct Schedule {
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for Schedule {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// impl Analyzable for Schedule {
///     fn analyze(&self) -> ScoreAnalysis<SimpleScore> {
///         ScoreAnalysis {
///             score: SimpleScore::of(0),
///             constraints: vec![],
///         }
///     }
/// }
///
/// let schedule = Schedule { score: Some(SimpleScore::of(0)) };
/// let analysis = schedule.analyze();
/// assert_eq!(analysis.score, SimpleScore::of(0));
/// ```
pub trait Analyzable: PlanningSolution + Clone + Send + 'static {
    /// Analyzes the solution and returns constraint breakdowns.
    fn analyze(&self) -> ScoreAnalysis<Self::Score>;
}

/// Stateless service for score analysis.
///
/// This is the Rust equivalent of Timefold's `SolutionManager`. It provides
/// stateless operations on solutions without tracking jobs.
///
/// # Type Parameters
///
/// * `S` - Solution type that implements `Analyzable`
pub struct SolutionManager<S> {
    _marker: PhantomData<S>,
}

impl<S> Default for SolutionManager<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> SolutionManager<S> {
    /// Creates a new SolutionManager.
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<S> SolutionManager<S>
where
    S: Analyzable,
    S::Score: Score,
{
    /// Analyzes a solution for constraint violations.
    ///
    /// Returns a breakdown of each constraint's contribution to the score.
    pub fn analyze(&self, solution: &S) -> ScoreAnalysis<S::Score> {
        solution.analyze()
    }
}
