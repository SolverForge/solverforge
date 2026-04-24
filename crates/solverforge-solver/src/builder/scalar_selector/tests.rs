use solverforge_config::{
    CartesianProductConfig, ChangeMoveConfig, MoveSelectorConfig, NearbyChangeMoveConfig,
    NearbySwapMoveConfig, PillarChangeMoveConfig, PillarSwapMoveConfig, RecreateHeuristicType,
    RuinRecreateMoveSelectorConfig, SwapMoveConfig, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

use super::*;
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::FilteringMoveSelector;
use crate::heuristic::selector::move_selector::{collect_cursor_indices, MoveCandidateRef};
use crate::heuristic::selector::MoveSelector;

#[derive(Clone, Debug)]
struct Shift {
    worker: Option<usize>,
    allowed_workers: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Schedule {
    workers: Vec<usize>,
    shifts: Vec<Shift>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Schedule {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_shifts(solution: &Schedule) -> &Vec<Shift> {
    &solution.shifts
}

fn get_shifts_mut(solution: &mut Schedule) -> &mut Vec<Shift> {
    &mut solution.shifts
}

fn shift_count(solution: &Schedule) -> usize {
    solution.shifts.len()
}

fn get_worker(solution: &Schedule, entity_index: usize, _variable_index: usize) -> Option<usize> {
    solution.shifts[entity_index].worker
}

fn set_worker(
    solution: &mut Schedule,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.shifts[entity_index].worker = value;
}

fn worker_count(solution: &Schedule, _provider_index: usize) -> usize {
    solution.workers.len()
}

fn allowed_workers(solution: &Schedule, entity_index: usize, _variable_index: usize) -> &[usize] {
    &solution.shifts[entity_index].allowed_workers
}

fn nearby_worker_value_distance(
    _solution: &Schedule,
    entity_index: usize,
    _variable_index: usize,
    value: usize,
) -> Option<f64> {
    Some(entity_index.abs_diff(value) as f64)
}

fn nearby_worker_entity_distance(
    _solution: &Schedule,
    left: usize,
    right: usize,
    _variable_index: usize,
) -> Option<f64> {
    Some(match (left, right) {
        (0, 1) => 0.0,
        (0, 2) => 1.0,
        (1, 2) => 0.5,
        _ => left.abs_diff(right) as f64,
    })
}

fn create_director(solution: Schedule) -> ScoreDirector<Schedule, ()> {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Shift",
        "shifts",
        get_shifts,
        get_shifts_mut,
    ));
    let descriptor = SolutionDescriptor::new("Schedule", TypeId::of::<Schedule>()).with_entity(
        EntityDescriptor::new("Shift", TypeId::of::<Shift>(), "shifts").with_extractor(extractor),
    );

    ScoreDirector::simple(solution, descriptor, |solution, _| solution.shifts.len())
}

#[test]
fn builds_solution_count_scalar_selectors_without_descriptor_bindings() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![1],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        true,
    )];

    let selector = build_scalar_move_selector::<Schedule>(None, &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 9);
    assert_eq!(moves.len(), 9);
    assert_eq!(
        moves.iter()
            .filter(|mov| matches!(mov, crate::heuristic::r#move::ScalarMoveUnion::Change(change) if change.to_value().is_none()))
            .count(),
        2
    );
}

#[test]
fn filters_change_moves_against_entity_slice_candidates() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![1],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        true,
    )];

    let config = MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 5);
    assert_eq!(moves.len(), 5);
    assert_eq!(
        moves.iter()
            .filter(|mov| matches!(mov, crate::heuristic::r#move::ScalarMoveUnion::Change(change) if change.to_value().is_none()))
            .count(),
        2
    );
}

#[test]
fn filters_swap_moves_against_entity_slice_candidates_before_evaluation() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![0, 1],
            },
            Shift {
                worker: Some(2),
                allowed_workers: vec![2],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        true,
    )];
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    let swap_pairs: Vec<_> = moves
        .iter()
        .map(|mov| match mov {
            crate::heuristic::r#move::ScalarMoveUnion::Swap(swap) => {
                (swap.left_entity_index(), swap.right_entity_index())
            }
            other => panic!("expected swap move, got {other:?}"),
        })
        .collect();

    assert_eq!(swap_pairs, vec![(0, 1)]);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn swap_selector_emits_complete_assignment_swaps_without_domain() {
    let director = create_director(Schedule {
        workers: vec![],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::Empty,
        false,
    )];
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 1);
    assert_eq!(moves.len(), 1);
    assert!(matches!(
        &moves[0],
        crate::heuristic::r#move::ScalarMoveUnion::Swap(swap)
            if (swap.left_entity_index(), swap.right_entity_index()) == (0, 1)
    ));
    assert!(moves[0].is_doable(&director));
}

#[test]
fn swap_selector_rejects_explicit_empty_entity_slice_domain() {
    let director = create_director(Schedule {
        workers: vec![],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        false,
    )];
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 0);
    assert!(moves.is_empty());
}

#[test]
fn builds_nearby_change_selectors_when_meter_is_present() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        true,
    )
    .with_nearby_value_distance_meter(nearby_worker_value_distance)];

    let config = MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
        max_nearby: 1,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 4);
    let change_targets: Vec<_> = moves
        .iter()
        .map(|mov| match mov {
            crate::heuristic::r#move::ScalarMoveUnion::Change(change) => {
                (change.entity_index(), change.to_value().copied())
            }
            other => panic!("expected nearby change move, got {other:?}"),
        })
        .collect();
    assert_eq!(
        change_targets,
        vec![(0, Some(1)), (0, None), (1, Some(0)), (1, None)]
    );
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn nearby_swap_filters_same_value_candidates_before_limiting() {
    let director = create_director(Schedule {
        workers: vec![0, 1],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1],
            },
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![0, 1],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        true,
    )
    .with_nearby_entity_distance_meter(nearby_worker_entity_distance)];

    let config = MoveSelectorConfig::NearbySwapMoveSelector(NearbySwapMoveConfig {
        max_nearby: 1,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    let swap_pairs: Vec<_> = moves
        .iter()
        .map(|mov| match mov {
            crate::heuristic::r#move::ScalarMoveUnion::Swap(swap) => {
                (swap.left_entity_index(), swap.right_entity_index())
            }
            other => panic!("expected nearby swap move, got {other:?}"),
        })
        .collect();

    assert_eq!(swap_pairs, vec![(0, 2), (1, 2)]);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn ruin_recreate_skips_required_entities_without_recreate_values() {
    let director = create_director(Schedule {
        workers: vec![],
        shifts: vec![Shift {
            worker: Some(0),
            allowed_workers: vec![],
        }],
        score: None,
    });
    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        false,
    )];
    let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
        min_ruin_count: 1,
        max_ruin_count: 1,
        moves_per_step: Some(4),
        recreate_heuristic_type: RecreateHeuristicType::FirstFit,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert!(moves.is_empty());
}

#[test]
fn ruin_recreate_honors_configured_random_seed() {
    fn batches(seed: Option<u64>) -> Vec<Vec<usize>> {
        let director = create_director(Schedule {
            workers: vec![0, 1, 2],
            shifts: (0..8)
                .map(|_| Shift {
                    worker: Some(0),
                    allowed_workers: vec![0, 1, 2],
                })
                .collect(),
            score: None,
        });
        let scalar_variables = vec![ScalarVariableContext::new(
            0,
            0,
            "Shift",
            shift_count,
            "worker",
            get_worker,
            set_worker,
            ValueSource::SolutionCount {
                count_fn: worker_count,
                provider_index: 0,
            },
            false,
        )];
        let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
            min_ruin_count: 1,
            max_ruin_count: 3,
            moves_per_step: Some(16),
            recreate_heuristic_type: RecreateHeuristicType::FirstFit,
            target: VariableTargetConfig {
                entity_class: Some("Shift".to_string()),
                variable_name: Some("worker".to_string()),
            },
        });
        let selector = build_scalar_move_selector(Some(&config), &scalar_variables, seed);

        selector
            .iter_moves(&director)
            .map(|mov| {
                assert!(matches!(
                    mov,
                    crate::heuristic::r#move::ScalarMoveUnion::RuinRecreate(_)
                ));
                mov.entity_indices().to_vec()
            })
            .collect()
    }

    let first = batches(Some(17));
    let repeat = batches(Some(17));
    let changed = batches(Some(18));

    assert_eq!(first, repeat);
    assert_ne!(first, changed);
}

#[test]
fn ruin_recreate_do_move_preserves_required_assignment_when_recreate_values_are_empty() {
    let mut director = create_director(Schedule {
        workers: vec![],
        shifts: vec![Shift {
            worker: Some(0),
            allowed_workers: vec![],
        }],
        score: None,
    });
    let mov = crate::heuristic::r#move::RuinRecreateMove::new(
        &[0],
        get_worker,
        set_worker,
        0,
        0,
        "worker",
        crate::heuristic::r#move::ScalarRecreateValueSource::EntitySlice {
            values_for_entity: allowed_workers,
            variable_index: 0,
        },
        RecreateHeuristicType::FirstFit,
        false,
    );

    assert!(!mov.is_doable(&director));
    mov.do_move(&mut director);

    assert_eq!(director.working_solution().shifts[0].worker, Some(0));
}

#[test]
fn pillar_change_uses_public_pillar_semantics() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        true,
    )];

    let config = MoveSelectorConfig::PillarChangeMoveSelector(PillarChangeMoveConfig {
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 2);
    assert!(moves.iter().all(|mov| matches!(
        mov,
        crate::heuristic::r#move::ScalarMoveUnion::PillarChange(_)
    )));
}

#[test]
fn pillar_change_intersects_entity_slice_domains() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        true,
    )];
    let config = MoveSelectorConfig::PillarChangeMoveSelector(PillarChangeMoveConfig {
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 1);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
    assert!(matches!(
        &moves[0],
        crate::heuristic::r#move::ScalarMoveUnion::PillarChange(change)
            if change.to_value() == Some(&2)
    ));
}

#[test]
fn pillar_swap_prunes_illegal_entity_slice_partners() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
            },
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![1, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![1, 2],
            },
            Shift {
                worker: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
            Shift {
                worker: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        true,
    )];
    let config = MoveSelectorConfig::PillarSwapMoveSelector(PillarSwapMoveConfig {
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    let mut swap_pairs = Vec::new();
    for mov in &moves {
        assert!(mov.is_doable(&director));
        if let crate::heuristic::r#move::ScalarMoveUnion::PillarSwap(swap) = mov {
            let left_value =
                get_worker(director.working_solution(), swap.left_indices()[0], 0).unwrap();
            let right_value =
                get_worker(director.working_solution(), swap.right_indices()[0], 0).unwrap();
            swap_pairs.push((left_value, right_value));
        }
    }
    swap_pairs.sort_unstable();

    assert_eq!(swap_pairs, vec![(0, 2), (1, 2)]);
}

fn keep_all_cartesian_scalar_candidates(
    candidate: MoveCandidateRef<
        '_,
        Schedule,
        crate::heuristic::r#move::ScalarMoveUnion<Schedule, usize>,
    >,
) -> bool {
    matches!(candidate, MoveCandidateRef::Sequential(_))
}

#[test]
fn scalar_builder_cartesian_selector_survives_filtering_wrapper() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    });
    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        true,
    )];
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let filtered = FilteringMoveSelector::new(selector, keep_all_cartesian_scalar_candidates);
    let mut cursor = filtered.open_cursor(&director);
    let indices = collect_cursor_indices::<
        Schedule,
        crate::heuristic::r#move::ScalarMoveUnion<Schedule, usize>,
        _,
    >(&mut cursor);

    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(cursor.take_candidate(indices[0]).is_doable(&director));
}
