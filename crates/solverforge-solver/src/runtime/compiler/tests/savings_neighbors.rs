use super::*;

fn neighbors(_: &Plan, _: usize, values: &[usize], k: usize, out: &mut Vec<usize>) {
    let stride = k.min(values.len().saturating_sub(1));
    for index in 0..values.len() {
        out.extend(
            (0..values.len())
                .filter(|&other| other != index)
                .take(stride),
        );
    }
}

fn compile_bounded(
    model: RuntimeModel<Plan, usize, Meter, Meter>,
    construction_heuristic_type: ConstructionHeuristicType,
    savings_neighbors: Option<u32>,
) -> Result<(), super::super::RuntimeCompileError> {
    let config = SolverConfig {
        phases: vec![PhaseConfig::ConstructionHeuristic(
            ConstructionHeuristicConfig {
                construction_heuristic_type,
                savings_neighbors: savings_neighbors
                    .map(|k| std::num::NonZeroU32::new(k).expect("test bounds are positive")),
                ..ConstructionHeuristicConfig::default()
            },
        )],
        ..SolverConfig::default()
    };
    let context = SearchContext::new(descriptor(), model, config.resolved_random_seed())
        .expect("search context should bind");
    compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .map(|_| ())
}

fn neighbor_model() -> RuntimeModel<Plan, usize, Meter, Meter> {
    RuntimeModel::new(vec![VariableSlot::List(
        savings_slot().with_savings_neighbors_fn(Some(neighbors)),
    )])
}

#[test]
fn bounded_clarke_wright_requires_a_savings_neighbour_source() {
    let error = compile_bounded(
        savings_model(),
        ConstructionHeuristicType::ListClarkeWright,
        Some(8),
    )
    .expect_err("bounded savings need a neighbour hook");
    assert!(matches!(
        error.kind,
        RuntimeCompileErrorKind::MissingCapability {
            capability: RuntimeCapability::ListSavingsNeighbors,
            ..
        }
    ));
    compile_bounded(
        savings_model(),
        ConstructionHeuristicType::ListClarkeWright,
        None,
    )
    .expect("unbounded savings need no neighbour hook");
    compile_bounded(
        neighbor_model(),
        ConstructionHeuristicType::ListClarkeWright,
        Some(8),
    )
    .expect("a declared neighbour hook admits bounded savings");
}

#[test]
fn savings_neighbors_is_a_positive_clarke_wright_setting() {
    for heuristic in [
        ConstructionHeuristicType::ListCheapestInsertion,
        ConstructionHeuristicType::FirstFit,
    ] {
        let error = compile_bounded(neighbor_model(), heuristic, Some(8))
            .expect_err("savings_neighbors on another heuristic");
        assert_eq!(
            error.path, "phases[0].construction.savings_neighbors",
            "{error}"
        );
        assert!(
            matches!(error.kind, RuntimeCompileErrorKind::InvalidValue { .. }),
            "{error}"
        );
    }
    let zero = SolverConfig::from_toml_str(
        "[[phases]]\ntype = \"construction_heuristic\"\n\
         construction_heuristic_type = \"list_clarke_wright\"\nsavings_neighbors = 0\n",
    );
    assert!(zero.is_err(), "zero savings_neighbors must not parse");
}
