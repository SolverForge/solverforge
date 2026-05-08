fn coupled_model_with_group_provider(
    provider: crate::builder::context::ScalarCandidateProvider<CoupledScalarPlan>,
) -> RuntimeModel<CoupledScalarPlan, usize, DefaultMeter, DefaultMeter> {
    let variables = vec![
        VariableSlot::Scalar(ScalarVariableSlot::new(
            0,
            0,
            "CoupledScalarChoice",
            |solution: &CoupledScalarPlan| solution.choices.len(),
            "first",
            |solution, entity_index, _variable_index| solution.choices[entity_index].first,
            |solution, entity_index, _variable_index, value| {
                solution.choices[entity_index].first = value;
            },
            ValueSource::CountableRange { from: 0, to: 2 },
            true,
        )),
        VariableSlot::Scalar(ScalarVariableSlot::new(
            0,
            1,
            "CoupledScalarChoice",
            |solution: &CoupledScalarPlan| solution.choices.len(),
            "second",
            |solution, entity_index, _variable_index| solution.choices[entity_index].second,
            |solution, entity_index, _variable_index, value| {
                solution.choices[entity_index].second = value;
            },
            ValueSource::CountableRange { from: 0, to: 2 },
            true,
        )),
        VariableSlot::Scalar(ScalarVariableSlot::new(
            0,
            2,
            "CoupledScalarChoice",
            |solution: &CoupledScalarPlan| solution.choices.len(),
            "third",
            |solution, entity_index, _variable_index| solution.choices[entity_index].third,
            |solution, entity_index, _variable_index, value| {
                solution.choices[entity_index].third = value;
            },
            ValueSource::CountableRange { from: 0, to: 2 },
            true,
        )),
    ];
    let scalar_slots = variables
        .iter()
        .filter_map(|variable| match variable {
            VariableSlot::Scalar(ctx) => Some(*ctx),
            VariableSlot::List(_) => None,
        })
        .collect::<Vec<_>>();

    RuntimeModel::new(variables).with_scalar_groups(bind_scalar_groups(
        vec![ScalarGroup::candidates(
            "coupled_assignment",
            vec![
                ScalarTarget::from_descriptor_index(0, "first"),
                ScalarTarget::from_descriptor_index(0, "second"),
                ScalarTarget::from_descriptor_index(0, "third"),
            ],
            provider,
        )],
        &scalar_slots,
    ))
}

fn grouped_config(
    heuristic: ConstructionHeuristicType,
    obligation: ConstructionObligation,
) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type: heuristic,
        construction_obligation: obligation,
        group_name: Some("coupled_assignment".to_string()),
        ..ConstructionHeuristicConfig::default()
    }
}

fn coupled_edit_candidate(reason: &'static str, value: usize) -> ScalarCandidate<CoupledScalarPlan> {
    coupled_edit_candidate_for_entity(reason, 0, value)
}

fn coupled_edit_candidate_for_entity(
    reason: &'static str,
    entity_index: usize,
    value: usize,
) -> ScalarCandidate<CoupledScalarPlan> {
    ScalarCandidate::new(
        reason,
        vec![
            ScalarTarget::from_descriptor_index(0, "first").set(entity_index, Some(value)),
            ScalarTarget::from_descriptor_index(0, "second").set(entity_index, Some(value)),
            ScalarTarget::from_descriptor_index(0, "third").set(entity_index, Some(value)),
        ],
    )
    .with_construction_slot_key(entity_index)
}

fn worse_then_better_group_candidates(
    _plan: &CoupledScalarPlan,
    _limits: ScalarGroupLimits,
) -> Vec<ScalarCandidate<CoupledScalarPlan>> {
    vec![
        coupled_edit_candidate("worse", 0),
        coupled_edit_candidate("better", 1),
    ]
}

fn worse_only_group_candidates(
    _plan: &CoupledScalarPlan,
    _limits: ScalarGroupLimits,
) -> Vec<ScalarCandidate<CoupledScalarPlan>> {
    vec![coupled_edit_candidate("worse", 0)]
}

fn ordered_group_candidates(
    _plan: &CoupledScalarPlan,
    _limits: ScalarGroupLimits,
) -> Vec<ScalarCandidate<CoupledScalarPlan>> {
    vec![
        coupled_edit_candidate("stronger", 0).with_construction_value_order_key(10),
        coupled_edit_candidate("weaker", 1).with_construction_value_order_key(1),
    ]
}

fn assigned_then_open_group_candidates(
    _plan: &CoupledScalarPlan,
    limits: ScalarGroupLimits,
) -> Vec<ScalarCandidate<CoupledScalarPlan>> {
    assert_eq!(limits.group_candidate_limit, None);
    vec![
        coupled_edit_candidate_for_entity("assigned", 0, 0),
        coupled_edit_candidate_for_entity("open", 1, 1),
    ]
}

#[test]
fn grouped_scalar_first_fit_scans_past_worse_candidate_for_later_improvement() {
    let descriptor = coupled_plan_descriptor();
    let director = CoupledScalarDirector {
        working_solution: coupled_empty_plan(),
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(grouped_config(
            ConstructionHeuristicType::FirstFit,
            ConstructionObligation::PreserveUnassigned,
        )),
        descriptor,
        coupled_model_with_group_provider(worse_then_better_group_candidates),
    );
    phase.solve(&mut solver_scope);

    let choice = &solver_scope.working_solution().choices[0];
    assert_eq!((choice.first, choice.second, choice.third), (Some(1), Some(1), Some(1)));
    assert_eq!(solver_scope.current_score().copied(), Some(HardSoftScore::of(0, 0)));
}

#[test]
fn grouped_scalar_keep_current_marks_scalar_slots_complete() {
    let descriptor = coupled_plan_descriptor();
    let director = CoupledScalarDirector {
        working_solution: coupled_empty_plan(),
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut grouped_phase = Construction::new(
        Some(grouped_config(
            ConstructionHeuristicType::FirstFit,
            ConstructionObligation::PreserveUnassigned,
        )),
        descriptor.clone(),
        coupled_model_with_group_provider(worse_only_group_candidates),
    );
    grouped_phase.solve(&mut solver_scope);

    let mut scalar_phase = Construction::new(
        Some(ConstructionHeuristicConfig {
            construction_heuristic_type: ConstructionHeuristicType::FirstFit,
            construction_obligation: ConstructionObligation::AssignWhenCandidateExists,
            ..ConstructionHeuristicConfig::default()
        }),
        descriptor,
        coupled_scalar_model(false),
    );
    scalar_phase.solve(&mut solver_scope);

    let choice = &solver_scope.working_solution().choices[0];
    assert_eq!((choice.first, choice.second, choice.third), (None, None, None));
}

#[test]
fn grouped_scalar_construction_skips_already_assigned_slots() {
    let descriptor = coupled_plan_descriptor();
    let director = CoupledScalarDirector {
        working_solution: CoupledScalarPlan {
            score: None,
            choices: vec![CoupledScalarChoice {
                first: Some(1),
                second: Some(1),
                third: Some(1),
            }],
        },
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(grouped_config(
            ConstructionHeuristicType::FirstFit,
            ConstructionObligation::AssignWhenCandidateExists,
        )),
        descriptor,
        coupled_model_with_group_provider(worse_only_group_candidates),
    );
    phase.solve(&mut solver_scope);

    let choice = &solver_scope.working_solution().choices[0];
    assert_eq!((choice.first, choice.second, choice.third), (Some(1), Some(1), Some(1)));
}

#[test]
fn grouped_scalar_construction_applies_group_limit_after_frontier_filtering() {
    let descriptor = coupled_plan_descriptor();
    let director = CoupledScalarDirector {
        working_solution: CoupledScalarPlan {
            score: None,
            choices: vec![
                CoupledScalarChoice {
                    first: Some(1),
                    second: Some(1),
                    third: Some(1),
                },
                CoupledScalarChoice {
                    first: None,
                    second: None,
                    third: None,
                },
            ],
        },
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(ConstructionHeuristicConfig {
            group_candidate_limit: Some(1),
            ..grouped_config(
                ConstructionHeuristicType::FirstFit,
                ConstructionObligation::AssignWhenCandidateExists,
            )
        }),
        descriptor,
        coupled_model_with_group_provider(assigned_then_open_group_candidates),
    );
    phase.solve(&mut solver_scope);

    let choice = &solver_scope.working_solution().choices[1];
    assert_eq!((choice.first, choice.second, choice.third), (Some(1), Some(1), Some(1)));
}

#[test]
fn grouped_scalar_weakest_fit_uses_candidate_strength_key() {
    let descriptor = coupled_plan_descriptor();
    let director = CoupledScalarDirector {
        working_solution: coupled_empty_plan(),
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(grouped_config(
            ConstructionHeuristicType::WeakestFit,
            ConstructionObligation::AssignWhenCandidateExists,
        )),
        descriptor,
        coupled_model_with_group_provider(ordered_group_candidates),
    );
    phase.solve(&mut solver_scope);

    let choice = &solver_scope.working_solution().choices[0];
    assert_eq!((choice.first, choice.second, choice.third), (Some(1), Some(1), Some(1)));
}

#[test]
fn grouped_scalar_construction_applies_group_candidate_limit_separately() {
    let descriptor = coupled_plan_descriptor();
    let director = CoupledScalarDirector {
        working_solution: coupled_empty_plan(),
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(ConstructionHeuristicConfig {
            group_candidate_limit: Some(1),
            value_candidate_limit: Some(usize::MAX),
            ..grouped_config(
                ConstructionHeuristicType::FirstFit,
                ConstructionObligation::PreserveUnassigned,
            )
        }),
        descriptor,
        coupled_model_with_group_provider(worse_then_better_group_candidates),
    );
    phase.solve(&mut solver_scope);

    let choice = &solver_scope.working_solution().choices[0];
    assert_eq!((choice.first, choice.second, choice.third), (None, None, None));
}
