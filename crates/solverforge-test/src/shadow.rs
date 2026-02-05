//! Shadow variable test fixtures.
//!
//! Provides a solution type with shadow variable support for testing
//! `ShadowAwareScoreDirector` and related shadow variable infrastructure.
//!
//! # Example
//!
//! ```ignore
//! use solverforge_test::shadow::{ShadowSolution, create_shadow_director};
//!
//! let mut director = create_shadow_director(vec![10, 20, 30]);
//! director.calculate_score();
//!
//! // Shadow variable (cached_sum) is updated automatically on variable changes
//! director.before_variable_changed(0, 0, "values");
//! director.working_solution_mut().values[0] = 50;
//! director.after_variable_changed(0, 0, "values");
//!
//! assert_eq!(director.working_solution().cached_sum, 100); // 50 + 20 + 30
//! ```

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;
use solverforge_scoring::director::shadow_aware::{
    ShadowAwareScoreDirector, ShadowVariableSupport,
};
use solverforge_scoring::director::typed::TypedScoreDirector;

/// A solution with shadow variable support for testing.
///
/// Contains:
/// - `values`: Planning variables (a vector of integers)
/// - `cached_sum`: Shadow variable (sum of all values)
/// - `score`: The solution's score
#[derive(Clone, Debug)]
pub struct ShadowSolution {
    pub values: Vec<i32>,
    /// Shadow variable: cached sum of all values.
    pub cached_sum: i32,
    pub score: Option<SimpleScore>,
}

impl ShadowSolution {
    /// Creates a new shadow solution with the given values.
    ///
    /// The cached_sum is initialized to 0 and will be updated
    /// when shadow variables are triggered.
    pub fn new(values: Vec<i32>) -> Self {
        Self {
            values,
            cached_sum: 0,
            score: None,
        }
    }

    /// Creates a shadow solution with pre-computed cached_sum.
    pub fn with_cached_sum(values: Vec<i32>, cached_sum: i32) -> Self {
        Self {
            values,
            cached_sum,
            score: None,
        }
    }
}

impl Default for ShadowSolution {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl PlanningSolution for ShadowSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl ShadowVariableSupport for ShadowSolution {
    fn update_entity_shadows(&mut self, _entity_index: usize) {
        // Update the cached sum when any entity changes
        self.cached_sum = self.values.iter().sum();
    }

    fn update_all_shadows(&mut self) {
        self.cached_sum = self.values.iter().sum();
    }
}

/// Creates a constraint that penalizes when cached_sum exceeds 100.
///
/// Returns an IncrementalUniConstraint that:
/// - Filters: `cached_sum > 100`
/// - Scores: `cached_sum - 100` penalty points
pub fn make_sum_constraint() -> IncrementalUniConstraint<
    ShadowSolution,
    i32,
    fn(&ShadowSolution) -> &[i32],
    fn(&ShadowSolution, &i32) -> bool,
    fn(&i32) -> SimpleScore,
    SimpleScore,
> {
    fn extract(s: &ShadowSolution) -> &[i32] {
        std::slice::from_ref(&s.cached_sum)
    }
    fn filter(_s: &ShadowSolution, &sum: &i32) -> bool {
        sum > 100
    }
    fn score(&sum: &i32) -> SimpleScore {
        SimpleScore::of((sum - 100) as i64)
    }

    IncrementalUniConstraint::new(
        ConstraintRef::new("", "SumLimit"),
        ImpactType::Penalty,
        extract as fn(&ShadowSolution) -> &[i32],
        filter as fn(&ShadowSolution, &i32) -> bool,
        score as fn(&i32) -> SimpleScore,
        false,
    )
}

/// Type alias for the shadow-aware score director with sum constraint.
pub type ShadowDirector = ShadowAwareScoreDirector<
    ShadowSolution,
    TypedScoreDirector<
        ShadowSolution,
        (
            IncrementalUniConstraint<
                ShadowSolution,
                i32,
                fn(&ShadowSolution) -> &[i32],
                fn(&ShadowSolution, &i32) -> bool,
                fn(&i32) -> SimpleScore,
                SimpleScore,
            >,
        ),
    >,
>;

/// Creates a ShadowAwareScoreDirector for testing shadow variable support.
///
/// The director uses a constraint that penalizes when `cached_sum > 100`.
///
/// # Example
///
/// ```ignore
/// use solverforge_test::shadow::create_shadow_director;
/// use solverforge_scoring::ScoreDirector;
///
/// let mut director = create_shadow_director(vec![10, 20, 30]);
/// director.calculate_score();
///
/// // Sum is 60, which is <= 100, so no penalty
/// assert_eq!(director.working_solution().cached_sum, 0); // Not yet updated
/// ```
pub fn create_shadow_director(values: Vec<i32>) -> ShadowDirector {
    let solution = ShadowSolution::new(values);
    let constraint = make_sum_constraint();
    let inner = TypedScoreDirector::new(solution, (constraint,));
    ShadowAwareScoreDirector::new(inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    use solverforge_scoring::ScoreDirector;

    #[test]
    fn test_shadow_solution_creation() {
        let s1 = ShadowSolution::new(vec![1, 2, 3]);
        assert_eq!(s1.values, vec![1, 2, 3]);
        assert_eq!(s1.cached_sum, 0);
        assert!(s1.score.is_none());

        let s2 = ShadowSolution::with_cached_sum(vec![1, 2, 3], 6);
        assert_eq!(s2.cached_sum, 6);
    }

    #[test]
    fn test_shadow_variable_update() {
        let mut solution = ShadowSolution::new(vec![10, 20, 30]);
        assert_eq!(solution.cached_sum, 0);

        solution.update_entity_shadows(0);
        assert_eq!(solution.cached_sum, 60);
    }

    #[test]
    fn test_create_shadow_director() {
        let mut director = create_shadow_director(vec![10, 20, 30]);

        // Calculate initial score
        director.calculate_score();

        // Shadow should not be updated yet (initialization doesn't call update_entity_shadows)
        assert_eq!(director.working_solution().cached_sum, 0);
    }

    #[test]
    fn test_shadow_update_on_variable_change() {
        let mut director = create_shadow_director(vec![10, 20, 30]);
        director.calculate_score();

        // Change value and verify shadow update
        director.before_variable_changed(0, 0, "values");
        director.working_solution_mut().values[0] = 50;
        director.after_variable_changed(0, 0, "values");

        // Shadow should now be updated: 50 + 20 + 30 = 100
        assert_eq!(director.working_solution().cached_sum, 100);
    }

    #[test]
    fn test_sum_constraint_no_penalty() {
        let constraint = make_sum_constraint();
        let solution = ShadowSolution::with_cached_sum(vec![], 50);

        // Sum <= 100, no penalty
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
    }

    #[test]
    fn test_sum_constraint_with_penalty() {
        let constraint = make_sum_constraint();
        let solution = ShadowSolution::with_cached_sum(vec![], 150);

        // Sum = 150 > 100, penalty = -(150 - 100) = -50
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-50));
    }
}
