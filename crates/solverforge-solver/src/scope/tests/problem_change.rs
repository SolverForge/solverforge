// Real-time problem-change control and application on the solver scope.

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_scoring::Director;

use crate::manager::{SolverManagerError, SolverRuntime, SolverTerminalReason};
use crate::realtime::ClosureProblemChange;
use crate::scope::{PendingControl, SolverScope};
use crate::test_utils::{
    create_nqueens_director, create_simple_nqueens_director, NQueensSolution, Queen,
};

fn queue_change(
    runtime: SolverRuntime<NQueensSolution>,
    id: i64,
) -> Result<crate::manager::ProblemChangeId, SolverManagerError> {
    runtime.submit_problem_change_for_test(Box::new(
        ClosureProblemChange::<NQueensSolution, _>::new("add queen", move |director| {
            director
                .working_solution_mut()
                .queens
                .push(Queen::unassigned(id, id));
        }),
    ))
}

#[test]
fn queued_change_ranks_below_cancellation_and_configured_limits() {
    let runtime = SolverRuntime::detached();
    let mut scope = SolverScope::new(create_simple_nqueens_director(4)).with_runtime(Some(runtime));
    scope.start_solving();
    assert_eq!(scope.pending_control(), PendingControl::Continue);
    assert!(!scope.work_should_stop());

    queue_change(runtime, 4).expect("solving job accepts a change");
    assert_eq!(
        scope.pending_control(),
        PendingControl::ProblemChangeRequested
    );

    scope.set_time_limit(Duration::ZERO);
    assert_eq!(
        scope.pending_control(),
        PendingControl::ConfigTerminationRequested
    );
    assert!(scope.should_terminate());
    assert!(
        !scope.yielded_to_parent(),
        "a fired configured limit is recorded instead of yielding"
    );
    assert_eq!(
        scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
    assert!(
        !scope.problem_change_interrupt_pending(),
        "a configured stop must keep its completion-gate semantics"
    );

    let cancelled = AtomicBool::new(true);
    let scope = scope.with_terminate(Some(&cancelled));
    assert_eq!(scope.pending_control(), PendingControl::CancelRequested);
    assert!(
        scope.take_problem_changes().is_none(),
        "a stopped solve closes its queue instead of applying it"
    );
    assert!(queue_change(runtime, 5).is_err());
}

#[test]
fn queued_change_stops_work_without_a_terminal_reason() {
    let runtime = SolverRuntime::detached();
    let mut scope = SolverScope::new(create_simple_nqueens_director(4)).with_runtime(Some(runtime));
    scope.start_solving();
    let id = queue_change(runtime, 4).expect("solving job accepts a change");
    assert_eq!(id.get(), 1);

    assert!(scope.should_terminate());
    assert!(
        scope.work_should_stop(),
        "the settled change latches the yield"
    );
    assert!(scope.should_terminate_construction());
    assert_eq!(scope.terminal_reason(), SolverTerminalReason::Completed);
    assert!(scope.problem_change_interrupt_pending());

    let changes = scope
        .take_problem_changes()
        .expect("an accepting solve drains its queue");
    assert_eq!(changes.len(), 1);
    assert!(!scope.problem_change_interrupt_pending());
    scope
        .apply_problem_changes(changes)
        .expect("changes without constraint weights apply");
    assert!(
        !scope.yielded_to_parent(),
        "applying a batch spends the yield"
    );
    assert!(!scope.should_terminate());
    assert!(!scope.work_should_stop());

    assert!(
        scope.take_problem_changes().is_none(),
        "an empty queue at the end of a pass closes"
    );
    assert!(matches!(
        queue_change(runtime, 5),
        Err(SolverManagerError::InvalidStateTransition {
            action: "add_problem_change",
            ..
        })
    ));
}

#[test]
fn child_scope_yields_to_its_parent_and_never_drains() {
    let runtime = SolverRuntime::detached();
    let parent = SolverScope::new(create_simple_nqueens_director(4)).with_runtime(Some(runtime));
    let budget = parent.child_phase_budget();
    let mut child = parent
        .child_config(Some(&budget))
        .build_scope(create_simple_nqueens_director(4), 7);
    queue_change(runtime, 4).expect("solving job accepts a change");

    assert!(child.should_terminate());
    assert!(child.yielded_to_parent());
    assert_eq!(child.terminal_reason(), SolverTerminalReason::Completed);
    assert!(child.take_problem_changes().is_none());
    assert!(
        runtime.is_problem_change_pending(),
        "the change stays queued for the top-level runner"
    );
}

#[test]
fn applying_changes_starts_from_best_in_fifo_order_and_drops_the_old_best() {
    let runtime = SolverRuntime::detached();
    let mut scope =
        SolverScope::new(create_nqueens_director(&[0, 1, 2, 3])).with_runtime(Some(runtime));
    scope.initialize_working_solution_as_best();
    scope.mutate(|director| director.working_solution_mut().queens[0].row = Some(3));
    let generation = scope.working_solution_generation();
    queue_change(runtime, 4).expect("first change queues");
    runtime
        .submit_problem_change_for_test(Box::new(ClosureProblemChange::<NQueensSolution, _>::new(
            "move the added queen",
            |director| {
                let queens = &mut director.working_solution_mut().queens;
                let added = queens
                    .last_mut()
                    .expect("the first change ran before this one");
                added.row = Some(2);
            },
        )))
        .expect("second change queues");

    let changes = scope
        .take_problem_changes()
        .expect("an accepting solve drains its queue");
    let score = scope
        .apply_problem_changes(changes)
        .expect("changes without constraint weights apply");

    let queens = &scope.working_solution().queens;
    assert_eq!(queens.len(), 5);
    assert_eq!(queens[0].row, Some(0), "changes apply to the best solution");
    assert_eq!(queens[4].row, Some(2), "changes apply in submission order");
    assert!(scope.best_solution().is_none());
    assert!(scope.best_score().is_none());
    assert!(!scope.best_solution_publication_enabled());
    assert!(scope.working_solution_generation() > generation);
    assert_eq!(scope.solution_revision(), 1);
    assert_eq!(scope.current_score().copied(), Some(score));
    assert_eq!(scope.score_director().fresh_score(), Some(score));
    assert_eq!(scope.stats().problem_changes_applied(), 2);
    assert_eq!(scope.stats().snapshot().problem_changes_applied, 2);
}

#[test]
fn pause_outranks_a_queued_change_in_the_shared_control_word() {
    let runtime = SolverRuntime::detached();
    let parent = SolverScope::new(create_simple_nqueens_director(4)).with_runtime(Some(runtime));
    let budget = parent.child_phase_budget();
    let mut child = parent
        .child_config(Some(&budget))
        .build_scope(create_simple_nqueens_director(4), 7);
    queue_change(runtime, 4).expect("solving job accepts a change");
    runtime.request_pause_for_test();

    assert_eq!(child.pending_control(), PendingControl::PauseRequested);
    assert!(child.should_terminate());
    assert!(child.yielded_to_parent());
    assert_eq!(child.terminal_reason(), SolverTerminalReason::Completed);
    assert!(runtime.is_pause_requested());
    assert!(runtime.is_problem_change_pending());
}
