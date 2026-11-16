//! Visits that several technicians start together follow every move
//! incrementally: a move on one route re-times the partner visits on the
//! other routes and everything after them, and rescores exactly those rows.

#[path = "joint_cascading_shadows/domain/mod.rs"]
mod domain;

use domain::{define_constraints, Crew, Technician, Visit};
use solverforge::prelude::*;
use solverforge::{
    PlanningSolution, ScoreDirector, SolverEvent, SolverManager, SolverTerminalReason,
};

fn visit(id: usize, duration: i64, partners: &[usize]) -> Visit {
    Visit {
        id,
        duration,
        partners: partners.to_vec(),
        technician: None,
        previous: None,
        next: None,
        start: None,
    }
}

fn starts(crew: &Crew) -> Vec<Option<i64>> {
    crew.visits.iter().map(|visit| visit.start).collect()
}

fn assert_fresh<C: ConstraintSet<Crew, HardSoftScore>>(director: &mut ScoreDirector<Crew, C>) {
    let mut fresh = director.working_solution().clone();
    fresh.update_all_shadows();
    assert_eq!(starts(director.working_solution()), starts(&fresh));
    let incremental = director.calculate_score();
    let mut rescored = ScoreDirector::new(fresh, define_constraints());
    assert_eq!(incremental, rescored.calculate_score());
}

// Ann: visit 0, then the joint visit 2. Bob: the joint visit 3, then
// visit 1. Visit 4 is long and unassigned.
fn ann_and_bob() -> Crew {
    Crew {
        technicians: vec![
            Technician {
                id: 0,
                visits: vec![0, 2],
            },
            Technician {
                id: 1,
                visits: vec![3, 1],
            },
        ],
        visits: vec![
            visit(0, 4, &[]),
            visit(1, 3, &[]),
            visit(2, 6, &[3]),
            visit(3, 6, &[2]),
            visit(4, 10, &[]),
        ],
        score: None,
    }
}

#[test]
fn a_long_visit_on_one_route_holds_back_the_partner_on_the_other() {
    let mut director = ScoreDirector::new(ann_and_bob(), define_constraints());
    director.calculate_score();
    // Ann arrives at the joint visit at 5, Bob at 0: both start at 5.
    assert_eq!(
        starts(director.working_solution()),
        vec![Some(0), Some(12), Some(5), Some(5), None]
    );

    director.before_variable_changed(0, 0);
    director.working_solution_mut().technicians[0]
        .visits
        .insert(0, 4);
    director.after_variable_changed(0, 0);

    // Ann now reaches the joint visit at 16; Bob waits for her.
    assert_eq!(
        starts(director.working_solution()),
        vec![Some(11), Some(23), Some(16), Some(16), Some(0)]
    );
    assert_fresh(&mut director);

    // Undo through the same path.
    director.before_variable_changed(0, 0);
    director.working_solution_mut().technicians[0]
        .visits
        .remove(0);
    director.after_variable_changed(0, 0);
    assert_eq!(
        starts(director.working_solution()),
        vec![Some(0), Some(12), Some(5), Some(5), None]
    );
    assert_fresh(&mut director);
}

#[test]
fn a_solve_with_full_assert_keeps_every_joint_score_fresh() {
    static MANAGER: SolverManager<Crew> = SolverManager::new();
    let technicians = (0..3)
        .map(|id| Technician {
            id,
            visits: Vec::new(),
        })
        .collect();
    // Two pairs and a triple among twelve visits.
    let jobs: [&[usize]; 3] = [&[0, 5], &[2, 7, 10], &[4, 9]];
    let visits = (0..12)
        .map(|id| {
            let partners: Vec<usize> = jobs
                .iter()
                .find(|job| job.contains(&id))
                .map(|job| job.iter().copied().filter(|&p| p != id).collect())
                .unwrap_or_default();
            visit(id, (id as i64 % 4) + 1, &partners)
        })
        .collect();
    let (_, mut receiver) = MANAGER
        .solve(Crew {
            technicians,
            visits,
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
    assert!(solved.visits.iter().all(|visit| visit.technician.is_some()));
}
