fn list_change() {
    let director = director(12, 24);
    let selector = ListChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_get,
        list_remove,
        list_insert,
        "visits",
        0,
    );
    emit("list_change", 85_824, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            for mov in selector.iter_moves(&director) {
                mix(&mut hash, mov.source_entity_index());
                mix(&mut hash, mov.source_position());
                mix(&mut hash, mov.dest_entity_index());
                mix(&mut hash, mov.dest_position());
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}

fn list_swap() {
    let director = director(12, 24);
    let selector = ListSwapMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_get,
        list_set,
        "visits",
        0,
    );
    emit("list_swap", 41_328, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            for mov in selector.iter_moves(&director) {
                mix(&mut hash, mov.first_entity_index());
                mix(&mut hash, mov.first_position());
                mix(&mut hash, mov.second_entity_index());
                mix(&mut hash, mov.second_position());
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}

fn nearby_change() {
    let director = director(12, 24);
    let selector = NearbyListChangeMoveSelector::<Plan, usize, _, _>::new(
        FromSolutionEntitySelector::new(0),
        PositionDistanceMeter,
        16,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "visits",
        0,
    );
    emit("nearby_change", 4_608, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            for mov in selector.iter_moves(&director) {
                mix(&mut hash, mov.source_entity_index());
                mix(&mut hash, mov.source_position());
                mix(&mut hash, mov.dest_entity_index());
                mix(&mut hash, mov.dest_position());
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}

fn nearby_swap() {
    let director = director(12, 24);
    let selector = NearbyListSwapMoveSelector::<Plan, usize, _, _>::new(
        FromSolutionEntitySelector::new(0),
        PositionDistanceMeter,
        16,
        list_len,
        list_get,
        list_set,
        "visits",
        0,
    );
    emit("nearby_swap", 4_472, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            for mov in selector.iter_moves(&director) {
                mix(&mut hash, mov.first_entity_index());
                mix(&mut hash, mov.first_position());
                mix(&mut hash, mov.second_entity_index());
                mix(&mut hash, mov.second_position());
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}

fn sublist_change() {
    let director = director(10, 18);
    let selector = SublistChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        1,
        3,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );
    emit("sublist_change", 95_390, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            for mov in selector.iter_moves(&director) {
                mix(&mut hash, mov.source_entity_index());
                mix(&mut hash, mov.source_start());
                mix(&mut hash, mov.source_end());
                mix(&mut hash, mov.dest_entity_index());
                mix(&mut hash, mov.dest_position());
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}

fn sublist_swap() {
    let director = director(10, 18);
    let selector = SublistSwapMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        1,
        3,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );
    emit("sublist_swap", 127_905, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            for mov in selector.iter_moves(&director) {
                mix(&mut hash, mov.first_entity_index());
                mix(&mut hash, mov.first_start());
                mix(&mut hash, mov.first_end());
                mix(&mut hash, mov.second_entity_index());
                mix(&mut hash, mov.second_start());
                mix(&mut hash, mov.second_end());
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}
