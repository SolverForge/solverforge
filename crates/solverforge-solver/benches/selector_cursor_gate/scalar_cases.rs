/* Scalar neighborhoods: each compiles one scalar move-selector
configuration into the scalar neighborhood leaf
(`RuntimeScalarNeighborhoodCursor` under the runtime flat and composition
cursors); `cartesian` composes two such leaves in the compiled Cartesian
composition. */

fn scalar_change() {
    let plan = routed_plan(128, 1);
    neighborhood_case(
        "scalar_change",
        8_192,
        scalar_director(plan, 1),
        compile_neighborhood(
            scalar_context(vec![first_slot(10..74)]),
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                selection_order: CANONICAL,
                target: target("first"),
                ..ChangeMoveConfig::default()
            }),
        ),
    );
}

/* Every entity's value lies inside a 4,096-value range, so every pair of
distinct values is a legal exchange. The range dwarfs the entity count, which
is the shape where opening the cursor must not scale with the value range. */
fn scalar_swap() {
    neighborhood_case(
        "scalar_swap",
        18_336,
        scalar_director(routed_plan(192, 1), 1),
        compile_neighborhood(
            scalar_context(vec![first_slot(0..4_096)]),
            MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
                selection_order: CANONICAL,
                target: target("first"),
                ..SwapMoveConfig::default()
            }),
        ),
    );
}

/* The value ranges lie outside the plan's current values, so every right
leg stays doable under its left leg's preview. */
fn cartesian() {
    let change = |variable_name: &str| {
        MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
            selection_order: CANONICAL,
            target: target(variable_name),
            ..ChangeMoveConfig::default()
        })
    };
    neighborhood_case(
        "cartesian",
        576,
        scalar_director(routed_plan(12, 4), 2),
        compile_neighborhood(
            scalar_context(vec![first_slot(100..102), second_slot(200..202)]),
            MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
                require_hard_improvement: false,
                selectors: vec![change("first"), change("second")],
            }),
        ),
    );
}

fn ruin() {
    neighborhood_case(
        "ruin",
        4_096,
        scalar_director(routed_plan(4_096, 1), 1),
        compile_neighborhood(
            scalar_context(vec![first_slot(0..4_096)]),
            MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
                min_ruin_count: 4,
                max_ruin_count: 8,
                moves_per_step: Some(4_096),
                selection_order: CANONICAL,
                target: target("first"),
                ..RuinRecreateMoveSelectorConfig::default()
            }),
        ),
    );
}
