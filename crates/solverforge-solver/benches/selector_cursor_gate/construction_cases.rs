fn construction_placer(
) -> QueuedEntityPlacer<Plan, usize, FromSolutionEntitySelector, StaticValueSelector<Plan, usize>> {
    QueuedEntityPlacer::new(
        FromSolutionEntitySelector::new(0),
        StaticValueSelector::new((10..74).collect::<Vec<_>>()),
        scalar_get,
        scalar_set,
        0,
        0,
        "first",
    )
}

#[cfg(not(feature = "candidate"))]
fn construction_full() {
    emit("construction_full", 8_192, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            let director = unassigned_director(128);
            for placement in construction_placer().get_placements(&director) {
                for mov in placement.moves {
                    mix_move_identity(&mut hash, &mov.tabu_signature(&director).move_id);
                    black_box(&mov);
                    count += 1;
                }
            }
        }
        (count, hash)
    });
}

#[cfg(feature = "candidate")]
fn construction_full() {
    emit("construction_full", 8_192, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            let director = unassigned_director(128);
            let placer = construction_placer();
            let mut placements = placer.open_cursor(&director);
            while let Some(mut placement) =
                placements.next_placement(&director, |_| false, || false)
            {
                while let Some(candidate_id) = placement.candidates_mut().next_candidate() {
                    let mov = placement
                        .candidates()
                        .candidate(candidate_id)
                        .expect("construction candidate must remain live");
                    mix_move_identity(&mut hash, &mov.tabu_signature(&director).move_id);
                    black_box(&mov);
                    drop(mov);
                    assert!(placement.candidates_mut().release_candidate(candidate_id));
                    count += 1;
                }
            }
        }
        (count, hash)
    });
}

fn construction_first_fit() {
    let expected_generated = if cfg!(feature = "candidate") {
        128
    } else {
        8_192
    };
    emit("construction_first_fit", expected_generated, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            let placer = construction_placer();
            let forager = FirstFitForager::<Plan, ChangeMove<Plan, usize>>::new();
            let mut phase = ConstructionHeuristicPhase::new(placer, forager);
            let mut scope = SolverScope::new(unassigned_director(128));
            scope.start_solving();
            phase.solve(&mut scope);
            count += scope.stats().moves_generated as usize;
            for vehicle in &scope.working_solution().vehicles {
                mix(&mut hash, vehicle.visits[0]);
            }
        }
        (count, hash)
    });
}
