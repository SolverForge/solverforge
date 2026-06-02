#[test]
fn dynamic_scalar_notifications_use_resolved_descriptor_index() {
    let descriptor = reversed_dynamic_descriptor();
    let scalar = DynamicScalarVariableSlot::new(
        EntityClassId(0),
        VariableId(0),
        "Task",
        "worker",
        false,
    )
    .resolved_against(&descriptor)
    .expect("dynamic scalar slot should resolve against logical IDs");
    assert_eq!(scalar.descriptor_index(), 1);

    let plan = DynamicPlan {
        score: None,
        scalar_values: vec![Some(0)],
        scalar_candidates: vec![vec![0, 1]],
        lists: Vec::new(),
        list_element_count: 0,
    };
    let mut director = ScoreDirector::with_descriptor(
        plan,
        PreferWorkerOne::for_descriptor_index(1),
        descriptor,
        |solution, descriptor_index| match descriptor_index {
            0 => solution.lists.len(),
            1 => solution.scalar_values.len(),
            _ => 0,
        },
    );
    assert_eq!(director.calculate_score(), SoftScore::of(-10));

    let mov = crate::heuristic::r#move::DynamicScalarChangeMove::new(scalar, 0, Some(1));
    crate::heuristic::r#move::Move::do_move(&mov, &mut director);

    let score = director.calculate_score();
    assert_eq!(score, SoftScore::of(0));
    assert_eq!(Director::fresh_score(&director), Some(score));
}

#[test]
fn dynamic_list_notifications_use_resolved_descriptor_index() {
    let descriptor = reversed_dynamic_descriptor();
    let list = DynamicListVariableSlot::new(EntityClassId(1), VariableId(0), "Vehicle", "visits")
        .resolved_against(&descriptor)
        .expect("dynamic list slot should resolve against logical IDs");
    assert_eq!(list.descriptor_index(), 0);

    let plan = DynamicPlan {
        score: None,
        scalar_values: Vec::new(),
        scalar_candidates: Vec::new(),
        lists: vec![vec![1, 0]],
        list_element_count: 2,
    };
    let mut director = ScoreDirector::with_descriptor(
        plan,
        PreferOrderedVisits::for_descriptor_index(0),
        descriptor,
        |solution, descriptor_index| match descriptor_index {
            0 => solution.lists.len(),
            1 => solution.scalar_values.len(),
            _ => 0,
        },
    );
    assert_eq!(director.calculate_score(), SoftScore::of(-10));

    let mov = crate::heuristic::r#move::DynamicListChangeMove::new(list, 0, 1, 0, 0);
    crate::heuristic::r#move::Move::do_move(&mov, &mut director);

    let score = director.calculate_score();
    assert_eq!(score, SoftScore::of(0));
    assert_eq!(Director::fresh_score(&director), Some(score));
}

#[test]
fn dynamic_list_change_allows_intra_list_tail_destination() {
    use crate::{Move, MoveSelector};

    let descriptor = dynamic_descriptor();
    let list = DynamicListVariableSlot::new(EntityClassId(1), VariableId(0), "Vehicle", "visits")
        .resolved_against(&descriptor)
        .expect("dynamic list slot should resolve against logical IDs");
    let plan = DynamicPlan {
        score: None,
        scalar_values: Vec::new(),
        scalar_candidates: Vec::new(),
        lists: vec![vec![0, 1, 2]],
        list_element_count: 3,
    };
    let mut director = dynamic_director(plan);

    let mov = crate::heuristic::r#move::DynamicListChangeMove::new(list.clone(), 0, 0, 0, 3);
    assert!(mov.is_doable(&director));
    mov.do_move(&mut director);
    assert_eq!(director.working_solution().lists, vec![vec![1, 2, 0]]);

    let selector = crate::DynamicListChangeMoveSelector::new(list);
    let director = dynamic_director(DynamicPlan {
        score: None,
        scalar_values: Vec::new(),
        scalar_candidates: Vec::new(),
        lists: vec![vec![0, 1, 2]],
        list_element_count: 3,
    });
    let moves = selector
        .iter_moves(&director)
        .map(|mov| {
            (
                mov.source_entity_index(),
                mov.source_position(),
                mov.dest_entity_index(),
                mov.dest_position(),
            )
        })
        .collect::<Vec<_>>();

    assert!(moves.contains(&(0, 0, 0, 3)));
    assert_eq!(selector.size(&director), moves.len());
    for mov in selector.iter_moves(&director) {
        assert!(mov.is_doable(&director), "generated move should be doable: {mov:?}");
    }
}
