//! Performance comparison: full recalc vs incremental scoring.
//!
//! This module demonstrates the performance difference between:
//! - Full recalculation on every move (O(n) or O(n²) per move)
//! - Incremental delta scoring (O(affected entities) per move)

#[cfg(test)]
mod benchmarks {
    use crate::constraint::incremental::IncrementalUniConstraint;
    use crate::constraint::IncrementalBiConstraint;
    use crate::director::typed::TypedScoreDirector;
    use solverforge_core::domain::PlanningSolution;
    use solverforge_core::score::SimpleScore;
    use solverforge_core::{ConstraintRef, ImpactType};
    use std::time::Instant;

    // ========================================================================
    // Test domain: Simplified employee scheduling
    // ========================================================================

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
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for Schedule {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    /// Full recalculation scoring function (for comparison).
    fn calculate_full(schedule: &Schedule) -> SimpleScore {
        let shifts = &schedule.shifts;
        let mut penalty = 0i64;

        // Constraint 1: Penalize unassigned shifts
        for shift in shifts {
            if shift.employee_id.is_none() {
                penalty += 1;
            }
        }

        // Constraint 2: Penalize overlapping shifts for same employee
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

        SimpleScore::of(-penalty)
    }

    /// Creates a schedule with n shifts, all assigned to the same employee.
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

    // ========================================================================
    // Benchmark: Full recalculation
    // ========================================================================

    #[test]
    fn bench_full_recalc_moves() {
        let n = 100; // Number of shifts
        let moves = 1000; // Number of simulated moves

        let mut schedule = create_schedule(n);

        let start = Instant::now();

        for i in 0..moves {
            // Simulate a move: change assignment of one shift
            let shift_idx = i % n;
            let old_employee = schedule.shifts[shift_idx].employee_id;
            schedule.shifts[shift_idx].employee_id = Some((i % 5) + 1);

            // Full recalculation
            let _score = calculate_full(&schedule);

            // Undo move
            schedule.shifts[shift_idx].employee_id = old_employee;
        }

        let elapsed = start.elapsed();
        let moves_per_sec = moves as f64 / elapsed.as_secs_f64();

        eprintln!(
            "Full recalc: {} moves in {:?} ({:.0} moves/sec)",
            moves, elapsed, moves_per_sec
        );

        // Just ensure it ran
        assert!(moves_per_sec > 0.0);
    }

    // ========================================================================
    // Benchmark: Incremental scoring
    // ========================================================================

    #[test]
    fn bench_incremental_moves() {
        let n = 100;
        let moves = 1000;

        let schedule = create_schedule(n);

        // Create zero-erasure constraints
        let unassigned = IncrementalUniConstraint::new(
            ConstraintRef::new("", "Unassigned"),
            ImpactType::Penalty,
            |s: &Schedule| s.shifts.as_slice(),
            |_sol: &Schedule, s: &Shift| s.employee_id.is_none(),
            |_s: &Shift| SimpleScore::of(1),
            false,
        );

        let overlapping = IncrementalBiConstraint::new(
            ConstraintRef::new("", "Overlapping"),
            ImpactType::Penalty,
            |s: &Schedule| s.shifts.as_slice(),
            |s: &Shift| s.employee_id, // Key by employee_id
            |_sol: &Schedule, a: &Shift, b: &Shift| {
                a.id < b.id // Ordering to avoid duplicates
                    && a.start_hour < b.end_hour
                    && b.start_hour < a.end_hour
            },
            |_a: &Shift, _b: &Shift| SimpleScore::of(10),
            false,
        );

        let constraints = (unassigned, overlapping);

        let mut director = TypedScoreDirector::new(schedule, constraints);

        // Initialize
        let initial = director.calculate_score();
        assert_eq!(initial, calculate_full(director.working_solution()));

        let start = Instant::now();

        for i in 0..moves {
            let shift_idx = i % n;
            let old_employee = director.working_solution().shifts[shift_idx].employee_id;

            // Incremental move: retract, change, insert
            director.before_variable_changed(shift_idx);
            director.working_solution_mut().shifts[shift_idx].employee_id = Some((i % 5) + 1);
            director.after_variable_changed(shift_idx);

            // Score is already updated (O(1) to get)
            let _score = director.get_score();

            // Undo move incrementally
            director.before_variable_changed(shift_idx);
            director.working_solution_mut().shifts[shift_idx].employee_id = old_employee;
            director.after_variable_changed(shift_idx);
        }

        let elapsed = start.elapsed();
        let moves_per_sec = moves as f64 / elapsed.as_secs_f64();

        eprintln!(
            "Incremental: {} moves in {:?} ({:.0} moves/sec)",
            moves, elapsed, moves_per_sec
        );

        // Verify final score matches full calculation
        let final_score = director.get_score();
        assert_eq!(final_score, calculate_full(director.working_solution()));

        assert!(moves_per_sec > 0.0);
    }

    // ========================================================================
    // Benchmark: Compare speedup
    // ========================================================================

    #[test]
    fn bench_compare_approaches() {
        for n in [50, 100, 200] {
            let moves = 500;
            let schedule = create_schedule(n);

            // Full recalc timing
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

            // Incremental timing
            let unassigned = IncrementalUniConstraint::new(
                ConstraintRef::new("", "Unassigned"),
                ImpactType::Penalty,
                |s: &Schedule| s.shifts.as_slice(),
                |_sol: &Schedule, s: &Shift| s.employee_id.is_none(),
                |_s: &Shift| SimpleScore::of(1),
                false,
            );

            let overlapping = IncrementalBiConstraint::new(
                ConstraintRef::new("", "Overlapping"),
                ImpactType::Penalty,
                |s: &Schedule| s.shifts.as_slice(),
                |s: &Shift| s.employee_id,
                |_sol: &Schedule, a: &Shift, b: &Shift| {
                    a.id < b.id && a.start_hour < b.end_hour && b.start_hour < a.end_hour
                },
                |_a: &Shift, _b: &Shift| SimpleScore::of(10),
                false,
            );

            let constraints = (unassigned, overlapping);
            let mut director = TypedScoreDirector::new(schedule.clone(), constraints);
            director.calculate_score();

            let start = Instant::now();
            for i in 0..moves {
                let shift_idx = i % n;
                let old = director.working_solution().shifts[shift_idx].employee_id;
                director.before_variable_changed(shift_idx);
                director.working_solution_mut().shifts[shift_idx].employee_id = Some((i % 5) + 1);
                director.after_variable_changed(shift_idx);
                let _score = director.get_score();
                director.before_variable_changed(shift_idx);
                director.working_solution_mut().shifts[shift_idx].employee_id = old;
                director.after_variable_changed(shift_idx);
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
}
