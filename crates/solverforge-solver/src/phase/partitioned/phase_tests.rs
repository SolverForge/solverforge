use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use crate::manager::SolverTerminalReason;

use super::super::partitioner::{FunctionalPartitioner, ThreadCount};
use super::*;

#[derive(Clone, Debug)]
struct PartitionedLifecycleSolution {
    value: i64,
    shadow: i64,
    score: Option<SoftScore>,
}

impl PlanningSolution for PartitionedLifecycleSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }

    fn update_entity_shadows(&mut self, _descriptor_index: usize, _entity_index: usize) {
        self.shadow = self.value * 10;
    }

    fn update_all_shadows(&mut self) {
        self.shadow = self.value * 10;
    }
}

#[derive(Clone, Debug)]
struct PartitionedLifecycleDirector {
    solution: PartitionedLifecycleSolution,
    descriptor: SolutionDescriptor,
    cached_score: SoftScore,
    initialized: bool,
}

impl PartitionedLifecycleDirector {
    fn new(solution: PartitionedLifecycleSolution) -> Self {
        Self {
            solution,
            descriptor: SolutionDescriptor::new(
                "PartitionedLifecycleSolution",
                TypeId::of::<PartitionedLifecycleSolution>(),
            ),
            cached_score: SoftScore::of(0),
            initialized: false,
        }
    }
}

impl Director<PartitionedLifecycleSolution> for PartitionedLifecycleDirector {
    fn working_solution(&self) -> &PartitionedLifecycleSolution {
        &self.solution
    }

    fn working_solution_mut(&mut self) -> &mut PartitionedLifecycleSolution {
        &mut self.solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        if !self.initialized {
            self.solution.update_all_shadows();
            self.cached_score = SoftScore::of(self.solution.shadow);
            self.initialized = true;
        }
        self.solution.set_score(Some(self.cached_score));
        self.cached_score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> PartitionedLifecycleSolution {
        self.solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        if !self.initialized {
            return;
        }
        self.solution
            .update_entity_shadows(descriptor_index, entity_index);
        self.cached_score = SoftScore::of(self.solution.shadow);
        self.solution.set_score(Some(self.cached_score));
    }

    fn entity_count(&self, _descriptor_index: usize) -> Option<usize> {
        Some(1)
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(1)
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }

    fn is_incremental(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.initialized = false;
        self.cached_score = SoftScore::of(0);
        self.solution.set_score(None);
    }
}

#[derive(Debug)]
struct SetValuePhase {
    value: i64,
}

impl<D, BestCb> Phase<PartitionedLifecycleSolution, D, BestCb> for SetValuePhase
where
    D: Director<PartitionedLifecycleSolution>,
    BestCb: ProgressCallback<PartitionedLifecycleSolution>,
{
    fn solve(
        &mut self,
        solver_scope: &mut SolverScope<'_, PartitionedLifecycleSolution, D, BestCb>,
    ) {
        solver_scope.mutate(|score_director| {
            score_director.working_solution_mut().value = self.value;
            score_director.after_variable_changed(0, 0);
        });

        let shadow = solver_scope.working_solution().shadow;
        let mut best = solver_scope.working_solution().clone();
        best.set_score(Some(SoftScore::of(shadow)));
        solver_scope.set_best_solution(best, SoftScore::of(shadow));
    }

    fn phase_type_name(&self) -> &'static str {
        "SetValue"
    }
}

#[derive(Debug)]
struct ObservePoolPhase {
    observed_threads: Arc<AtomicUsize>,
}

impl<D, BestCb> Phase<PartitionedLifecycleSolution, D, BestCb> for ObservePoolPhase
where
    D: Director<PartitionedLifecycleSolution>,
    BestCb: ProgressCallback<PartitionedLifecycleSolution>,
{
    fn solve(
        &mut self,
        solver_scope: &mut SolverScope<'_, PartitionedLifecycleSolution, D, BestCb>,
    ) {
        self.observed_threads
            .store(rayon::current_num_threads(), Ordering::SeqCst);
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ObservePool"
    }
}

#[derive(Debug)]
struct MarkCancelledPhase;

impl<D, BestCb> Phase<PartitionedLifecycleSolution, D, BestCb> for MarkCancelledPhase
where
    D: Director<PartitionedLifecycleSolution>,
    BestCb: ProgressCallback<PartitionedLifecycleSolution>,
{
    fn solve(
        &mut self,
        solver_scope: &mut SolverScope<'_, PartitionedLifecycleSolution, D, BestCb>,
    ) {
        solver_scope.mark_cancelled();
    }

    fn phase_type_name(&self) -> &'static str {
        "MarkCancelled"
    }
}

#[derive(Debug)]
struct MarkTerminatedPhase;

impl<D, BestCb> Phase<PartitionedLifecycleSolution, D, BestCb> for MarkTerminatedPhase
where
    D: Director<PartitionedLifecycleSolution>,
    BestCb: ProgressCallback<PartitionedLifecycleSolution>,
{
    fn solve(
        &mut self,
        solver_scope: &mut SolverScope<'_, PartitionedLifecycleSolution, D, BestCb>,
    ) {
        solver_scope.mark_terminated_by_config();
    }

    fn phase_type_name(&self) -> &'static str {
        "MarkTerminated"
    }
}

#[derive(Debug)]
struct CountStepsPhase {
    attempts: Arc<AtomicUsize>,
    max_steps: usize,
}

impl<D, BestCb> Phase<PartitionedLifecycleSolution, D, BestCb> for CountStepsPhase
where
    D: Director<PartitionedLifecycleSolution>,
    BestCb: ProgressCallback<PartitionedLifecycleSolution>,
{
    fn solve(
        &mut self,
        solver_scope: &mut SolverScope<'_, PartitionedLifecycleSolution, D, BestCb>,
    ) {
        for _ in 0..self.max_steps {
            if solver_scope.should_terminate() {
                return;
            }
            self.attempts.fetch_add(1, Ordering::SeqCst);
            solver_scope.increment_step_count();
        }
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "CountSteps"
    }
}

#[derive(Debug)]
struct CountMovesPhase {
    attempts: Arc<AtomicUsize>,
    max_moves: usize,
}

impl<D, BestCb> Phase<PartitionedLifecycleSolution, D, BestCb> for CountMovesPhase
where
    D: Director<PartitionedLifecycleSolution>,
    BestCb: ProgressCallback<PartitionedLifecycleSolution>,
{
    fn solve(
        &mut self,
        solver_scope: &mut SolverScope<'_, PartitionedLifecycleSolution, D, BestCb>,
    ) {
        for _ in 0..self.max_moves {
            if solver_scope.should_terminate() {
                return;
            }
            self.attempts.fetch_add(1, Ordering::SeqCst);
            solver_scope.record_evaluated_move(Duration::ZERO);
        }
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "CountMoves"
    }
}

#[derive(Debug)]
struct CountScoresPhase {
    attempts: Arc<AtomicUsize>,
    max_scores: usize,
}

impl<D, BestCb> Phase<PartitionedLifecycleSolution, D, BestCb> for CountScoresPhase
where
    D: Director<PartitionedLifecycleSolution>,
    BestCb: ProgressCallback<PartitionedLifecycleSolution>,
{
    fn solve(
        &mut self,
        solver_scope: &mut SolverScope<'_, PartitionedLifecycleSolution, D, BestCb>,
    ) {
        for _ in 0..self.max_scores {
            if solver_scope.should_terminate() {
                return;
            }
            self.attempts.fetch_add(1, Ordering::SeqCst);
            solver_scope.calculate_score();
        }
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "CountScores"
    }
}

#[test]
fn test_config_default() {
    let config = PartitionedSearchConfig::default();
    assert_eq!(config.thread_count, ThreadCount::Auto);
    assert!(!config.log_progress);
}

#[test]
fn partitioned_search_reinitializes_after_merge() {
    let solution = PartitionedLifecycleSolution {
        value: 1,
        shadow: 10,
        score: None,
    };
    let director = PartitionedLifecycleDirector::new(solution);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.initialize_working_solution_as_best();

    let partitioner = FunctionalPartitioner::new(
        |solution: &PartitionedLifecycleSolution| vec![solution.clone()],
        |_original, _partitions| PartitionedLifecycleSolution {
            value: 7,
            shadow: 0,
            score: None,
        },
    );
    let mut phase =
        PartitionedSearchPhase::new(partitioner, PartitionedLifecycleDirector::new, || {
            (SetValuePhase { value: 1 },)
        });

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().value, 7);
    assert_eq!(solver_scope.working_solution().shadow, 70);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(70))
    );
    assert_eq!(solver_scope.best_score().copied(), Some(SoftScore::of(70)));
}

#[test]
fn partitioned_search_bootstraps_child_scopes_before_mutation() {
    let solution = PartitionedLifecycleSolution {
        value: 1,
        shadow: 10,
        score: None,
    };
    let director = PartitionedLifecycleDirector::new(solution);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.initialize_working_solution_as_best();

    let partitioner = FunctionalPartitioner::new(
        |solution: &PartitionedLifecycleSolution| vec![solution.clone()],
        |_original, mut partitions| partitions.pop().unwrap(),
    );
    let mut phase =
        PartitionedSearchPhase::new(partitioner, PartitionedLifecycleDirector::new, || {
            (SetValuePhase { value: 5 },)
        });

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().value, 5);
    assert_eq!(solver_scope.working_solution().shadow, 50);
    assert_eq!(solver_scope.best_score().copied(), Some(SoftScore::of(50)));
}

#[test]
fn partitioned_search_honors_specific_thread_count() {
    let solution = PartitionedLifecycleSolution {
        value: 1,
        shadow: 10,
        score: None,
    };
    let director = PartitionedLifecycleDirector::new(solution);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.initialize_working_solution_as_best();

    let observed_threads = Arc::new(AtomicUsize::new(0));
    let observed_threads_for_phase = Arc::clone(&observed_threads);
    let partitioner = FunctionalPartitioner::new(
        |solution: &PartitionedLifecycleSolution| vec![solution.clone(), solution.clone()],
        |_original, mut partitions| partitions.pop().unwrap(),
    );
    let config = PartitionedSearchConfig {
        thread_count: ThreadCount::Specific(2),
        log_progress: false,
    };
    let mut phase = PartitionedSearchPhase::with_config(
        partitioner,
        PartitionedLifecycleDirector::new,
        move || {
            (ObservePoolPhase {
                observed_threads: Arc::clone(&observed_threads_for_phase),
            },)
        },
        config,
    );

    phase.solve(&mut solver_scope);

    assert_eq!(observed_threads.load(Ordering::SeqCst), 2);
}

#[test]
fn sequential_partitioned_search_shares_step_limit_across_children() {
    let solution = PartitionedLifecycleSolution {
        value: 1,
        shadow: 10,
        score: None,
    };
    let director = PartitionedLifecycleDirector::new(solution);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.initialize_working_solution_as_best();
    solver_scope.inphase_step_count_limit = Some(2);

    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_phase = Arc::clone(&attempts);
    let partitioner = FunctionalPartitioner::new(
        |solution: &PartitionedLifecycleSolution| vec![solution.clone(), solution.clone()],
        |_original, mut partitions| partitions.pop().unwrap(),
    );
    let config = PartitionedSearchConfig {
        thread_count: ThreadCount::Specific(1),
        log_progress: false,
    };
    let mut phase = PartitionedSearchPhase::with_config(
        partitioner,
        PartitionedLifecycleDirector::new,
        move || {
            (CountStepsPhase {
                attempts: Arc::clone(&attempts_for_phase),
                max_steps: 3,
            },)
        },
        config,
    );

    phase.solve(&mut solver_scope);

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn sequential_partitioned_search_shares_move_limit_across_children() {
    let solution = PartitionedLifecycleSolution {
        value: 1,
        shadow: 10,
        score: None,
    };
    let director = PartitionedLifecycleDirector::new(solution);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.initialize_working_solution_as_best();
    solver_scope.inphase_move_count_limit = Some(2);

    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_phase = Arc::clone(&attempts);
    let partitioner = FunctionalPartitioner::new(
        |solution: &PartitionedLifecycleSolution| vec![solution.clone(), solution.clone()],
        |_original, mut partitions| partitions.pop().unwrap(),
    );
    let config = PartitionedSearchConfig {
        thread_count: ThreadCount::Specific(1),
        log_progress: false,
    };
    let mut phase = PartitionedSearchPhase::with_config(
        partitioner,
        PartitionedLifecycleDirector::new,
        move || {
            (CountMovesPhase {
                attempts: Arc::clone(&attempts_for_phase),
                max_moves: 3,
            },)
        },
        config,
    );

    phase.solve(&mut solver_scope);

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn sequential_partitioned_search_shares_score_limit_across_children() {
    let solution = PartitionedLifecycleSolution {
        value: 1,
        shadow: 10,
        score: None,
    };
    let director = PartitionedLifecycleDirector::new(solution);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.initialize_working_solution_as_best();
    solver_scope.inphase_score_calc_count_limit = Some(3);

    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_phase = Arc::clone(&attempts);
    let partitioner = FunctionalPartitioner::new(
        |solution: &PartitionedLifecycleSolution| vec![solution.clone(), solution.clone()],
        |_original, mut partitions| partitions.pop().unwrap(),
    );
    let config = PartitionedSearchConfig {
        thread_count: ThreadCount::Specific(1),
        log_progress: false,
    };
    let mut phase = PartitionedSearchPhase::with_config(
        partitioner,
        PartitionedLifecycleDirector::new,
        move || {
            (CountScoresPhase {
                attempts: Arc::clone(&attempts_for_phase),
                max_scores: 3,
            },)
        },
        config,
    );

    phase.solve(&mut solver_scope);

    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn partitioned_search_does_not_merge_cancelled_children() {
    let solution = PartitionedLifecycleSolution {
        value: 1,
        shadow: 10,
        score: None,
    };
    let director = PartitionedLifecycleDirector::new(solution);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.initialize_working_solution_as_best();

    let merge_count = Arc::new(AtomicUsize::new(0));
    let merge_count_for_partitioner = Arc::clone(&merge_count);
    let partitioner = FunctionalPartitioner::new(
        |solution: &PartitionedLifecycleSolution| vec![solution.clone()],
        move |_original, _partitions| {
            merge_count_for_partitioner.fetch_add(1, Ordering::SeqCst);
            PartitionedLifecycleSolution {
                value: 99,
                shadow: 0,
                score: None,
            }
        },
    );
    let mut phase =
        PartitionedSearchPhase::new(partitioner, PartitionedLifecycleDirector::new, || {
            (MarkCancelledPhase,)
        });

    phase.solve(&mut solver_scope);

    assert_eq!(merge_count.load(Ordering::SeqCst), 0);
    assert_eq!(solver_scope.working_solution().value, 1);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::Cancelled
    );
}

#[test]
fn partitioned_search_does_not_merge_config_terminated_children() {
    let solution = PartitionedLifecycleSolution {
        value: 1,
        shadow: 10,
        score: None,
    };
    let director = PartitionedLifecycleDirector::new(solution);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.initialize_working_solution_as_best();

    let merge_count = Arc::new(AtomicUsize::new(0));
    let merge_count_for_partitioner = Arc::clone(&merge_count);
    let partitioner = FunctionalPartitioner::new(
        |solution: &PartitionedLifecycleSolution| vec![solution.clone()],
        move |_original, _partitions| {
            merge_count_for_partitioner.fetch_add(1, Ordering::SeqCst);
            PartitionedLifecycleSolution {
                value: 99,
                shadow: 0,
                score: None,
            }
        },
    );
    let mut phase =
        PartitionedSearchPhase::new(partitioner, PartitionedLifecycleDirector::new, || {
            (MarkTerminatedPhase,)
        });

    phase.solve(&mut solver_scope);

    assert_eq!(merge_count.load(Ordering::SeqCst), 0);
    assert_eq!(solver_scope.working_solution().value, 1);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}
