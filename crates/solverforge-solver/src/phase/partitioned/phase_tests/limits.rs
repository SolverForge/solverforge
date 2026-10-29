use super::*;

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

#[test]
fn partitioned_merge_reinitializes_construction_frontier_and_revision() {
    let solution = PartitionedLifecycleSolution {
        value: 1,
        shadow: 10,
        score: None,
    };
    let director = PartitionedLifecycleDirector::new(solution);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.initialize_working_solution_as_best();

    // Mark construction work completed so the merge must invalidate it.
    let slot_id = crate::phase::construction::ConstructionSlotId::new(0, 0);
    let element_id = crate::phase::construction::ConstructionListElementId::new(0, 0);
    solver_scope.mark_scalar_slot_completed(slot_id);
    solver_scope.mark_list_element_completed(element_id);
    assert!(solver_scope.is_scalar_slot_completed(slot_id));

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
            (SetValuePhase { value: 7 },)
        });

    phase.solve(&mut solver_scope);

    // replace_working_solution_and_reinitialize is the wholesale-replacement
    // writer: revision returns to 1 and every frontier entry is invalidated.
    assert_eq!(solver_scope.solution_revision(), 1);
    assert!(
        !solver_scope.is_scalar_slot_completed(slot_id),
        "partition merge must invalidate completed construction slots"
    );
    assert!(
        !solver_scope.is_list_element_completed(element_id),
        "partition merge must invalidate completed list elements"
    );
}
