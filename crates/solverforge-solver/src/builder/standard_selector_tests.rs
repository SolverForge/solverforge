use solverforge_config::{ChangeMoveConfig, MoveSelectorConfig, VariableTargetConfig};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

use super::*;

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
fn builds_solution_count_standard_selectors_without_descriptor_bindings() {
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

    let selector = build_standard_move_selector::<Schedule>(None, &scalar_variables);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 9);
    assert_eq!(moves.len(), 9);
    assert_eq!(
        moves.iter()
            .filter(|mov| matches!(mov, crate::heuristic::r#move::EitherMove::Change(change) if change.to_value().is_none()))
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

    let selector = build_standard_move_selector(Some(&config), &scalar_variables);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 5);
    assert_eq!(moves.len(), 5);
    assert_eq!(
        moves.iter()
            .filter(|mov| matches!(mov, crate::heuristic::r#move::EitherMove::Change(change) if change.to_value().is_none()))
            .count(),
        2
    );
}
