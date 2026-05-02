// Tests for constraint set types.

use super::super::{ConstraintSet, IncrementalConstraint};
use solverforge_core::score::SoftScore;
use solverforge_core::ConstraintRef;

// Simple test constraint that counts entities matching a predicate.
struct CountingConstraint<S, F> {
    constraint_ref: ConstraintRef,
    extractor: fn(&S) -> usize,
    predicate: F,
    weight: i64,
    is_hard: bool,
}

impl<S, F> CountingConstraint<S, F>
where
    F: Fn(&S, usize) -> bool,
{
    fn new(name: &str, extractor: fn(&S) -> usize, predicate: F, weight: i64) -> Self {
        Self {
            constraint_ref: ConstraintRef::new("", name),
            extractor,
            predicate,
            weight,
            is_hard: false,
        }
    }

    fn new_with_hardness(
        name: &str,
        extractor: fn(&S) -> usize,
        predicate: F,
        weight: i64,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref: ConstraintRef::new("", name),
            extractor,
            predicate,
            weight,
            is_hard,
        }
    }

    fn new_with_ref_and_hardness(
        package: &str,
        name: &str,
        extractor: fn(&S) -> usize,
        predicate: F,
        weight: i64,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref: ConstraintRef::new(package, name),
            extractor,
            predicate,
            weight,
            is_hard,
        }
    }
}

impl<S, F> IncrementalConstraint<S, SoftScore> for CountingConstraint<S, F>
where
    S: Send + Sync,
    F: Fn(&S, usize) -> bool + Send + Sync,
{
    fn evaluate(&self, solution: &S) -> SoftScore {
        let count = (self.extractor)(solution);
        let matches = (0..count)
            .filter(|&i| (self.predicate)(solution, i))
            .count() as i64;
        SoftScore::of(-matches * self.weight)
    }

    fn match_count(&self, solution: &S) -> usize {
        let count = (self.extractor)(solution);
        (0..count)
            .filter(|&i| (self.predicate)(solution, i))
            .count()
    }

    fn initialize(&mut self, solution: &S) -> SoftScore {
        self.evaluate(solution)
    }

    fn on_insert(
        &mut self,
        solution: &S,
        entity_index: usize,
        _descriptor_index: usize,
    ) -> SoftScore {
        if (self.predicate)(solution, entity_index) {
            SoftScore::of(-self.weight)
        } else {
            SoftScore::of(0)
        }
    }

    fn on_retract(
        &mut self,
        solution: &S,
        entity_index: usize,
        _descriptor_index: usize,
    ) -> SoftScore {
        if (self.predicate)(solution, entity_index) {
            SoftScore::of(self.weight)
        } else {
            SoftScore::of(0)
        }
    }

    fn reset(&mut self) {}

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        self.constraint_ref.clone()
    }
}

#[derive(Clone)]
struct TestSolution {
    values: Vec<Option<i32>>,
}

fn entity_count(s: &TestSolution) -> usize {
    s.values.len()
}

#[test]
fn test_empty_constraint_set() {
    let constraints: () = ();
    let solution = TestSolution {
        values: vec![Some(1), None],
    };

    let score: SoftScore = constraints.evaluate_all(&solution);
    assert_eq!(score, SoftScore::of(0));
    assert_eq!(
        <() as ConstraintSet<TestSolution, SoftScore>>::constraint_count(&constraints),
        0
    );
}

#[test]
fn test_single_constraint() {
    let constraint = CountingConstraint::new(
        "unassigned",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_none(),
        1,
    );

    let constraints = (constraint,);
    let solution = TestSolution {
        values: vec![Some(1), None, None],
    };

    assert_eq!(constraints.evaluate_all(&solution), SoftScore::of(-2));
    assert_eq!(constraints.constraint_count(), 1);
}

#[test]
fn test_two_constraints() {
    let c1 = CountingConstraint::new(
        "unassigned",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_none(),
        1,
    );
    let c2 = CountingConstraint::new(
        "high_value",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_some_and(|v| v > 5),
        2,
    );

    let constraints = (c1, c2);
    let solution = TestSolution {
        values: vec![Some(10), None, Some(3)],
    };

    // c1: 1 unassigned (-1)
    // c2: 1 high value (-2)
    assert_eq!(constraints.evaluate_all(&solution), SoftScore::of(-3));
    assert_eq!(constraints.constraint_count(), 2);
}

#[test]
fn constraint_set_returns_constraint_metadata() {
    let c1 = CountingConstraint::new_with_hardness(
        "unassigned",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_none(),
        1,
        true,
    );
    let c2 = CountingConstraint::new(
        "high_value",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_some_and(|v| v > 5),
        2,
    );

    let metadata = (c1, c2).constraint_metadata();

    assert_eq!(metadata.len(), 2);
    assert_eq!(metadata[0].name(), "unassigned");
    assert!(metadata[0].is_hard);
    assert_eq!(metadata[1].name(), "high_value");
    assert!(!metadata[1].is_hard);
}

#[test]
fn constraint_set_deduplicates_matching_constraint_metadata() {
    let c1 = CountingConstraint::new_with_ref_and_hardness(
        "pkg_a",
        "same",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_none(),
        1,
        true,
    );
    let c2 = CountingConstraint::new_with_ref_and_hardness(
        "pkg_a",
        "same",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_some(),
        1,
        true,
    );

    let metadata = (c1, c2).constraint_metadata();

    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0].name(), "same");
    assert_eq!(metadata[0].full_name(), "pkg_a/same");
    assert!(metadata[0].is_hard);
}

#[test]
fn constraint_set_preserves_same_name_in_different_packages() {
    let c1 = CountingConstraint::new_with_ref_and_hardness(
        "pkg_a",
        "same",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_none(),
        1,
        true,
    );
    let c2 = CountingConstraint::new_with_ref_and_hardness(
        "pkg_b",
        "same",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_some(),
        1,
        true,
    );

    let metadata = (c1, c2).constraint_metadata();

    assert_eq!(metadata.len(), 2);
    assert_eq!(metadata[0].full_name(), "pkg_a/same");
    assert_eq!(metadata[1].full_name(), "pkg_b/same");
}

#[test]
fn constraint_set_preserves_same_name_with_different_package_hardness() {
    let c1 = CountingConstraint::new_with_ref_and_hardness(
        "pkg_a",
        "same",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_none(),
        1,
        true,
    );
    let c2 = CountingConstraint::new_with_ref_and_hardness(
        "pkg_b",
        "same",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_some(),
        1,
        false,
    );

    let metadata = (c1, c2).constraint_metadata();

    assert_eq!(metadata.len(), 2);
    assert!(metadata[0].is_hard);
    assert!(!metadata[1].is_hard);
}

#[test]
#[should_panic(expected = "constraint `pkg_a/same` has conflicting hard/non-hard metadata")]
fn constraint_set_rejects_conflicting_constraint_metadata() {
    let c1 = CountingConstraint::new_with_ref_and_hardness(
        "pkg_a",
        "same",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_none(),
        1,
        true,
    );
    let c2 = CountingConstraint::new_with_ref_and_hardness(
        "pkg_a",
        "same",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_some(),
        1,
        false,
    );

    let _ = (c1, c2).constraint_metadata();
}

#[test]
fn test_incremental_insert() {
    let c1 = CountingConstraint::new(
        "unassigned",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_none(),
        1,
    );

    let mut constraints = (c1,);
    let solution = TestSolution {
        values: vec![None, Some(5), None],
    };

    // Entity 0 is unassigned -> delta = -1
    let delta = constraints.on_insert_all(&solution, 0, 0);
    assert_eq!(delta, SoftScore::of(-1));

    // Entity 1 is assigned -> delta = 0
    let delta = constraints.on_insert_all(&solution, 1, 0);
    assert_eq!(delta, SoftScore::of(0));
}

#[test]
fn test_incremental_retract() {
    let c1 = CountingConstraint::new(
        "unassigned",
        entity_count,
        |s: &TestSolution, i| s.values[i].is_none(),
        1,
    );

    let mut constraints = (c1,);
    let solution = TestSolution {
        values: vec![None, Some(5)],
    };

    // Retract unassigned entity -> delta = +1 (removes penalty)
    let delta = constraints.on_retract_all(&solution, 0, 0);
    assert_eq!(delta, SoftScore::of(1));

    // Retract assigned entity -> delta = 0
    let delta = constraints.on_retract_all(&solution, 1, 0);
    assert_eq!(delta, SoftScore::of(0));
}

#[test]
fn test_sixteen_constraints() {
    // Test the maximum tuple size
    let make_constraint = |n: i32| {
        CountingConstraint::new(
            &format!("c{}", n),
            entity_count,
            move |s: &TestSolution, i| s.values[i] == Some(n),
            1,
        )
    };

    let constraints = (
        make_constraint(0),
        make_constraint(1),
        make_constraint(2),
        make_constraint(3),
        make_constraint(4),
        make_constraint(5),
        make_constraint(6),
        make_constraint(7),
        make_constraint(8),
        make_constraint(9),
        make_constraint(10),
        make_constraint(11),
        make_constraint(12),
        make_constraint(13),
        make_constraint(14),
        make_constraint(15),
    );

    let solution = TestSolution {
        values: vec![Some(5), Some(10), Some(15)],
    };

    // Only values 5, 10, 15 match -> 3 penalties
    assert_eq!(constraints.evaluate_all(&solution), SoftScore::of(-3));
    assert_eq!(constraints.constraint_count(), 16);
}
