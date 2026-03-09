// Shadow variable support tests using ScoreDirector directly.

use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::incremental::IncrementalUniConstraint;
use crate::director::score_director::ScoreDirector;
use crate::director::shadow_aware::ShadowVariableSupport;

use solverforge_test::shadow::ShadowSolution;

// Implement ShadowVariableSupport for ShadowSolution (trait is in this crate)
impl ShadowVariableSupport for ShadowSolution {
    fn update_entity_shadows(&mut self, _entity_index: usize) {
        self.cached_sum = self.values.iter().sum();
    }

    fn update_all_shadows(&mut self) {
        self.cached_sum = self.values.iter().sum();
    }
}

// Creates a constraint that penalizes when cached_sum exceeds 100.
fn make_sum_constraint() -> IncrementalUniConstraint<
    ShadowSolution,
    i32,
    fn(&ShadowSolution) -> &[i32],
    fn(&ShadowSolution, &i32) -> bool,
    fn(&i32) -> SoftScore,
    SoftScore,
> {
    fn extract(s: &ShadowSolution) -> &[i32] {
        std::slice::from_ref(&s.cached_sum)
    }
    fn filter(_s: &ShadowSolution, &sum: &i32) -> bool {
        sum > 100
    }
    fn score(&sum: &i32) -> SoftScore {
        SoftScore::of((sum - 100) as i64)
    }

    IncrementalUniConstraint::new(
        ConstraintRef::new("", "SumLimit"),
        ImpactType::Penalty,
        extract as fn(&ShadowSolution) -> &[i32],
        filter as fn(&ShadowSolution, &i32) -> bool,
        score as fn(&i32) -> SoftScore,
        false,
    )
}

// Creates a ScoreDirector for testing shadow variable support.
fn create_director(
    values: Vec<i32>,
) -> ScoreDirector<
    ShadowSolution,
    (
        IncrementalUniConstraint<
            ShadowSolution,
            i32,
            fn(&ShadowSolution) -> &[i32],
            fn(&ShadowSolution, &i32) -> bool,
            fn(&i32) -> SoftScore,
            SoftScore,
        >,
    ),
> {
    let solution = ShadowSolution::new(values);
    let constraint = make_sum_constraint();
    ScoreDirector::new(solution, (constraint,))
}

#[test]
fn test_shadow_update_called_on_variable_change() {
    let mut director = create_director(vec![10, 20, 30]);

    // Initialize
    director.calculate_score();

    // Shadow should have been updated during initialization
    assert_eq!(director.working_solution().cached_sum, 0);

    // Change value and verify shadow update via after_variable_changed_with_shadows
    director.before_variable_changed(0, 0);
    director.working_solution_mut().values[0] = 50;
    director.after_variable_changed_with_shadows(0, 0);

    assert_eq!(director.working_solution().cached_sum, 100); // 50 + 20 + 30
}

#[test]
fn test_director_is_not_initialized_before_calculate() {
    let director = create_director(vec![1, 2, 3]);
    assert!(!director.is_initialized());
}

#[test]
fn test_take_solution_after_use() {
    let director = create_director(vec![1]);
    let solution = director.take_solution();
    assert_eq!(solution.values.len(), 1);
}
