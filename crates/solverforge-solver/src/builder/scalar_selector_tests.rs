use solverforge_config::{
    ChangeMoveConfig, MoveSelectorConfig, NearbyChangeMoveConfig, PillarChangeMoveConfig,
    PillarSwapMoveConfig, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

use super::*;
use crate::heuristic::r#move::Move;

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

fn get_worker(solution: &Schedule, entity_index: usize) -> Option<usize> {
    solution.shifts[entity_index].worker
}

fn set_worker(solution: &mut Schedule, entity_index: usize, value: Option<usize>) {
    solution.shifts[entity_index].worker = value;
}

fn worker_count(solution: &Schedule) -> usize {
    solution.workers.len()
}

fn allowed_workers(solution: &Schedule, entity_index: usize) -> &[usize] {
    &solution.shifts[entity_index].allowed_workers
}

fn nearby_worker_value_distance(_solution: &Schedule, entity_index: usize, value: usize) -> f64 {
    entity_index.abs_diff(value) as f64
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
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
        },
        true,
    )];

    let selector = build_scalar_move_selector::<Schedule>(None, &scalar_variables);
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

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables);
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
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
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

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 4);
    assert!(moves
        .iter()
        .all(|mov| matches!(mov, crate::heuristic::r#move::ScalarMoveUnion::Change(_))));
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
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
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

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables);
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

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables);
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

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    let mut swap_pairs = Vec::new();
    for mov in &moves {
        assert!(mov.is_doable(&director));
        if let crate::heuristic::r#move::ScalarMoveUnion::PillarSwap(swap) = mov {
            let left_value =
                get_worker(director.working_solution(), swap.left_indices()[0]).unwrap();
            let right_value =
                get_worker(director.working_solution(), swap.right_indices()[0]).unwrap();
            swap_pairs.push((left_value, right_value));
        }
    }
    swap_pairs.sort_unstable();

    assert_eq!(swap_pairs, vec![(0, 2), (1, 2)]);
}
