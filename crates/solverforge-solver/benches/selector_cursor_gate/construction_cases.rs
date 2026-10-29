/* Scalar construction through the compiled runtime.

`construction_full` enumerates every placement and candidate of the
descriptor-placement placer a compiled first-fit construction phase drives
(`RuntimeScalarConstructionPlacer` and its placement cursors).
`construction_first_fit` solves with the compiled runtime phase runner
itself; runners are compiled before the timed region, so it times the solve
a configured construction phase runs. */

fn construction_config() -> SolverConfig {
    solver_config(PhaseConfig::ConstructionHeuristic(
        ConstructionHeuristicConfig {
            construction_heuristic_type: ConstructionHeuristicType::FirstFit,
            ..ConstructionHeuristicConfig::default()
        },
    ))
}

fn construction_runner() -> Runner {
    Runner::compile(
        &construction_config(),
        scalar_context(vec![first_slot(10..74)]),
    )
    .expect("the gate construction configuration must compile")
}

fn construction_full() {
    let placer = construction_runner()
        .into_scalar_construction_placer()
        .expect("the gate configuration's first phase is a scalar construction");
    let order_director = scalar_director(unassigned_plan(128), 1);
    let mut order_hash = FNV_OFFSET;
    let mut placements = placer.open_cursor(&order_director);
    while let Some(mut placement) = placements.next_placement(&order_director) {
        let solution = order_director.working_solution();
        while let Some(candidate_id) = placement.next_candidate(solution) {
            let (entity_index, value) = placement
                .candidate_assignment(candidate_id)
                .expect("construction candidate must remain live");
            mix(&mut order_hash, entity_index);
            mix(&mut order_hash, value);
            assert!(placement.release_candidate(candidate_id));
        }
    }
    emit("construction_full", 8_192, || {
        let mut count = 0;
        let mut identity = FNV_OFFSET;
        for _ in 0..iterations() {
            let director = scalar_director(unassigned_plan(128), 1);
            let mut placements = placer.open_cursor(&director);
            while let Some(mut placement) = placements.next_placement(&director) {
                let solution = director.working_solution();
                while let Some(candidate_id) = placement.next_candidate(solution) {
                    placement
                        .visit_candidate(candidate_id, EntityFold(&mut identity))
                        .expect("construction candidate must remain live");
                    assert!(placement.release_candidate(candidate_id));
                    count += 1;
                }
            }
        }
        (count, order_hash, identity)
    });
}

fn construction_first_fit() {
    let expected_generated = 128;
    let mut runners = (0..iterations())
        .map(|_| construction_runner())
        .collect::<Vec<_>>();
    emit("construction_first_fit", expected_generated, || {
        let mut count = 0;
        let mut hash = FNV_OFFSET;
        for runner in &mut runners {
            let mut scope = SolverScope::new(scalar_director(unassigned_plan(128), 1));
            scope.start_solving();
            runner.solve(&mut scope);
            count += scope.stats().moves_generated() as usize;
            for vehicle in &scope.working_solution().vehicles {
                mix(&mut hash, vehicle.visits[0]);
            }
        }
        (count, hash, hash)
    });
}
