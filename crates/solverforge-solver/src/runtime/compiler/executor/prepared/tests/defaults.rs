use super::*;

#[test]
fn fully_counted_default_rejects_duplicate_assigned_source_key() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let mut prepared = executor(&SolverConfig::default())
        .instantiate()
        .expect("structural preparation must not validate a source");
    let default = match prepared.phases.remove(0) {
        PreparedRuntimePhase::DefaultRuntime(default) => default,
        _ => panic!("omitted configuration must retain one unresolved default node"),
    };
    let input = plan(vec![7, 8], vec![vec![7, 7]]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    let error = execute_prepared_default_construction(&mut prepared, &default, &mut scope)
        .expect_err("equal raw counts must not hide a duplicate assigned source key");
    assert!(matches!(
        error.kind,
        RuntimeInstantiationErrorKind::SourceBinding {
            error: crate::builder::context::ListConstructionKernelError::DuplicateAssignedElement {
                source_index: 0,
                first_assigned_occurrence: 0,
                duplicate_assigned_occurrence: 1,
            },
            ..
        }
    ));
    assert_eq!(prepared.bound_list_source_count(), 0);
}

#[test]
fn fully_counted_default_rejects_undeclared_assigned_source_key() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let mut prepared = executor(&SolverConfig::default())
        .instantiate()
        .expect("structural preparation must not validate a source");
    let default = match prepared.phases.remove(0) {
        PreparedRuntimePhase::DefaultRuntime(default) => default,
        _ => panic!("omitted configuration must retain one unresolved default node"),
    };
    let input = plan(vec![7, 8], vec![vec![7, 9]]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    let error = execute_prepared_default_construction(&mut prepared, &default, &mut scope)
        .expect_err("equal raw counts must not hide an undeclared assigned source key");
    assert!(matches!(
        error.kind,
        RuntimeInstantiationErrorKind::SourceBinding {
            error:
                crate::builder::context::ListConstructionKernelError::AssignedElementNotDeclared {
                    assigned_occurrence: 1,
                },
            ..
        }
    ));
    assert_eq!(prepared.bound_list_source_count(), 0);
}

#[test]
fn valid_fully_assigned_default_binds_once_and_skips_list_work() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let mut prepared = executor(&SolverConfig::default())
        .instantiate()
        .expect("structural preparation must not validate a source");
    let default = match prepared.phases.remove(0) {
        PreparedRuntimePhase::DefaultRuntime(default) => default,
        _ => panic!("omitted configuration must retain one unresolved default node"),
    };
    let input = plan(vec![7, 8], vec![vec![7, 8]]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    let execution = execute_prepared_default_construction(&mut prepared, &default, &mut scope)
        .expect("a valid complete source should bind and report no list-construction work");
    let list_stage = execution
        .stages
        .iter()
        .find(|stage| {
            stage.stage
                == crate::runtime::compiler::DefaultConstructionStage::Preconstruction(
                    DefaultPreconstructionStage::ListConstruction,
                )
        })
        .expect("default execution records the list-construction boundary");
    assert_eq!(
        list_stage.outcome,
        ResolvedConstructionExecutionOutcome::SkippedNoWork
    );
    assert_eq!(prepared.bound_list_source_count(), 1);
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 4);
}

#[test]
fn staged_default_prepares_clarke_wright_from_the_structural_catalog() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let executor = executor(&SolverConfig::default());
    let prepared = executor
        .instantiate()
        .expect("default source registration is structural");
    let phase_index = match prepared.phases.as_slice() {
        [PreparedRuntimePhase::DefaultRuntime(default)] => default.phase_index,
        _ => panic!("omitted configuration must remain one unresolved default node"),
    };
    let input = plan(vec![1, 2, 3], vec![Vec::new()]);
    let staged = super::super::super::super::defaults::resolve_default_preconstruction_stage(
        executor.graph().default_bindings(),
        DefaultPreconstructionStage::ListConstruction,
        &input,
    );
    let [step] = staged.steps.as_slice() else {
        panic!("savings-capable default must choose exactly one CW child");
    };

    let resolved = prepared
        .prepare_resolved_construction(phase_index, &step.construction)
        .expect("staged CW resolves the structural source catalog");
    assert!(matches!(
        resolved,
        PreparedConstruction::ClarkeWright { ref slots, .. }
            if slots.len() == 1 && slots[0].catalog_index == 0
    ));
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn staged_default_executes_canonical_clarke_wright_at_the_reached_boundary() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let executor = executor(&SolverConfig::default());
    let mut prepared = executor
        .instantiate()
        .expect("default source registration is structural");
    let default = match prepared.phases.remove(0) {
        PreparedRuntimePhase::DefaultRuntime(default) => default,
        _ => panic!("omitted configuration must retain one staged default node"),
    };
    let input = plan(vec![1, 2, 3], vec![Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    assert!(
        execute_prepared_default_construction(&mut prepared, &default, &mut scope)
            .expect("reached default CW binds and executes")
            .ran_child_phase
    );
    assert_eq!(scope.working_solution().routes.iter().flatten().count(), 3);
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 3);
    assert_eq!(prepared.bound_list_source_count(), 1);
}

#[derive(Debug)]
struct Marker;

impl CustomSearchPhase<Plan> for Marker {
    fn solve<D, ProgressCb>(&mut self, _: &mut SolverScope<'_, Plan, D, ProgressCb>)
    where
        D: solverforge_scoring::Director<Plan>,
        ProgressCb: ProgressCallback<Plan>,
    {
    }
}

fn build_marker(_: &SearchContext<Plan, usize, Meter, Meter>) -> Marker {
    EXTENSION_BUILD_CALLS.fetch_add(1, Ordering::SeqCst);
    Marker
}

#[test]
fn eager_extension_preparation_precedes_lazy_source_binding() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    EXTENSION_BUILD_CALLS.store(0, Ordering::SeqCst);
    let config = SolverConfig {
        phases: vec![
            PhaseConfig::Custom(CustomPhaseConfig {
                name: "marker".to_string(),
            }),
            construction(ConstructionHeuristicType::ListCheapestInsertion),
        ],
        ..SolverConfig::default()
    };
    let context = SearchContext::new(descriptor(), model(), config.resolved_random_seed())
        .expect("search context should bind");
    let input = RuntimeGraphInput::new(
        context,
        CustomPhaseNode::new(NoTypedExtensions, "marker", build_marker),
    );
    let graph = compile_runtime_graph(&config, input).expect("extension name is declared");
    let mut prepared = CompiledRuntimeExecutor::new(graph)
        .instantiate()
        .expect("source binding belongs to the reached construction boundary");
    assert_eq!(EXTENSION_BUILD_CALLS.load(Ordering::SeqCst), 1);
    let construction = match prepared.phases.remove(1) {
        PreparedRuntimePhase::Construction(construction) => construction,
        _ => panic!("configured construction should remain after eager extension preparation"),
    };
    let input = plan(vec![9, 9], vec![Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);
    let error = execute_prepared_construction(&mut prepared, &construction, false, &mut scope)
        .expect_err("the reached malformed source must fail at its own boundary");

    assert!(matches!(
        error.kind,
        RuntimeInstantiationErrorKind::SourceBinding { .. }
    ));
    assert_eq!(EXTENSION_BUILD_CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn optional_construction_keeps_its_share_of_a_time_limited_solve() {
    use crate::runtime::compiler::executor::default_construction::optional_construction_termination;

    let input = plan(vec![7], vec![Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);
    assert!(
        optional_construction_termination(&scope).is_none(),
        "a solve without a time limit leaves the optional pass unbounded"
    );
    scope.set_time_limit(std::time::Duration::from_secs(60));
    assert_eq!(
        optional_construction_termination(&scope).and_then(|config| config.seconds_spent_limit),
        Some(12),
        "a fifth of a 60 s solve"
    );
    scope.set_time_limit(std::time::Duration::from_secs(4));
    assert_eq!(
        optional_construction_termination(&scope).and_then(|config| config.seconds_spent_limit),
        Some(1),
        "the 0.8 s share rounds up to whole seconds"
    );
}

#[test]
fn optional_construction_in_a_partition_shares_the_inherited_deadline() {
    use crate::runtime::compiler::executor::default_construction::optional_construction_termination;

    let director =
        ScoreDirector::simple(plan(vec![7], vec![Vec::new()]), descriptor(), |plan, _| {
            entity_count(plan)
        });
    let mut parent = SolverScope::new(director);
    parent.set_time_limit(std::time::Duration::from_secs(60));
    parent.start_solving();
    let budget = parent.child_phase_budget();
    let child_director =
        ScoreDirector::simple(plan(vec![7], vec![Vec::new()]), descriptor(), |plan, _| {
            entity_count(plan)
        });
    let mut child = parent
        .child_config(Some(&budget))
        .build_scope(child_director, 7);
    child.start_solving();
    assert_eq!(
        optional_construction_termination(&child).and_then(|config| config.seconds_spent_limit),
        Some(12),
        "a fifth of the 60 s the partition inherits"
    );
}
