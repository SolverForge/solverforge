use crate::stream::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use crate::stream::joiner::{equal_bi, filtering, less_than, Joiner};
use crate::stream::relational::DenseRowStore;

#[test]
fn equality_components_normalize_across_interleaved_residuals() {
    use std::borrow::Cow;
    let plan = equal_bi(|x: &(u32, u32)| x.0, |x: &(u32, u32)| x.0)
        .and(filtering(|x: &(u32, u32), _: &(u32, u32)| x.1 != 7))
        .and(equal_bi(|x: &(u32, u32)| x.1, |x: &(u32, u32)| x.1))
        .compile();
    let mut state = plan.new_indexes();
    let mut store = DenseRowStore::new();
    for value in [(1, 7), (1, 8), (2, 7)] {
        let h = store.insert(value);
        plan.insert_right(&mut state, h, &value);
    }
    let candidates = plan.right_candidates(&state, &(1, 7));
    assert!(
        matches!(candidates, Cow::Borrowed(_)),
        "interleaved equality must normalize to one bucket"
    );
    assert_eq!(candidates.len(), 1);
    assert!(!plan.candidate_matches(&(1, 7), &(1, 7)));
    assert!(plan.candidate_matches(&(1, 8), &(1, 8)));
}

#[test]
fn composed_equalities_use_one_borrowed_heterogeneous_bucket() {
    use std::borrow::Cow;
    let plan = equal_bi(|x: &(u32, String)| x.0, |x: &(u32, String)| x.0)
        .and(equal_bi(
            |x: &(u32, String)| x.1.clone(),
            |x: &(u32, String)| x.1.clone(),
        ))
        .compile();
    let mut state = plan.new_indexes();
    let mut store = DenseRowStore::new();
    let values = [
        (1, "a".to_owned()),
        (1, "b".to_owned()),
        (2, "a".to_owned()),
        (1, "a".to_owned()),
    ];
    let handles: Vec<_> = values.iter().map(|v| store.insert(v)).collect();
    for (&h, v) in handles.iter().zip(&values) {
        plan.insert_left(&mut state, h, v);
        plan.insert_right(&mut state, h, v);
    }
    let candidates = plan.right_candidates(&state, &(1, "a".to_owned()));
    assert!(
        matches!(candidates, Cow::Borrowed(_)),
        "composite equality must not intersect allocated lists"
    );
    assert_eq!(&*candidates, &[handles[0], handles[3]]);
    assert_eq!(
        &*plan.left_candidates(&state, &(1, "a".to_owned())),
        &[handles[0], handles[3]]
    );
    plan.remove_right(&mut state, handles[0]);
    assert_eq!(
        &*plan.right_candidates(&state, &(1, "a".to_owned())),
        &[handles[3]]
    );
}

#[test]
fn compiled_equality_predicate_only_probes_equal_bucket_in_both_orders() {
    let mut store = DenseRowStore::new();
    let rows: Vec<_> = (0..100).map(|i| (store.insert(i), i)).collect();
    let condition =
        equal_bi(|x: &i32| *x, |x: &i32| *x).and(filtering(|x: &i32, _: &i32| *x % 2 == 0));
    let plan = condition.compile();
    let mut state = plan.new_indexes();
    for &(handle, value) in &rows {
        plan.insert_right(&mut state, handle, &value);
        plan.insert_left(&mut state, handle, &value);
    }
    assert_eq!(plan.right_candidates(&state, &42), vec![rows[42].0]);
    assert_eq!(plan.left_candidates(&state, &42), vec![rows[42].0]);
    assert!(plan.matches(&42, &42));
    assert!(!plan.matches(&43, &43));
    plan.remove_right(&mut state, rows[42].0);
    assert!(plan.right_candidates(&state, &42).is_empty());

    let reversed = filtering(|x: &i32, _: &i32| *x % 2 == 0)
        .and(equal_bi(|x: &i32| *x, |x: &i32| *x))
        .compile();
    let mut state = reversed.new_indexes();
    for &(handle, value) in &rows {
        reversed.insert_right(&mut state, handle, &value);
    }
    assert_eq!(reversed.right_candidates(&state, &42), vec![rows[42].0]);
}

#[test]
fn compiled_ordered_queries_invert_direction_on_left_updates() {
    let mut store = DenseRowStore::new();
    let rows: Vec<_> = [i64::MIN, 0, 0, i64::MAX]
        .into_iter()
        .map(|i| (store.insert(i), i))
        .collect();
    let plan = less_than(|x: &i64| *x, |x: &i64| *x).compile();
    let mut state = plan.new_indexes();
    for &(handle, value) in &rows {
        plan.insert_left(&mut state, handle, &value);
        plan.insert_right(&mut state, handle, &value);
    }
    assert_eq!(plan.right_candidates(&state, &0), vec![rows[3].0]);
    assert_eq!(plan.left_candidates(&state, &0), vec![rows[0].0]);
    plan.insert_right(&mut state, rows[3].0, &-1);
    assert!(plan.right_candidates(&state, &0).is_empty());
}
