// ShadowAwareScoreDirector tests

use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::incremental::IncrementalUniConstraint;
use crate::director::shadow_aware::{ShadowAwareScoreDirector, ShadowVariableSupport};
use crate::director::typed::TypedScoreDirector;
use crate::director::ScoreDirector;

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

// Creates a ShadowAwareScoreDirector for testing.
fn create_shadow_director(
    values: Vec<i32>,
) -> ShadowAwareScoreDirector<
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
> {
    let solution = ShadowSolution::new(values);
    let constraint = make_sum_constraint();
    let inner = TypedScoreDirector::new(solution, (constraint,));
    ShadowAwareScoreDirector::new(inner)
}

#[test]
fn test_shadow_update_called_on_variable_change() {
    let mut director = create_shadow_director(vec![10, 20, 30]);

    // Initialize
    director.calculate_score();

    // Shadow should have been updated during initialization
    // (via working_solution_mut access pattern)
    assert_eq!(director.working_solution().cached_sum, 0);

    // Change value and verify shadow update
    director.before_variable_changed(0, 0, "values");
    director.working_solution_mut().values[0] = 50;
    director.after_variable_changed(0, 0, "values");

    assert_eq!(director.working_solution().cached_sum, 100); // 50 + 20 + 30
}

#[test]
fn test_shadow_inner_access() {
    let director = create_shadow_director(vec![1, 2, 3]);
    assert!(!director.inner().is_initialized());
}

#[test]
fn test_shadow_into_inner_consumes() {
    let director = create_shadow_director(vec![1]);
    let recovered = director.into_inner();
    assert_eq!(recovered.working_solution().values.len(), 1);
}
