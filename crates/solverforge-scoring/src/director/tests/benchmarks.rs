use crate::constraint::incremental::IncrementalUniConstraint;
use crate::constraint::IncrementalBiConstraint;
use crate::director::score_director::ScoreDirector;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use std::time::Instant;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Shift {
    id: usize,
    employee_id: Option<usize>,
    start_hour: u8,
    end_hour: u8,
}

#[derive(Clone, Debug)]
struct Schedule {
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

fn shifts(s: &Schedule) -> &[Shift] {
    s.shifts.as_slice()
}

fn calculate_full(schedule: &Schedule) -> SoftScore {
    let shifts = &schedule.shifts;
    let mut penalty = 0i64;

    for shift in shifts {
        if shift.employee_id.is_none() {
            penalty += 1;
        }
    }

    for i in 0..shifts.len() {
        for j in (i + 1)..shifts.len() {
            let a = &shifts[i];
            let b = &shifts[j];
            if a.employee_id.is_some()
                && a.employee_id == b.employee_id
                && a.start_hour < b.end_hour
                && b.start_hour < a.end_hour
            {
                penalty += 10;
            }
        }
    }

    SoftScore::of(-penalty)
}

fn create_schedule(n: usize) -> Schedule {
    let shifts: Vec<_> = (0..n)
        .map(|i| Shift {
            id: i,
            employee_id: Some(0),
            start_hour: (i % 24) as u8,
            end_hour: ((i % 24) + 1) as u8,
        })
        .collect();

    Schedule {
        shifts,
        score: None,
    }
}

#[test]
fn bench_full_recalc_moves() {
    let n = 100;
    let moves = 1000;

    let mut schedule = create_schedule(n);

    let start = Instant::now();

    for i in 0..moves {
        let shift_idx = i % n;
        let old_employee = schedule.shifts[shift_idx].employee_id;
        schedule.shifts[shift_idx].employee_id = Some((i % 5) + 1);
        let _score = calculate_full(&schedule);
        schedule.shifts[shift_idx].employee_id = old_employee;
    }

    let elapsed = start.elapsed();
    let moves_per_sec = moves as f64 / elapsed.as_secs_f64();

    eprintln!(
        "Full recalc: {} moves in {:?} ({:.0} moves/sec)",
        moves, elapsed, moves_per_sec
    );

    assert!(moves_per_sec > 0.0);
}

#[test]
fn bench_incremental_moves() {
    let n = 100;
    let moves = 1000;

    let schedule = create_schedule(n);

    let unassigned = IncrementalUniConstraint::new(
        ConstraintRef::new("", "Unassigned"),
        ImpactType::Penalty,
        shifts,
        |_sol: &Schedule, s: &Shift| s.employee_id.is_none(),
        |_s: &Shift| SoftScore::of(1),
        false,
    );

    let overlapping = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Overlapping"),
        ImpactType::Penalty,
        shifts,
        |_sol: &Schedule, s: &Shift, _idx: usize| s.employee_id,
        |_sol: &Schedule, a: &Shift, b: &Shift, _ai: usize, _bi: usize| {
            a.id < b.id && a.start_hour < b.end_hour && b.start_hour < a.end_hour
        },
        |_s: &Schedule, _a_idx: usize, _b_idx: usize| SoftScore::of(10),
        false,
    );

    let constraints = (unassigned, overlapping);
    let mut director = ScoreDirector::new(schedule, constraints);

    let initial = director.calculate_score();
    assert_eq!(initial, calculate_full(director.working_solution()));

    let start = Instant::now();

    for i in 0..moves {
        let shift_idx = i % n;
        let old_employee = director.working_solution().shifts[shift_idx].employee_id;

        director.before_variable_changed(0, shift_idx);
        director.working_solution_mut().shifts[shift_idx].employee_id = Some((i % 5) + 1);
        director.after_variable_changed(0, shift_idx);
        let _score = director.get_score();
        director.before_variable_changed(0, shift_idx);
        director.working_solution_mut().shifts[shift_idx].employee_id = old_employee;
        director.after_variable_changed(0, shift_idx);
    }

    let elapsed = start.elapsed();
    let moves_per_sec = moves as f64 / elapsed.as_secs_f64();

    eprintln!(
        "Incremental: {} moves in {:?} ({:.0} moves/sec)",
        moves, elapsed, moves_per_sec
    );

    let final_score = director.get_score();
    assert_eq!(final_score, calculate_full(director.working_solution()));
    assert!(moves_per_sec > 0.0);
}

#[test]
fn bench_compare_approaches() {
    for n in [50, 100, 200] {
        let moves = 500;
        let schedule = create_schedule(n);

        let mut schedule_full = schedule.clone();
        let start = Instant::now();
        for i in 0..moves {
            let shift_idx = i % n;
            let old = schedule_full.shifts[shift_idx].employee_id;
            schedule_full.shifts[shift_idx].employee_id = Some((i % 5) + 1);
            let _score = calculate_full(&schedule_full);
            schedule_full.shifts[shift_idx].employee_id = old;
        }
        let full_elapsed = start.elapsed();

        let unassigned = IncrementalUniConstraint::new(
            ConstraintRef::new("", "Unassigned"),
            ImpactType::Penalty,
            shifts,
            |_sol: &Schedule, s: &Shift| s.employee_id.is_none(),
            |_s: &Shift| SoftScore::of(1),
            false,
        );

        let overlapping = IncrementalBiConstraint::new(
            ConstraintRef::new("", "Overlapping"),
            ImpactType::Penalty,
            shifts,
            |_sol: &Schedule, s: &Shift, _idx: usize| s.employee_id,
            |_sol: &Schedule, a: &Shift, b: &Shift, _ai: usize, _bi: usize| {
                a.id < b.id && a.start_hour < b.end_hour && b.start_hour < a.end_hour
            },
            |_s: &Schedule, _a_idx: usize, _b_idx: usize| SoftScore::of(10),
            false,
        );

        let constraints = (unassigned, overlapping);
        let mut director = ScoreDirector::new(schedule.clone(), constraints);
        director.calculate_score();

        let start = Instant::now();
        for i in 0..moves {
            let shift_idx = i % n;
            let old = director.working_solution().shifts[shift_idx].employee_id;
            director.before_variable_changed(0, shift_idx);
            director.working_solution_mut().shifts[shift_idx].employee_id = Some((i % 5) + 1);
            director.after_variable_changed(0, shift_idx);
            let _score = director.get_score();
            director.before_variable_changed(0, shift_idx);
            director.working_solution_mut().shifts[shift_idx].employee_id = old;
            director.after_variable_changed(0, shift_idx);
        }
        let incr_elapsed = start.elapsed();

        let full_rate = moves as f64 / full_elapsed.as_secs_f64();
        let incr_rate = moves as f64 / incr_elapsed.as_secs_f64();
        let speedup = incr_rate / full_rate;

        eprintln!(
            "n={:3}: Full={:7.0} m/s, Incr={:7.0} m/s, Speedup={:.1}x",
            n, full_rate, incr_rate, speedup
        );
    }
}
