use std::time::Duration;

use solverforge_core::score::SoftScore;
use solverforge_core::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::super::*;
use super::common::NoOpPhase;

type TestDirector = ScoreDirector<TestSolution, ()>;
type EntityTestDirector = ScoreDirector<EntityTestSolution, ()>;

#[derive(Clone, Debug)]
struct TestSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct TestEntity {
    #[allow(dead_code)]
    id: i64,
    value: Option<i64>,
}

#[derive(Clone, Debug)]
struct EntityTestSolution {
    entities: Vec<TestEntity>,
    target_sum: i64,
    score: Option<SoftScore>,
}

impl PlanningSolution for EntityTestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn calculate_entity_score(solution: &EntityTestSolution) -> SoftScore {
    let sum: i64 = solution.entities.iter().filter_map(|e| e.value).sum();
    let diff = (sum - solution.target_sum).abs();
    SoftScore::of(-diff)
}

#[test]
fn test_solver_with_time_limit_termination() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(1),
            },
            TestEntity {
                id: 1,
                value: Some(2),
            },
            TestEntity {
                id: 2,
                value: Some(2),
            },
        ],
        target_sum: 5,
        score: None,
    };

    let factory =
        solver_factory_builder::<EntityTestSolution, EntityTestDirector, _>(calculate_entity_score)
            .with_time_limit(Duration::from_millis(100))
            .build()
            .expect("Failed to build factory");

    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(0));
}

#[test]
fn test_solver_with_step_limit_termination() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(0),
            },
            TestEntity {
                id: 1,
                value: Some(0),
            },
        ],
        target_sum: 6,
        score: None,
    };

    let factory =
        solver_factory_builder::<EntityTestSolution, EntityTestDirector, _>(calculate_entity_score)
            .with_step_limit(5)
            .build()
            .expect("Failed to build factory");

    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(-6));
}

#[test]
fn test_solver_factory_with_entity_solution() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(2),
            },
            TestEntity {
                id: 1,
                value: Some(3),
            },
        ],
        target_sum: 5,
        score: None,
    };

    let factory =
        solver_factory_builder::<EntityTestSolution, EntityTestDirector, _>(calculate_entity_score)
            .build()
            .expect("Failed to build factory");

    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(0));
}

#[test]
fn test_solver_factory_with_phases() {
    let factory = solver_factory_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SoftScore::of(-s.value)
    })
    .with_phase(NoOpPhase)
    .with_step_limit(10)
    .build()
    .expect("Failed to build factory");

    let solution = TestSolution {
        value: 5,
        score: None,
    };
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(-5));
}

#[test]
fn test_solver_factory_with_multiple_phases() {
    let factory = solver_factory_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SoftScore::of(-s.value)
    })
    .with_phase(NoOpPhase)
    .with_phase(NoOpPhase)
    .with_time_limit(Duration::from_secs(1))
    .build()
    .expect("Failed to build factory");

    let solution = TestSolution {
        value: 7,
        score: None,
    };
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(-7));
}

#[test]
fn test_construction_and_local_acceptor_types_exist() {
    assert_eq!(ConstructionType::default(), ConstructionType::FirstFit);
    assert_eq!(
        LocalSearchAcceptorType::default(),
        LocalSearchAcceptorType::HillClimbing
    );

    let _tabu = LocalSearchAcceptorType::TabuSearch { tabu_size: 10 };
    let _sa = LocalSearchAcceptorType::SimulatedAnnealing {
        starting_temp: 1.0,
        decay_rate: 0.99,
    };
    let _la = LocalSearchAcceptorType::LateAcceptance { size: 100 };
    let _bf = ConstructionType::BestFit;
}
