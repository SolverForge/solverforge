use super::*;

/// Element 3 has no legal owner once element 1 is back in route 0.
fn three_blocked_by_one(solution: &VrpSolution, element: &i32) -> Option<usize> {
    (*element == 3 && solution.routes[0].stops.contains(&1)).then_some(usize::MAX)
}

#[test]
fn failed_recreate_restores_elements_it_already_placed() {
    let mut director = create_director(vec![1, 2, 3]);

    let mov = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[0, 2],
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_element_owner_fn(Some(three_blocked_by_one));

    let undo = mov.do_move(&mut director);

    assert!(undo.is_empty());
    assert_eq!(director.working_solution().routes[0].stops, vec![1, 2, 3]);
}
