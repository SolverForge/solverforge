use std::any::{Any, TypeId};

use solverforge_config::{
    CartesianProductConfig, ChangeMoveConfig, ConstructionHeuristicConfig,
    ConstructionHeuristicType, MoveSelectorConfig, NearbyChangeMoveConfig, PillarChangeMoveConfig,
    PillarSwapMoveConfig, RecreateHeuristicType, RuinRecreateMoveSelectorConfig, SwapMoveConfig,
    VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, ProblemFactDescriptor,
    SolutionDescriptor, VariableDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, ScoreDirector};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveSelector;
use crate::phase::localsearch::{FirstAcceptedForager, HillClimbingAcceptor, LocalSearchPhase};
use crate::phase::Phase;
use crate::scope::SolverScope;

use super::{build_descriptor_construction, build_descriptor_move_selector, scalar_work_remaining};

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
struct RestrictedTask {
    worker_idx: Option<usize>,
    allowed_workers: Vec<usize>,
}

#[derive(Clone, Debug)]
struct RestrictedPlan {
    workers: Vec<Worker>,
    tasks: Vec<RestrictedTask>,
    score: Option<SoftScore>,
}

impl PlanningSolution for RestrictedPlan {
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
    ByWorker {
        unassigned_score: i64,
        assigned_scores: [i64; 3],
    },
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
            PlanScoreMode::ByWorker {
                unassigned_score,
                assigned_scores,
            } => SoftScore::of(
                self.working_solution.tasks[0]
                    .worker_idx
                    .map(|worker_idx| assigned_scores[worker_idx])
                    .unwrap_or(unassigned_score),
            ),
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

fn nearby_worker_value_distance(solution: &dyn Any, entity_index: usize, value: usize) -> f64 {
    let plan = solution.downcast_ref::<Plan>().expect("plan expected");
    let current = plan.tasks[entity_index].worker_idx.unwrap_or(0);
    current.abs_diff(value) as f64
}

fn restricted_get_worker_idx(entity: &dyn Any) -> Option<usize> {
    entity
        .downcast_ref::<RestrictedTask>()
        .expect("restricted task expected")
        .worker_idx
}

fn restricted_set_worker_idx(entity: &mut dyn Any, value: Option<usize>) {
    entity
        .downcast_mut::<RestrictedTask>()
        .expect("restricted task expected")
        .worker_idx = value;
}

fn restricted_allowed_workers(entity: &dyn Any) -> Vec<usize> {
    entity
        .downcast_ref::<RestrictedTask>()
        .expect("restricted task expected")
        .allowed_workers
        .clone()
}

fn descriptor_with_allows_unassigned(allows_unassigned: bool) -> SolutionDescriptor {
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
                        .with_allows_unassigned(allows_unassigned)
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

fn descriptor() -> SolutionDescriptor {
    descriptor_with_allows_unassigned(true)
}

fn descriptor_with_nearby_value_meter() -> SolutionDescriptor {
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
                        .with_usize_accessors(get_worker_idx, set_worker_idx)
                        .with_nearby_value_distance_meter(nearby_worker_value_distance),
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

fn restricted_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("RestrictedPlan", TypeId::of::<RestrictedPlan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<RestrictedTask>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |s: &RestrictedPlan| &s.tasks,
                    |s: &mut RestrictedPlan| &mut s.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_usize_accessors(restricted_get_worker_idx, restricted_set_worker_idx)
                        .with_entity_value_provider(restricted_allowed_workers),
                ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<Worker>(), "workers").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |s: &RestrictedPlan| &s.workers,
                    |s: &mut RestrictedPlan| &mut s.workers,
                )),
            ),
        )
}

#[test]
fn solution_level_value_range_generates_scalar_work() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };

    assert!(scalar_work_remaining(
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
fn descriptor_first_fit_required_slot_still_assigns_first_doable_value() {
    let descriptor = descriptor_with_allows_unassigned(false);
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
fn descriptor_first_fit_optional_slot_keeps_none_when_baseline_is_not_beaten() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [-5, -1, -2],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = build_descriptor_construction::<Plan>(None, &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(0))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 0);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn descriptor_first_fit_optional_slot_skips_worse_candidate_for_later_improvement() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [-5, 7, -1],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = build_descriptor_construction::<Plan>(None, &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(7))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn descriptor_first_fit_optional_slot_takes_first_improving_candidate() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [7, -5, 3],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = build_descriptor_construction::<Plan>(None, &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(7))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
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

    solver_scope
        .score_director_mut()
        .set_score_mode(PlanScoreMode::AllAssignedBonus);

    let mut reconstruct = build_descriptor_construction::<Plan>(None, &descriptor);
    reconstruct.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
}

#[test]
fn descriptor_nearby_change_uses_value_distance_meter() {
    let descriptor = descriptor_with_nearby_value_meter();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
        max_nearby: 1,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 4);
    assert_eq!(moves.len(), 4);
    assert!(moves
        .iter()
        .all(|mov| matches!(mov, super::DescriptorScalarMoveUnion::Change(_))));
}

#[test]
fn descriptor_pillar_change_uses_public_pillar_semantics() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::PillarChangeMoveSelector(PillarChangeMoveConfig {
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 2);
    assert_eq!(moves.len(), 2);
    assert!(moves
        .iter()
        .all(|mov| matches!(mov, super::DescriptorScalarMoveUnion::PillarChange(_))));
}

#[test]
fn descriptor_pillar_change_intersects_entity_domains() {
    let descriptor = restricted_descriptor();
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    };
    let mut director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::PillarChangeMoveSelector(PillarChangeMoveConfig {
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<RestrictedPlan>(Some(&config), &descriptor);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 1);
    assert!(moves[0].is_doable(&director));
    moves[0].do_move(&mut director);
    assert_eq!(director.working_solution().tasks[0].worker_idx, Some(2));
    assert_eq!(director.working_solution().tasks[1].worker_idx, Some(2));
}

#[test]
fn descriptor_pillar_swap_prunes_illegal_partners() {
    let descriptor = restricted_descriptor();
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
            RestrictedTask {
                worker_idx: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
            RestrictedTask {
                worker_idx: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::PillarSwapMoveSelector(PillarSwapMoveConfig {
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<RestrictedPlan>(Some(&config), &descriptor);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 2);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn descriptor_manual_illegal_pillar_moves_are_not_doable() {
    let descriptor = restricted_descriptor();
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let binding = super::bindings::collect_bindings(&descriptor)
        .into_iter()
        .next()
        .expect("restricted descriptor binding");

    let illegal_change = super::DescriptorPillarChangeMove::new(
        binding.clone(),
        vec![0, 1],
        Some(1),
        descriptor.clone(),
    );
    let illegal_swap =
        super::DescriptorPillarSwapMove::new(binding, vec![0, 1], vec![2, 3], descriptor);

    assert!(!illegal_change.is_doable(&director));
    assert!(!illegal_swap.is_doable(&director));
}

#[test]
fn descriptor_cartesian_builds_composite_moves() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
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

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert!(!moves.is_empty());
    assert!(moves
        .iter()
        .all(|mov| matches!(mov, super::DescriptorScalarMoveUnion::Composite(_))));
    let signature = moves[0].tabu_signature(&director);
    assert!(!signature.move_id.is_empty());
    assert!(!signature.entity_tokens.is_empty());
}

#[test]
fn descriptor_ruin_recreate_first_fit_reassigns_to_first_improving_value() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task {
            worker_idx: Some(1),
        }],
        score: None,
    };
    let mut director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [-5, -1, 7],
        },
    );
    let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
        min_ruin_count: 1,
        max_ruin_count: 1,
        moves_per_step: Some(1),
        recreate_heuristic_type: RecreateHeuristicType::FirstFit,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert_eq!(moves.len(), 1);

    moves[0].do_move(&mut director);

    assert_eq!(director.working_solution().tasks[0].worker_idx, Some(2));
}

#[test]
fn descriptor_ruin_recreate_cheapest_insertion_picks_best_value() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task {
            worker_idx: Some(0),
        }],
        score: None,
    };
    let mut director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [2, 7, 3],
        },
    );
    let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
        min_ruin_count: 1,
        max_ruin_count: 1,
        moves_per_step: Some(1),
        recreate_heuristic_type: RecreateHeuristicType::CheapestInsertion,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert_eq!(moves.len(), 1);

    moves[0].do_move(&mut director);

    assert_eq!(director.working_solution().tasks[0].worker_idx, Some(1));
}
