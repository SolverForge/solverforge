use super::*;

#[test]
fn list_precedence_selector_ignores_noncritical_route_arcs() {
    let director = create_director(
        vec![
            Task {
                duration: 3,
                next: Some(1),
            },
            Task {
                duration: 2,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
        ],
        vec![vec![0, 2], vec![1]],
    );
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    assert_eq!(moves.len(), 6);
    assert!(
        matches!(&moves[0], ListMoveUnion::ListChange(mov) if mov.source_entity_index() == 0 && mov.source_position() == 0 && mov.dest_position() == 2)
    );
    assert!(
        matches!(&moves[1], ListMoveUnion::ListChange(mov) if mov.source_entity_index() == 0 && mov.source_position() == 1 && mov.dest_position() == 0)
    );
    assert!(
        matches!(&moves[2], ListMoveUnion::ListSwap(mov) if mov.first_entity_index() == 0 && mov.first_position() == 0 && mov.second_position() == 1)
    );
    assert!(
        matches!(&moves[3], ListMoveUnion::ListReverse(mov) if mov.entity_index() == 0 && mov.start() == 0 && mov.end() == 2)
    );
    assert!(
        matches!(&moves[4], ListMoveUnion::ListRuin(mov) if mov.entity_index() == 0 && mov.element_indices() == [0, 1])
    );
    assert!(
        matches!(&moves[5], ListMoveUnion::ListPermute(mov) if mov.entity_index() == 0 && mov.start() == 0 && mov.end() == 2 && mov.permutation() == [1, 0])
    );
}

#[test]
fn list_precedence_selector_skips_moves_that_force_fixed_successor_cycles() {
    let director = create_director(
        vec![
            Task {
                duration: 1,
                next: Some(1),
            },
            Task {
                duration: 1,
                next: None,
            },
        ],
        vec![vec![0, 1]],
    );
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    assert_eq!(selector.size(&director), 1);
    assert_eq!(moves.len(), 1);
    assert!(
        matches!(&moves[0], ListMoveUnion::ListRuin(mov) if mov.entity_index() == 0 && mov.element_indices() == [0, 1])
    );
}

#[test]
fn list_precedence_sublist_destinations_use_route_coordinates() {
    let director = create_director(
        vec![
            Task {
                duration: 1,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
            Task {
                duration: 10,
                next: Some(1),
            },
        ],
        vec![vec![0, 1, 2], vec![3]],
    );
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    assert_eq!(moves.len(), selector.size(&director));
    assert!(moves.iter().any(
        |mov| matches!(mov, ListMoveUnion::SublistChange(mov) if mov.source_start() == 1 && mov.source_end() == 3 && mov.dest_position() == 0)
    ));
    assert!(moves.iter().all(|mov| match mov {
        ListMoveUnion::SublistChange(mov) => mov.source_start() != mov.dest_position(),
        _ => true,
    }));
}

#[test]
fn list_precedence_selector_emits_singleton_critical_node_relocations() {
    let director = create_director(
        vec![
            Task {
                duration: 3,
                next: Some(2),
            },
            Task {
                duration: 1,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
        ],
        vec![vec![0, 1], vec![2]],
    );
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    assert_eq!(moves.len(), selector.size(&director));
    assert!(moves.iter().any(
        |mov| matches!(mov, ListMoveUnion::ListChange(mov) if mov.source_entity_index() == 0 && mov.source_position() == 0 && mov.dest_position() == 2)
    ));
}

#[test]
fn list_precedence_selector_skips_cyclic_current_graph() {
    let director = create_director(
        vec![
            Task {
                duration: 1,
                next: Some(1),
            },
            Task {
                duration: 1,
                next: None,
            },
        ],
        vec![vec![1, 0]],
    );
    let selector = selector();

    assert_eq!(selector.size(&director), 0);
    assert_eq!(selector.iter_moves(&director).count(), 0);
}
