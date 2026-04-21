use std::any::{Any, TypeId};

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, ProblemFactDescriptor,
    SolutionDescriptor, VariableDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::selector::move_selector::MoveSelector;
use crate::phase::localsearch::{FirstAcceptedForager, HillClimbingAcceptor, LocalSearchPhase};
use crate::phase::Phase;
use crate::scope::SolverScope;

use super::{
    build_descriptor_construction, build_descriptor_move_selector, standard_work_remaining,
};

#[derive(Clone, Debug)]
struct Worker;

#[derive(Clone, Debug)]
struct Task {
    worker_idx: Option<usize>,
}

#[derive(Clone, Debug)]
struct Plan {
    workers: Vec<Worker>,
    tasks: Vec<Task>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct PlanScoreDirector {
    working_solution: Plan,
    descriptor: SolutionDescriptor,
    score_mode: PlanScoreMode,
}

#[derive(Clone, Copy, Debug)]
enum PlanScoreMode {
    AllAssignedBonus,
    PreferUnassigned,
}

impl PlanScoreDirector {
    fn new(solution: Plan, descriptor: SolutionDescriptor) -> Self {
        Self::with_mode(solution, descriptor, PlanScoreMode::AllAssignedBonus)
    }

    fn with_mode(
        solution: Plan,
        descriptor: SolutionDescriptor,
        score_mode: PlanScoreMode,
    ) -> Self {
        Self {
            working_solution: solution,
            descriptor,
            score_mode,
        }
    }

    fn set_score_mode(&mut self, score_mode: PlanScoreMode) {
        self.score_mode = score_mode;
    }
}

impl solverforge_scoring::Director<Plan> for PlanScoreDirector {
    fn working_solution(&self) -> &Plan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut Plan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = match self.score_mode {
            PlanScoreMode::AllAssignedBonus => {
                if self
                    .working_solution
                    .tasks
                    .iter()
                    .all(|task| task.worker_idx.is_some())
                {
                    SoftScore::of(10)
                } else {
                    SoftScore::of(0)
                }
            }
            PlanScoreMode::PreferUnassigned => {
                if self.working_solution.tasks[0].worker_idx.is_none() {
                    SoftScore::of(0)
                } else {
                    SoftScore::of(-1)
                }
            }
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> Plan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.tasks.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.tasks.len())
    }
}

fn get_worker_idx(entity: &dyn Any) -> Option<usize> {
    entity
        .downcast_ref::<Task>()
        .expect("task expected")
        .worker_idx
}

fn set_worker_idx(entity: &mut dyn Any, value: Option<usize>) {
    entity
        .downcast_mut::<Task>()
        .expect("task expected")
        .worker_idx = value;
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |s: &Plan| &s.tasks,
                    |s: &mut Plan| &mut s.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_value_range("workers")
                        .with_usize_accessors(get_worker_idx, set_worker_idx),
                ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<Worker>(), "workers").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |s: &Plan| &s.workers,
                    |s: &mut Plan| &mut s.workers,
                )),
            ),
        )
}

#[test]
fn solution_level_value_range_generates_standard_work() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };

    assert!(standard_work_remaining(
        &descriptor,
        Some("Task"),
        Some("worker_idx"),
        &plan
    ));
}

#[test]
fn solution_level_value_range_builds_change_moves() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let selector = build_descriptor_move_selector::<Plan>(None, &descriptor);

    assert_eq!(selector.size(&director), 3);
}

#[test]
fn descriptor_change_selector_adds_to_none_for_assigned_optional_variables() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task {
            worker_idx: Some(1),
        }],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let selector = build_descriptor_move_selector::<Plan>(None, &descriptor);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 4);
    assert_eq!(moves.len(), 4);
}

#[test]
fn descriptor_first_fit_assigns_optional_variables_when_moves_are_doable() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }, Task { worker_idx: None }],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = build_descriptor_construction::<Plan>(None, &descriptor);
    phase.solve(&mut solver_scope);

    assert!(solver_scope
        .working_solution()
        .tasks
        .iter()
        .all(|task| task.worker_idx == Some(0)));
    assert_eq!(solver_scope.stats().moves_accepted, 2);
}

#[test]
fn descriptor_best_fit_assigns_optional_variable_when_candidate_improves_score() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let config = ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::CheapestInsertion,
        target: VariableTargetConfig::default(),
        k: 1,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<Plan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn descriptor_completed_optional_none_is_skipped_by_later_construction_passes() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director =
        PlanScoreDirector::with_mode(plan, descriptor.clone(), PlanScoreMode::PreferUnassigned);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let best_fit_config = ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::CheapestInsertion,
        target: VariableTargetConfig::default(),
        k: 1,
        termination: None,
    };
    let mut best_fit_phase =
        build_descriptor_construction::<Plan>(Some(&best_fit_config), &descriptor);
    best_fit_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(solver_scope.stats().step_count, 1);

    let mut first_fit_phase = build_descriptor_construction::<Plan>(None, &descriptor);
    first_fit_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(solver_scope.stats().step_count, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 0);
}

#[test]
fn descriptor_reopened_optional_slot_is_revisited_by_later_construction() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut first_fit_phase = build_descriptor_construction::<Plan>(None, &descriptor);
    first_fit_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));

    solver_scope
        .score_director_mut()
        .set_score_mode(PlanScoreMode::PreferUnassigned);

    let move_selector = build_descriptor_move_selector::<Plan>(None, &descriptor);
    let mut local_search = LocalSearchPhase::new(
        move_selector,
        HillClimbingAcceptor::new(),
        FirstAcceptedForager::new(),
        Some(1),
    );
    local_search.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);

    let mut reconstruct = build_descriptor_construction::<Plan>(None, &descriptor);
    reconstruct.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
}
