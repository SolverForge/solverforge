// Stateless score analysis for planning solutions.

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

// Analysis of a single constraint's contribution to the score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintAnalysis<Sc> {
    // Name of the constraint.
    pub name: String,
    // Weight of the constraint.
    pub weight: Sc,
    // Score contribution from this constraint.
    pub score: Sc,
    // Number of matches (violations or rewards).
    pub match_count: usize,
}

// Result of analyzing a solution's constraints.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreAnalysis<Sc> {
    // The total score.
    pub score: Sc,
    // Analysis of each constraint.
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
/// use solverforge_core::score::SoftScore;
/// use solverforge_solver::manager::{Analyzable, ScoreAnalysis, ConstraintAnalysis};
///
/// #[derive(Clone)]
/// struct Schedule {
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for Schedule {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// impl Analyzable for Schedule {
///     fn analyze(&self) -> ScoreAnalysis<SoftScore> {
///         ScoreAnalysis {
///             score: SoftScore::of(0),
///             constraints: vec![],
///         }
///     }
/// }
///
/// let schedule = Schedule { score: Some(SoftScore::of(0)) };
/// let analysis = schedule.analyze();
/// assert_eq!(analysis.score, SoftScore::of(0));
/// ```
pub trait Analyzable: PlanningSolution + Clone + Send + 'static {
    // Analyzes the solution and returns constraint breakdowns.
    fn analyze(&self) -> ScoreAnalysis<Self::Score>;
}

/// Analyzes a solution for constraint violations.
///
/// Returns a breakdown of each constraint's contribution to the score.
///
/// # Example
///
/// ```
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
/// use solverforge_solver::manager::{analyze, Analyzable, ScoreAnalysis};
///
/// #[derive(Clone)]
/// struct Schedule { score: Option<SoftScore> }
///
/// impl PlanningSolution for Schedule {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// impl Analyzable for Schedule {
///     fn analyze(&self) -> ScoreAnalysis<SoftScore> {
///         ScoreAnalysis { score: SoftScore::of(0), constraints: vec![] }
///     }
/// }
///
/// let schedule = Schedule { score: Some(SoftScore::of(0)) };
/// let result = analyze(&schedule);
/// assert_eq!(result.score, SoftScore::of(0));
/// ```
pub fn analyze<S>(solution: &S) -> ScoreAnalysis<S::Score>
where
    S: Analyzable,
    S::Score: Score,
{
    solution.analyze()
}
