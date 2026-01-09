//! Integration tests for SolverManager with termination and solving.

use super::*;
use std::any::TypeId;

use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::termination::StepCountTermination;

#[derive(Clone, Debug)]
struct TestEntity {
    value: Option<i64>,
}

#[derive(Clone, Debug)]
struct EntityTestSolution {
    entities: Vec<TestEntity>,
    target_sum: i64,
    score: Option<SimpleScore>,
}

impl PlanningSolution for EntityTestSolution {
    type Score = SimpleScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_entities(s: &EntityTestSolution) -> &Vec<TestEntity> {
    &s.entities
}

fn get_entities_mut(s: &mut EntityTestSolution) -> &mut Vec<TestEntity> {
    &mut s.entities
}

fn get_entity_value(s: &EntityTestSolution, idx: usize) -> Option<i64> {
    s.entities.get(idx).and_then(|e| e.value)
}

fn set_entity_value(s: &mut EntityTestSolution, idx: usize, v: Option<i64>) {
    if let Some(entity) = s.entities.get_mut(idx) {
        entity.value = v;
    }
}

fn calculate_entity_score(solution: &EntityTestSolution) -> SimpleScore {
    let sum: i64 = solution.entities.iter().filter_map(|e| e.value).sum();
    let diff = (sum - solution.target_sum).abs();
    SimpleScore::of(-diff)
}

fn create_entity_director(
    solution: EntityTestSolution,
) -> SimpleScoreDirector<EntityTestSolution, impl Fn(&EntityTestSolution) -> SimpleScore> {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor);

    let descriptor =
        SolutionDescriptor::new("EntityTestSolution", TypeId::of::<EntityTestSolution>())
            .with_entity(entity_desc);

    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_entity_score)
}

type EntityMove = ChangeMove<EntityTestSolution, i64>;

#[test]
fn test_solver_with_step_limit_termination() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity { value: Some(0) },
            TestEntity { value: Some(0) },
            TestEntity { value: Some(0) },
        ],
        target_sum: 10,
        score: None,
    };

    let director = create_entity_director(solution);

    let termination_factory: Box<
        dyn Fn() -> Box<dyn crate::termination::Termination<EntityTestSolution>> + Send + Sync,
    > = Box::new(|| Box::new(StepCountTermination::new(10)));

    let phase_factory =
        LocalSearchPhaseFactory::<EntityTestSolution, EntityMove, _>::hill_climbing(|| {
            let values: Vec<i64> = (0..=10).collect();
            Box::new(ChangeMoveSelector::<EntityTestSolution, i64>::simple(
                get_entity_value,
                set_entity_value,
                0,
                "value",
                values,
            ))
        })
        .with_step_limit(20);

    let manager = SolverManager::new(vec![Box::new(phase_factory)], Some(termination_factory));

    let mut solver = manager.create_solver();
    let result = solver.solve_with_director(Box::new(director));

    assert!(result.entities.iter().all(|e| e.value.is_some()));
}

#[test]
fn test_solver_manager_solve_improves_score() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity { value: Some(0) },
            TestEntity { value: Some(0) },
            TestEntity { value: Some(0) },
        ],
        target_sum: 6,
        score: None,
    };

    let initial_score = calculate_entity_score(&solution);
    assert_eq!(initial_score, SimpleScore::of(-6));

    let director = create_entity_director(solution);

    let phase_factory =
        LocalSearchPhaseFactory::<EntityTestSolution, EntityMove, _>::hill_climbing(|| {
            let values: Vec<i64> = (0..=10).collect();
            Box::new(ChangeMoveSelector::<EntityTestSolution, i64>::simple(
                get_entity_value,
                set_entity_value,
                0,
                "value",
                values,
            ))
        })
        .with_step_limit(100);

    let manager = SolverManager::new(vec![Box::new(phase_factory)], None);

    let mut solver = manager.create_solver();
    let result = solver.solve_with_director(Box::new(director));

    let final_score = calculate_entity_score(&result);
    assert!(final_score >= initial_score);
}
