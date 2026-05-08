use crate::builder::selector::CoverageRepairSelector;
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCursor, MoveSelector};

fn coverage_repair_selector(
    value_candidate_limit: Option<usize>,
    max_moves_per_step: Option<usize>,
    require_hard_improvement: bool,
) -> CoverageRepairSelector<CoveragePlan> {
    coverage_repair_selector_with_model(
        coverage_model(),
        value_candidate_limit,
        max_moves_per_step,
        require_hard_improvement,
    )
}

fn coverage_repair_selector_with_model(
    model: RuntimeModel<CoveragePlan, usize, DefaultMeter, DefaultMeter>,
    value_candidate_limit: Option<usize>,
    max_moves_per_step: Option<usize>,
    require_hard_improvement: bool,
) -> CoverageRepairSelector<CoveragePlan> {
    CoverageRepairSelector::new(
        model.coverage_groups()[0],
        value_candidate_limit,
        max_moves_per_step,
        require_hard_improvement,
    )
}

fn repair_move_results(
    plan: &CoveragePlan,
    selector: &CoverageRepairSelector<CoveragePlan>,
) -> Vec<CoveragePlan> {
    let director = CoverageDirector {
        working_solution: plan.clone(),
        descriptor: coverage_plan_descriptor(),
    };
    let mut cursor = selector.open_cursor(&director);
    let mut results = Vec::new();
    while let Some(id) = cursor.next_candidate() {
        let mov = cursor.take_candidate(id);
        let mut trial = CoverageDirector {
            working_solution: plan.clone(),
            descriptor: coverage_plan_descriptor(),
        };
        assert!(mov.is_doable(&trial));
        mov.do_move(&mut trial);
        trial.calculate_score();
        results.push(trial.working_solution);
    }
    results
}

#[test]
fn coverage_repair_selector_emits_only_hard_improving_required_moves() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0]),
            coverage_slot(true, 0, Some(0), &[0, 1]),
        ],
    );
    let selector = coverage_repair_selector(None, Some(8), true);
    let director = CoverageDirector {
        working_solution: plan.clone(),
        descriptor: coverage_plan_descriptor(),
    };
    let mut cursor = selector.open_cursor(&director);
    let mut emitted = 0;

    while let Some(id) = cursor.next_candidate() {
        let mov = cursor.take_candidate(id);
        let mut trial = CoverageDirector {
            working_solution: plan.clone(),
            descriptor: coverage_plan_descriptor(),
        };
        let current = trial.calculate_score();
        assert!(mov.requires_hard_improvement());
        assert!(mov.is_doable(&trial));
        mov.do_move(&mut trial);
        let next = trial.calculate_score();
        assert!(next.hard() > current.hard());
        emitted += 1;
    }

    assert!(emitted > 0);
}

#[test]
fn coverage_repair_falls_back_to_moving_preferred_keeper() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 0, Some(0), &[0]),
        ],
    );
    let selector = coverage_repair_selector(None, Some(8), true);
    let results = repair_move_results(&plan, &selector);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].slots[0].assigned, Some(1));
    assert_eq!(results[0].slots[1].assigned, Some(0));
    assert_eq!(results[0].score, Some(HardSoftScore::of(0, 0)));
}

#[test]
fn coverage_repair_orders_capacity_conflict_groups_by_coverage_order() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(1), &[1]),
            coverage_slot(false, 0, Some(1), &[1]),
            coverage_slot(true, 0, Some(0), &[0]),
            coverage_slot(false, 0, Some(0), &[0]),
        ],
    );
    let selector = coverage_repair_selector(None, Some(1), false);
    let results = repair_move_results(&plan, &selector);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].slots[1].assigned, None);
    assert_eq!(results[0].slots[3].assigned, Some(0));
}

#[test]
fn coverage_repair_honors_value_candidate_limit_when_relocating_blocker() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0]),
            coverage_slot(true, 0, Some(0), &[0, 1]),
        ],
    );
    let limited_selector = coverage_repair_selector(Some(1), Some(8), false);
    let unlimited_selector = coverage_repair_selector(None, Some(8), false);

    assert!(repair_move_results(&plan, &limited_selector).is_empty());

    let unlimited_results = repair_move_results(&plan, &unlimited_selector);
    assert_eq!(unlimited_results.len(), 1);
    assert_eq!(unlimited_results[0].slots[0].assigned, Some(0));
    assert_eq!(unlimited_results[0].slots[1].assigned, Some(1));
}

#[test]
fn coverage_repair_uses_group_cap_when_selector_cap_is_omitted() {
    let model = coverage_model_with_limits(CoverageGroupLimits {
        max_moves_per_step: Some(1),
        max_augmenting_depth: Some(3),
        ..CoverageGroupLimits::new()
    });
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 0, None, &[0, 1, 2]),
        ],
    );
    let selector = coverage_repair_selector_with_model(model, None, None, false);

    assert_eq!(repair_move_results(&plan, &selector).len(), 1);
}

#[test]
fn coverage_repair_selector_cap_overrides_group_cap() {
    let model = coverage_model_with_limits(CoverageGroupLimits {
        max_moves_per_step: Some(1),
        max_augmenting_depth: Some(3),
        ..CoverageGroupLimits::new()
    });
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 0, None, &[0, 1, 2]),
        ],
    );
    let selector = coverage_repair_selector_with_model(model, None, Some(2), false);

    assert_eq!(repair_move_results(&plan, &selector).len(), 2);
}
