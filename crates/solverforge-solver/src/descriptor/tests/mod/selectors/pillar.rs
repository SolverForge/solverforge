
#[test]
fn descriptor_pillar_change_uses_public_pillar_semantics() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::PillarChangeMoveSelector(PillarChangeMoveConfig {
        selection_order: None,
        selection_metric: None,
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        value_candidate_limit: None,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 2);
    assert_eq!(moves.len(), 2);
    assert!(moves
        .iter()
        .all(|mov| matches!(mov, super::DescriptorMoveUnion::PillarChange(_))));
}

#[test]
fn descriptor_pillar_change_intersects_entity_domains() {
    let descriptor = restricted_descriptor();
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    };
    let mut director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::PillarChangeMoveSelector(PillarChangeMoveConfig {
        selection_order: None,
        selection_metric: None,
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        value_candidate_limit: None,
        target: VariableTargetConfig::default(),
    });

    let selector =
        build_descriptor_move_selector::<RestrictedPlan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 1);
    assert!(moves[0].is_doable(&director));
    moves[0].do_move(&mut director);
    assert_eq!(director.working_solution().tasks[0].worker_idx, Some(2));
    assert_eq!(director.working_solution().tasks[1].worker_idx, Some(2));
}

#[test]
fn descriptor_pillar_swap_prunes_illegal_partners() {
    let descriptor = restricted_descriptor();
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
            RestrictedTask {
                worker_idx: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
            RestrictedTask {
                worker_idx: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::PillarSwapMoveSelector(PillarSwapMoveConfig {
        selection_order: None,
        selection_metric: None,
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        target: VariableTargetConfig::default(),
    });

    let selector =
        build_descriptor_move_selector::<RestrictedPlan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 2);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn descriptor_pillar_swap_tabu_identity_is_direction_stable() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker],
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let mut director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::PillarSwapMoveSelector(PillarSwapMoveConfig {
        selection_order: None,
        selection_metric: None,
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        target: VariableTargetConfig::default(),
    });
    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let mut moves: Vec<_> = selector.iter_moves(&director).collect();
    assert_eq!(moves.len(), 1);
    let forward = moves
        .pop()
        .expect("descriptor pillar swap should be generated");
    let forward_signature = forward.tabu_signature(&director);

    forward.do_move(&mut director);

    let reverse_selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let mut reverse_moves: Vec<_> = reverse_selector.iter_moves(&director).collect();
    assert_eq!(reverse_moves.len(), 1);
    let reverse = reverse_moves
        .pop()
        .expect("reverse descriptor pillar swap should be generated");
    let reverse_signature = reverse.tabu_signature(&director);

    assert_eq!(forward_signature.move_id, *forward_signature.undo_id());
    assert_eq!(forward_signature.move_id, reverse_signature.move_id);
}

#[test]
fn descriptor_pillar_swap_reports_both_pillars_through_union() {
    let mut descriptor = restricted_descriptor();
    descriptor.entity_descriptors[0] =
        descriptor.entity_descriptors[0]
            .clone()
            .with_pin_predicate(|entity| {
                entity.downcast_ref::<RestrictedTask>().unwrap().worker_idx == Some(1)
            });
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker],
        tasks: [0, 0, 1, 1]
            .into_iter()
            .map(|value| RestrictedTask {
                worker_idx: Some(value),
                allowed_workers: vec![0, 1],
            })
            .collect(),
        score: None,
    };
    let mut director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let binding = super::bindings::collect_bindings(&descriptor).remove(0);
    let mov = super::DescriptorPillarSwapMove::new(binding, vec![0, 1], vec![2, 3], descriptor);
    let mut direct = Vec::new();
    mov.for_each_affected_entity(&mut |entity| {
        direct.push((
            entity.descriptor_index,
            entity.entity_index,
            entity.variable_name.to_owned(),
        ))
    });
    assert_eq!(
        direct.iter().map(|(_, row, _)| *row).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    let mov = super::DescriptorMoveUnion::PillarSwap(mov);
    let mut wrapped = Vec::new();
    mov.for_each_affected_entity(&mut |entity| {
        wrapped.push((
            entity.descriptor_index,
            entity.entity_index,
            entity.variable_name.to_owned(),
        ))
    });
    assert_eq!(wrapped, direct);
    assert!(
        crate::pinning::move_changes_pinned(&mov, &director),
        "a pinned right-pillar row must reject the move before evaluation"
    );
    assert!(mov.is_doable(&director));
    let undo = mov.do_move(&mut director);
    assert_eq!(
        director
            .working_solution()
            .tasks
            .iter()
            .map(|task| task.worker_idx)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(1), Some(0), Some(0)]
    );
    mov.undo_move(&mut director, undo);
    assert_eq!(
        director
            .working_solution()
            .tasks
            .iter()
            .map(|task| task.worker_idx)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(0), Some(1), Some(1)]
    );
}

#[test]
fn descriptor_manual_illegal_pillar_moves_are_not_doable() {
    let descriptor = restricted_descriptor();
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let binding = super::bindings::collect_bindings(&descriptor)
        .into_iter()
        .next()
        .expect("restricted descriptor binding");

    let illegal_change = super::DescriptorPillarChangeMove::new(
        binding.clone(),
        vec![0, 1],
        Some(1),
        descriptor.clone(),
    );
    let illegal_swap =
        super::DescriptorPillarSwapMove::new(binding, vec![0, 1], vec![2, 3], descriptor);

    assert!(!illegal_change.is_doable(&director));
    assert!(!illegal_swap.is_doable(&director));
}
