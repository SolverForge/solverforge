use super::*;

#[test]
fn bench_sublist_change_cursor_enumeration() {
    let director = create_director(benchmark_plan(10, 18));
    let selector = SublistChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        1,
        3,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    benchmark_cursor("sublist_change_cursor", 15, || {
        selector.iter_moves(&director).count()
    });
}

#[test]
fn bench_sublist_swap_cursor_enumeration() {
    let director = create_director(benchmark_plan(10, 18));
    let selector = SublistSwapMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        1,
        3,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    benchmark_cursor("sublist_swap_cursor", 15, || {
        selector.iter_moves(&director).count()
    });
}
