//! A start time that depends on the previous task on the same machine and on
//! a task of another machine follows every move incrementally: the director
//! re-times the dependent tasks on other machines and rescores exactly their
//! rows.

#[path = "cascading_list_shadows/domain/mod.rs"]
mod domain;

use domain::{define_constraints, Machine, Shop, Task};
use solverforge::prelude::*;
use solverforge::{
    PlanningSolution, ScoreDirector, SolverEvent, SolverManager, SolverTerminalReason,
};

fn task(id: usize, duration: i64, waits_on: Option<usize>) -> Task {
    Task {
        id,
        duration,
        waits_on,
        machine: None,
        previous: None,
        next: None,
        start: None,
    }
}

fn starts(shop: &Shop) -> Vec<Option<i64>> {
    shop.tasks.iter().map(|task| task.start).collect()
}

fn assert_fresh<C: ConstraintSet<Shop, HardSoftScore>>(director: &mut ScoreDirector<Shop, C>) {
    let mut fresh = director.working_solution().clone();
    fresh.update_all_shadows();
    assert_eq!(starts(director.working_solution()), starts(&fresh));
    let incremental = director.calculate_score();
    let mut rescored = ScoreDirector::new(fresh, define_constraints());
    assert_eq!(incremental, rescored.calculate_score());
}

// Saw (machine 0): cut (0). Mill (machine 1): milling (1), which waits on
// the cut, then task 2. Task 3 is long and unassigned.
fn saw_and_mill() -> Shop {
    Shop {
        machines: vec![
            Machine {
                id: 0,
                tasks: vec![0],
            },
            Machine {
                id: 1,
                tasks: vec![1, 2],
            },
        ],
        tasks: vec![
            task(0, 4, None),
            task(1, 3, Some(0)),
            task(2, 2, None),
            task(3, 10, None),
        ],
        score: None,
    }
}

#[test]
fn inserting_a_long_task_before_the_cut_retimes_the_milling_on_the_other_machine() {
    let mut director = ScoreDirector::new(saw_and_mill(), define_constraints());
    director.calculate_score();
    assert_eq!(
        starts(director.working_solution()),
        vec![Some(0), Some(4), Some(7), None]
    );

    director.before_variable_changed(0, 0);
    director.working_solution_mut().machines[0]
        .tasks
        .insert(0, 3);
    director.after_variable_changed(0, 0);

    assert_eq!(
        starts(director.working_solution()),
        vec![Some(10), Some(14), Some(17), Some(0)]
    );
    assert_fresh(&mut director);

    // Undo through the same path.
    director.before_variable_changed(0, 0);
    director.working_solution_mut().machines[0].tasks.remove(0);
    director.after_variable_changed(0, 0);
    assert_eq!(
        starts(director.working_solution()),
        vec![Some(0), Some(4), Some(7), None]
    );
    assert_fresh(&mut director);
}

#[test]
fn a_solve_with_full_assert_keeps_every_cascading_score_fresh() {
    static MANAGER: SolverManager<Shop> = SolverManager::new();
    let machines = (0..3)
        .map(|id| Machine {
            id,
            tasks: Vec::new(),
        })
        .collect();
    // Every task from 3 on waits on the task three before it, so a chain
    // spans the machines whatever the solver assigns.
    let tasks = (0..12)
        .map(|id| task(id, (id as i64 % 4) + 1, id.checked_sub(3)))
        .collect();
    let (_, mut receiver) = MANAGER
        .solve(Shop {
            machines,
            tasks,
            score: None,
        })
        .expect("job should start");
    let mut completed = None;
    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(
                    metadata.terminal_reason,
                    Some(SolverTerminalReason::Completed)
                );
                completed = Some(solution);
                break;
            }
            SolverEvent::Failed { error, .. } => panic!("the solve failed: {error}"),
            _ => {}
        }
    }
    let solved = completed.expect("expected a completed solution");
    let mut fresh = solved.clone();
    fresh.update_all_shadows();
    assert_eq!(starts(&solved), starts(&fresh));
    assert!(solved.tasks.iter().all(|task| task.machine.is_some()));
}
