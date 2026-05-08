#[derive(Clone, Debug)]
struct CoupledScalarChoice {
    first: Option<usize>,
    second: Option<usize>,
    third: Option<usize>,
}

#[derive(Clone, Debug)]
struct CoupledScalarPlan {
    score: Option<HardSoftScore>,
    choices: Vec<CoupledScalarChoice>,
}

#[derive(Clone, Debug)]
struct CoupledScalarDirector {
    working_solution: CoupledScalarPlan,
    descriptor: SolutionDescriptor,
}

impl PlanningSolution for CoupledScalarPlan {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Director<CoupledScalarPlan> for CoupledScalarDirector {
    fn working_solution(&self) -> &CoupledScalarPlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut CoupledScalarPlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> HardSoftScore {
        let choice = &self.working_solution.choices[0];
        let score = match (choice.first, choice.second, choice.third) {
            (Some(1), Some(1), Some(1)) => HardSoftScore::of(0, 0),
            (None, None, None) => HardSoftScore::of(-1, 0),
            _ => HardSoftScore::of(-2, 0),
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> CoupledScalarPlan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.choices.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.choices.len())
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

fn coupled_plan_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("CoupledScalarPlan", TypeId::of::<CoupledScalarPlan>()).with_entity(
        EntityDescriptor::new(
            "CoupledScalarChoice",
            TypeId::of::<CoupledScalarChoice>(),
            "choices",
        )
        .with_extractor(Box::new(EntityCollectionExtractor::new(
            "CoupledScalarChoice",
            "choices",
            |solution: &CoupledScalarPlan| &solution.choices,
            |solution: &mut CoupledScalarPlan| &mut solution.choices,
        )))
        .with_variable(scalar_choice_variable(
            "first",
            coupled_get_first,
            coupled_set_first,
        ))
        .with_variable(scalar_choice_variable(
            "second",
            coupled_get_second,
            coupled_set_second,
        ))
        .with_variable(scalar_choice_variable(
            "third",
            coupled_get_third,
            coupled_set_third,
        )),
    )
}

fn scalar_choice_variable(
    variable_name: &'static str,
    getter: fn(&dyn std::any::Any) -> Option<usize>,
    setter: fn(&mut dyn std::any::Any, Option<usize>),
) -> VariableDescriptor {
    VariableDescriptor::genuine(variable_name)
        .with_allows_unassigned(true)
        .with_value_range_type(solverforge_core::domain::ValueRangeType::CountableRange {
            from: 0,
            to: 2,
        })
        .with_usize_accessors(getter, setter)
}

fn coupled_choice(entity: &dyn std::any::Any) -> &CoupledScalarChoice {
    entity
        .downcast_ref::<CoupledScalarChoice>()
        .expect("coupled choice expected")
}

fn coupled_choice_mut(entity: &mut dyn std::any::Any) -> &mut CoupledScalarChoice {
    entity
        .downcast_mut::<CoupledScalarChoice>()
        .expect("coupled choice expected")
}

fn coupled_get_first(entity: &dyn std::any::Any) -> Option<usize> {
    coupled_choice(entity).first
}

fn coupled_set_first(entity: &mut dyn std::any::Any, value: Option<usize>) {
    coupled_choice_mut(entity).first = value;
}

fn coupled_get_second(entity: &dyn std::any::Any) -> Option<usize> {
    coupled_choice(entity).second
}

fn coupled_set_second(entity: &mut dyn std::any::Any, value: Option<usize>) {
    coupled_choice_mut(entity).second = value;
}

fn coupled_get_third(entity: &dyn std::any::Any) -> Option<usize> {
    coupled_choice(entity).third
}

fn coupled_set_third(entity: &mut dyn std::any::Any, value: Option<usize>) {
    coupled_choice_mut(entity).third = value;
}

fn coupled_scalar_model(
    with_group: bool,
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
    let model = RuntimeModel::new(variables);
    if !with_group {
        return model;
    }

    model.with_scalar_groups(bind_scalar_groups(
        vec![ScalarGroup::candidates(
            "coupled_assignment",
            vec![
                ScalarTarget::from_descriptor_index(0, "first"),
                ScalarTarget::from_descriptor_index(0, "second"),
                ScalarTarget::from_descriptor_index(0, "third"),
            ],
            coupled_group_candidates,
        )],
        &scalar_slots,
    ))
}

fn coupled_group_candidates(
    plan: &CoupledScalarPlan,
    _limits: ScalarGroupLimits,
) -> Vec<ScalarCandidate<CoupledScalarPlan>> {
    if plan.choices.is_empty() {
        return Vec::new();
    }
    vec![ScalarCandidate::new(
        "witness",
        vec![
            ScalarTarget::from_descriptor_index(0, "first").set(0, Some(1)),
            ScalarTarget::from_descriptor_index(0, "second").set(0, Some(1)),
            ScalarTarget::from_descriptor_index(0, "third").set(0, Some(1)),
        ],
    )]
}

fn coupled_empty_plan() -> CoupledScalarPlan {
    CoupledScalarPlan {
        score: None,
        choices: vec![CoupledScalarChoice {
            first: None,
            second: None,
            third: None,
        }],
    }
}

#[test]
fn coupled_scalar_witness_is_hard_feasible_only_as_compound_assignment() {
    let mut empty = CoupledScalarDirector {
        working_solution: coupled_empty_plan(),
        descriptor: coupled_plan_descriptor(),
    };
    assert_eq!(empty.calculate_score(), HardSoftScore::of(-1, 0));

    for variable_index in 0..3 {
        let mut one_edit = coupled_empty_plan();
        match variable_index {
            0 => one_edit.choices[0].first = Some(1),
            1 => one_edit.choices[0].second = Some(1),
            2 => one_edit.choices[0].third = Some(1),
            _ => unreachable!(),
        }
        let mut director = CoupledScalarDirector {
            working_solution: one_edit,
            descriptor: coupled_plan_descriptor(),
        };
        assert!(director.calculate_score() < HardSoftScore::of(-1, 0));
    }

    let mut witness = CoupledScalarDirector {
        working_solution: CoupledScalarPlan {
            score: None,
            choices: vec![CoupledScalarChoice {
                first: Some(1),
                second: Some(1),
                third: Some(1),
            }],
        },
        descriptor: coupled_plan_descriptor(),
    };
    assert_eq!(witness.calculate_score(), HardSoftScore::of(0, 0));
}

#[test]
fn scalar_construction_is_order_local_for_coupled_nullable_slots() {
    let descriptor = coupled_plan_descriptor();
    let director = CoupledScalarDirector {
        working_solution: coupled_empty_plan(),
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(ConstructionHeuristicConfig {
            construction_heuristic_type: ConstructionHeuristicType::FirstFit,
            construction_obligation: ConstructionObligation::AssignWhenCandidateExists,
            ..ConstructionHeuristicConfig::default()
        }),
        descriptor,
        coupled_scalar_model(false),
    );
    phase.solve(&mut solver_scope);

    let choice = &solver_scope.working_solution().choices[0];
    assert_eq!(choice.first, Some(0));
    assert_eq!(choice.second, Some(0));
    assert_eq!(choice.third, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(-2, 0))
    );
}

#[test]
fn grouped_scalar_construction_reaches_coupled_hard_witness() {
    let descriptor = coupled_plan_descriptor();
    let director = CoupledScalarDirector {
        working_solution: coupled_empty_plan(),
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(ConstructionHeuristicConfig {
            construction_heuristic_type: ConstructionHeuristicType::FirstFit,
            construction_obligation: ConstructionObligation::AssignWhenCandidateExists,
            group_name: Some("coupled_assignment".to_string()),
            ..ConstructionHeuristicConfig::default()
        }),
        descriptor,
        coupled_scalar_model(true),
    );
    phase.solve(&mut solver_scope);

    let choice = &solver_scope.working_solution().choices[0];
    assert_eq!(choice.first, Some(1));
    assert_eq!(choice.second, Some(1));
    assert_eq!(choice.third, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
}
