use super::*;

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
