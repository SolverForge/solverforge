/* List neighborhoods: each compiles one list move-selector configuration
into the compiled list leaf (`RuntimeListNeighborhoodCursor` under the
runtime flat and composition cursors). */

fn list_neighborhood_case(
    case: &str,
    expected_count: usize,
    vehicle_count: usize,
    visits_per_vehicle: usize,
    move_selector: MoveSelectorConfig,
) {
    neighborhood_case(
        case,
        expected_count,
        list_director(vehicle_count, visits_per_vehicle),
        compile_neighborhood(list_context(), move_selector),
    );
}

fn list_change_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig {
        selection_order: CANONICAL,
        ..ListChangeMoveConfig::default()
    })
}

fn list_swap_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::ListSwapMoveSelector(ListSwapMoveConfig {
        selection_order: CANONICAL,
        ..ListSwapMoveConfig::default()
    })
}

fn list_change() {
    list_neighborhood_case("list_change", 85_824, 12, 24, list_change_selector());
}

fn list_swap() {
    list_neighborhood_case("list_swap", 41_328, 12, 24, list_swap_selector());
}

fn nearby_change() {
    list_neighborhood_case(
        "nearby_change",
        4_608,
        12,
        24,
        MoveSelectorConfig::NearbyListChangeMoveSelector(NearbyListChangeMoveConfig {
            max_nearby: 16,
            selection_order: CANONICAL,
            ..NearbyListChangeMoveConfig::default()
        }),
    );
}

fn nearby_swap() {
    list_neighborhood_case(
        "nearby_swap",
        4_472,
        12,
        24,
        MoveSelectorConfig::NearbyListSwapMoveSelector(NearbyListSwapMoveConfig {
            max_nearby: 16,
            selection_order: CANONICAL,
            ..NearbyListSwapMoveConfig::default()
        }),
    );
}

fn sublist_change() {
    list_neighborhood_case(
        "sublist_change",
        95_390,
        10,
        18,
        MoveSelectorConfig::SublistChangeMoveSelector(SublistChangeMoveConfig {
            min_sublist_size: 1,
            max_sublist_size: 3,
            selection_order: CANONICAL,
            ..SublistChangeMoveConfig::default()
        }),
    );
}

fn sublist_swap() {
    list_neighborhood_case(
        "sublist_swap",
        127_905,
        10,
        18,
        MoveSelectorConfig::SublistSwapMoveSelector(SublistSwapMoveConfig {
            min_sublist_size: 1,
            max_sublist_size: 3,
            selection_order: CANONICAL,
            ..SublistSwapMoveConfig::default()
        }),
    );
}

fn k_opt() {
    list_neighborhood_case(
        "k_opt",
        4_760,
        1,
        18,
        MoveSelectorConfig::KOptMoveSelector(KOptMoveSelectorConfig {
            k: 3,
            min_segment_len: 1,
            max_nearby: 0,
            selection_order: CANONICAL,
            ..KOptMoveSelectorConfig::default()
        }),
    );
}

/* A union of two list leaves: the compiled union composition schedules both
children, as the default runtime's root union does. */
fn union() {
    list_neighborhood_case(
        "union",
        127_152,
        12,
        24,
        MoveSelectorConfig::UnionMoveSelector(UnionMoveSelectorConfig {
            selectors: vec![list_change_selector(), list_swap_selector()],
            ..UnionMoveSelectorConfig::default()
        }),
    );
}
