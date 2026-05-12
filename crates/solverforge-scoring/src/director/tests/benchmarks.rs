use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::incremental::IncrementalUniConstraint;
use crate::constraint::IncrementalBiConstraint;
use crate::director::score_director::ScoreDirector;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::ConstraintFactory;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use std::hash::Hash;
use std::hint::black_box;
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
        source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        |_sol: &Schedule, s: &Shift| s.employee_id.is_none(),
        |_s: &Shift| SoftScore::of(1),
        false,
    );

    let overlapping = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Overlapping"),
        ImpactType::Penalty,
        source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        |_sol: &Schedule, s: &Shift, _idx: usize| s.employee_id,
        |_sol: &Schedule, a: &Shift, b: &Shift, _ai: usize, _bi: usize| {
            a.id < b.id && a.start_hour < b.end_hour && b.start_hour < a.end_hour
        },
        |_s: &Schedule, _shifts: &[Shift], _a_idx: usize, _b_idx: usize| SoftScore::of(10),
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
            source(
                shifts as fn(&Schedule) -> &[Shift],
                ChangeSource::Descriptor(0),
            ),
            |_sol: &Schedule, s: &Shift| s.employee_id.is_none(),
            |_s: &Shift| SoftScore::of(1),
            false,
        );

        let overlapping = IncrementalBiConstraint::new(
            ConstraintRef::new("", "Overlapping"),
            ImpactType::Penalty,
            source(
                shifts as fn(&Schedule) -> &[Shift],
                ChangeSource::Descriptor(0),
            ),
            |_sol: &Schedule, s: &Shift, _idx: usize| s.employee_id,
            |_sol: &Schedule, a: &Shift, b: &Shift, _ai: usize, _bi: usize| {
                a.id < b.id && a.start_hour < b.end_hour && b.start_hour < a.end_hour
            },
            |_s: &Schedule, _shifts: &[Shift], _a_idx: usize, _b_idx: usize| SoftScore::of(10),
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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct ExistsBenchKey(usize);

#[derive(Clone, Debug)]
struct ExistsBenchState<K> {
    customers: Vec<K>,
    routes: Vec<Vec<K>>,
}

fn exists_bench_customers<K>(state: &ExistsBenchState<K>) -> &[K] {
    state.customers.as_slice()
}

fn exists_bench_routes<K>(state: &ExistsBenchState<K>) -> &[Vec<K>] {
    state.routes.as_slice()
}

fn make_exists_bench_state<K, F>(
    customer_count: usize,
    route_count: usize,
    route_len: usize,
    make_key: F,
) -> ExistsBenchState<K>
where
    K: Copy,
    F: Fn(usize) -> K + Copy,
{
    let customers = (0..customer_count).map(make_key).collect();
    let mut routes = Vec::with_capacity(route_count);
    for route_idx in 0..route_count {
        let mut route = Vec::with_capacity(route_len);
        for offset in 0..route_len {
            route.push(make_key((route_idx * route_len + offset) % customer_count));
        }
        routes.push(route);
    }
    ExistsBenchState { customers, routes }
}

fn rewrite_exists_bench_route<K, F>(
    route: &mut [K],
    move_idx: usize,
    customer_count: usize,
    make_key: F,
) where
    K: Copy,
    F: Fn(usize) -> K + Copy,
{
    let base = (move_idx * 37) % customer_count;
    for (offset, value) in route.iter_mut().enumerate() {
        *value = make_key((base + offset * 17) % customer_count);
    }
}

#[derive(Clone, Copy)]
struct ExistsBenchSpec {
    customer_count: usize,
    route_count: usize,
    route_len: usize,
    moves: usize,
    check_every: usize,
}

fn run_flattened_exists_storage_bench<K, F>(
    label: &str,
    spec: ExistsBenchSpec,
    make_key: F,
) -> (SoftScore, f64)
where
    K: Copy + Clone + Eq + Hash + Send + Sync + 'static,
    F: Fn(usize) -> K + Copy,
{
    let mut state = make_exists_bench_state(
        spec.customer_count,
        spec.route_count,
        spec.route_len,
        make_key,
    );
    let mut constraint = ConstraintFactory::<ExistsBenchState<K>, SoftScore>::new()
        .for_each(source(
            exists_bench_customers::<K> as fn(&ExistsBenchState<K>) -> &[K],
            ChangeSource::Static,
        ))
        .if_not_exists((
            ConstraintFactory::<ExistsBenchState<K>, SoftScore>::new()
                .for_each(source(
                    exists_bench_routes::<K> as fn(&ExistsBenchState<K>) -> &[Vec<K>],
                    ChangeSource::Descriptor(0),
                ))
                .flattened(|route: &Vec<K>| route),
            equal_bi(|customer: &K| *customer, |assigned: &K| *assigned),
        ))
        .penalize(SoftScore::of(1))
        .named(label);

    let mut total = constraint.initialize(&state);
    assert_eq!(total, constraint.evaluate(&state));

    let start = Instant::now();
    for move_idx in 0..spec.moves {
        let route_idx = move_idx % spec.route_count;
        total = total + constraint.on_retract(&state, route_idx, 0);
        rewrite_exists_bench_route(
            &mut state.routes[route_idx],
            move_idx,
            spec.customer_count,
            make_key,
        );
        total = total + constraint.on_insert(&state, route_idx, 0);

        if move_idx % spec.check_every == 0 {
            assert_eq!(total, constraint.evaluate(&state));
        }
        black_box(total);
    }
    let elapsed = start.elapsed();

    assert_eq!(total, constraint.evaluate(&state));
    let moves_per_sec = spec.moves as f64 / elapsed.as_secs_f64();
    eprintln!(
        "{label}: {} route rewrites in {:?} ({:.0} moves/sec), final score {}",
        spec.moves, elapsed, moves_per_sec, total
    );
    (total, moves_per_sec)
}

#[test]
fn bench_exists_usize_storage() {
    let spec = ExistsBenchSpec {
        customer_count: 512,
        route_count: 16,
        route_len: 16,
        moves: 512,
        check_every: 128,
    };
    let (usize_score, usize_rate) =
        run_flattened_exists_storage_bench("exists usize indexed", spec, |idx| idx);
    let (key_score, key_rate) =
        run_flattened_exists_storage_bench("exists newtype hashed", spec, ExistsBenchKey);

    assert_eq!(usize_score, key_score);
    eprintln!(
        "exists storage comparison: usize indexed {:.0} moves/sec, newtype hashed {:.0} moves/sec",
        usize_rate, key_rate
    );
}

#[test]
fn bench_exists_indexed_usize_storage_only() {
    let spec = ExistsBenchSpec {
        customer_count: 512,
        route_count: 16,
        route_len: 16,
        moves: 512,
        check_every: 128,
    };
    let (score, moves_per_sec) =
        run_flattened_exists_storage_bench("exists usize indexed only", spec, |idx| idx);
    eprintln!(
        "exists indexed-only profile target: {:.0} moves/sec, final score {}",
        moves_per_sec, score
    );
}
