use crate::domain::{
    ComputedValueRangeProvider, FieldValueRangeProvider, IntegerRange, StaticValueRange,
    ValueRangeProvider,
};

struct TestSolution {
    n: i32,
    values: Vec<i32>,
}

#[test]
fn test_static_value_range() {
    let range = StaticValueRange::new(vec![1, 2, 3, 4, 5]);
    let solution = TestSolution {
        n: 5,
        values: vec![],
    };

    assert_eq!(range.get_values(&solution), vec![1, 2, 3, 4, 5]);
    assert_eq!(range.value_count(&solution), 5);
    assert!(!range.is_empty(&solution));
}

#[test]
fn test_field_value_range_provider() {
    let provider = FieldValueRangeProvider::new(|s: &TestSolution| &s.values);
    let solution = TestSolution {
        n: 3,
        values: vec![10, 20, 30],
    };

    assert_eq!(provider.get_values(&solution), vec![10, 20, 30]);
    assert_eq!(provider.value_count(&solution), 3);
}

#[test]
fn test_computed_value_range_provider() {
    let provider = ComputedValueRangeProvider::new(|s: &TestSolution| (0..s.n).collect());
    let solution = TestSolution {
        n: 4,
        values: vec![],
    };

    assert_eq!(provider.get_values(&solution), vec![0, 1, 2, 3]);
    assert_eq!(provider.value_count(&solution), 4);
}

#[test]
fn test_computed_value_range_type() {
    use crate::domain::variable::ValueRangeType;
    type TestProvider =
        ComputedValueRangeProvider<TestSolution, i32, fn(&TestSolution) -> Vec<i32>>;
    assert_eq!(
        TestProvider::value_range_type(),
        ValueRangeType::EntityDependent
    );
}

#[test]
fn test_integer_range() {
    let range = IntegerRange::new(5, 10);
    let solution = TestSolution {
        n: 0,
        values: vec![],
    };

    let values: Vec<i64> = ValueRangeProvider::<TestSolution, i64>::get_values(&range, &solution);
    assert_eq!(values, vec![5, 6, 7, 8, 9]);
    assert_eq!(
        ValueRangeProvider::<TestSolution, i64>::value_count(&range, &solution),
        5
    );
}

#[test]
fn test_integer_range_value_range_type() {
    use crate::domain::variable::ValueRangeType;

    let range = IntegerRange::new(5, 10);
    assert_eq!(
        range.value_range_type(),
        ValueRangeType::CountableRange { from: 5, to: 10 }
    );
}

#[test]
fn test_integer_range_i32() {
    let range = IntegerRange::from_zero(3);
    let solution = TestSolution {
        n: 0,
        values: vec![],
    };

    let values: Vec<i32> = ValueRangeProvider::<TestSolution, i32>::get_values(&range, &solution);
    assert_eq!(values, vec![0, 1, 2]);
}
