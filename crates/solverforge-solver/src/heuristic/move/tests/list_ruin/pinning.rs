use super::*;

#[test]
fn ruin_never_probes_or_commits_into_a_pinned_destination() {
    let mut director = create_director(vec![1, 2]);
    director.working_solution_mut().routes.push(Route {
        stops: vec![3],
        pinned: true,
    });
    let mov = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[1],
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    assert!(mov.is_doable(&director));
    let undo = mov.do_move(&mut director);
    assert_eq!(director.working_solution().routes[1].stops, vec![3]);
    mov.undo_move(&mut director, undo);
    assert_eq!(director.working_solution().routes[0].stops, vec![1, 2]);
    assert_eq!(director.working_solution().routes[1].stops, vec![3]);
}

#[test]
fn ruin_rejects_a_pinned_source() {
    let mut director = create_director(vec![1, 2]);
    director.working_solution_mut().routes[0].pinned = true;
    director.working_solution_mut().routes.push(Route {
        stops: vec![],
        pinned: false,
    });
    let mov = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[1],
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    assert!(!mov.is_doable(&director));
    assert_eq!(director.working_solution().routes[0].stops, vec![1, 2]);
}
