use super::*;
use solverforge_solver::CrossEntityDistanceMeter;

struct Fleet<M> {
    data: ProblemData<M>,
    routes: Vec<Vec<usize>>,
}

impl<M: RouteMetric> VrpSolution for Fleet<M> {
    type Metric = M;

    fn vehicle_data(&self, _entity_idx: usize) -> Option<&ProblemData<M>> {
        Some(&self.data)
    }

    fn vehicle_visits(&self, entity_idx: usize) -> &[usize] {
        &self.routes[entity_idx]
    }

    fn vehicle_visits_mut(&mut self, entity_idx: usize) -> &mut Vec<usize> {
        &mut self.routes[entity_idx]
    }

    fn vehicle_count(&self) -> usize {
        self.routes.len()
    }
}

fn coordinates() -> Vec<(f64, f64)> {
    vec![
        (0.0, 0.0),
        (3.0, 4.0),
        (6.0, 8.0),
        (0.5, 0.0),
        (-2.5, 0.0),
        (1.0, 1.0),
    ]
}

fn problem_data<M>(metric: M) -> ProblemData<M> {
    ProblemData {
        capacity: 9,
        depot: 0,
        demands: vec![0, 2, 3, 4, 1, 2],
        metric,
        time_windows: vec![(0, 100), (0, 40), (5, 60), (0, 30), (0, 50), (0, 70)],
        service_durations: vec![0, 2, 2, 3, 1, 1],
        vehicle_departure_time: 0,
    }
}

/// The dense tables an `EuclideanMetric` stands for, built from its own legs.
fn tabulated(metric: &EuclideanMetric) -> DenseMatrix {
    let n = metric.coordinates().len();
    let table = |leg: &dyn Fn(usize, usize) -> Option<i64>| -> Vec<Vec<i64>> {
        (0..n)
            .map(|from| (0..n).map(|to| leg(from, to).unwrap()).collect())
            .collect()
    };
    DenseMatrix::new(
        table(&|from, to| metric.distance(from, to)),
        table(&|from, to| metric.travel_time(from, to)),
    )
}

#[test]
fn euclidean_legs_round_half_away_from_zero() {
    let metric = EuclideanMetric::new(coordinates(), 2.0);

    assert_eq!(metric.distance(0, 1), Some(5));
    assert_eq!(metric.distance(1, 2), Some(5));
    assert_eq!(metric.distance(0, 2), Some(10));
    assert_eq!(metric.distance(0, 3), Some(1));
    assert_eq!(metric.distance(0, 4), Some(3));
    assert_eq!(metric.distance(0, 5), Some(1));
    assert_eq!(metric.travel_time(0, 1), Some(3));
    assert_eq!(metric.travel_time(0, 2), Some(5));
    assert_eq!(metric.travel_time(0, 3), Some(0));
    assert_eq!(metric.distance(3, 3), Some(0));
}

#[test]
fn euclidean_legs_outside_the_coordinates_are_undefined() {
    let metric = EuclideanMetric::new(coordinates(), 1.0);

    assert_eq!(metric.distance(0, 6), None);
    assert_eq!(metric.travel_time(6, 0), None);
    assert!(metric.covers(&[1, 5], 0, 5));
    assert!(!metric.covers(&[1, 6], 0, 6));
}

#[test]
#[should_panic(expected = "speed must be finite and positive")]
fn euclidean_metric_rejects_a_non_positive_speed() {
    EuclideanMetric::new(coordinates(), 0.0);
}

#[test]
#[should_panic(expected = "coordinates must be finite")]
fn euclidean_metric_rejects_non_finite_coordinates() {
    EuclideanMetric::new(vec![(0.0, f64::NAN)], 1.0);
}

#[test]
fn dense_matrix_covers_exactly_the_supplied_rows() {
    let metric = DenseMatrix::new(
        vec![vec![0, 1, 2], vec![1, 0, 3], vec![2, 3]],
        vec![vec![0, 1, 2], vec![1, 0, 3], vec![2, 3, 0]],
    );

    assert!(metric.covers(&[1], 0, 2));
    assert!(!metric.covers(&[2], 0, 2));
    assert!(!metric.covers(&[3], 0, 3));
    assert_eq!(metric.distance(2, 2), None);
    assert_eq!(metric.travel_time(2, 2), Some(0));
}

#[test]
fn euclidean_metric_answers_every_stock_helper_like_its_tabulated_matrix() {
    let euclidean = EuclideanMetric::new(coordinates(), 2.0);
    let dense = tabulated(&euclidean);
    let routes = vec![vec![1, 2], vec![3, 4, 5], vec![2, 5, 1, 3], vec![]];
    let on_demand = Fleet {
        data: problem_data(euclidean),
        routes: routes.clone(),
    };
    let tabled = Fleet {
        data: problem_data(dense),
        routes,
    };
    let candidates: [&[usize]; 6] = [&[], &[1], &[1, 2], &[3, 4, 5], &[2, 5, 1, 3], &[5, 4, 3, 2]];

    for entity in 0..on_demand.vehicle_count() {
        assert_eq!(
            route_distance(&on_demand, entity, 0, 0),
            route_distance(&tabled, entity, 0, 0)
        );
        for &route in &candidates {
            assert_eq!(
                route_feasible(&on_demand, entity, route),
                route_feasible(&tabled, entity, route)
            );
            assert_eq!(
                savings_feasible(&on_demand, entity, route),
                savings_feasible(&tabled, entity, route)
            );
        }
        for from in 0..6 {
            for to in 0..6 {
                assert_eq!(
                    route_distance(&on_demand, entity, from, to),
                    route_distance(&tabled, entity, from, to)
                );
                assert_eq!(
                    savings_distance(&on_demand, entity, from, to),
                    savings_distance(&tabled, entity, from, to)
                );
            }
        }
    }
    for (src_entity, src_pos, dst_entity, dst_pos) in [(0, 0, 1, 2), (2, 3, 2, 0), (1, 1, 0, 1)] {
        assert_eq!(
            MatrixDistanceMeter.distance(&on_demand, src_entity, src_pos, dst_entity, dst_pos),
            MatrixDistanceMeter.distance(&tabled, src_entity, src_pos, dst_entity, dst_pos)
        );
        assert_eq!(
            MatrixIntraDistanceMeter.distance(&on_demand, src_entity, src_pos, src_entity, dst_pos),
            MatrixIntraDistanceMeter.distance(&tabled, src_entity, src_pos, src_entity, dst_pos)
        );
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % bound
    }
}

/// Coordinates spread, clustered, duplicated, on a line, and on a fine
/// lattice where many legs round to the same distance.
fn shapes(seed: u64, count: usize) -> Vec<Vec<(f64, f64)>> {
    let mut rng = Lcg(seed);
    let spread = (0..count)
        .map(|_| (rng.next(10_000) as f64 / 7.0, rng.next(10_000) as f64 / 3.0))
        .collect();
    let clustered = (0..count)
        .map(|i| {
            let centre = (i % 3) as f64 * 5_000.0;
            (centre + rng.next(20) as f64, centre + rng.next(20) as f64)
        })
        .collect();
    let duplicated = (0..count)
        .map(|_| (rng.next(4) as f64, rng.next(4) as f64))
        .collect();
    let line = (0..count)
        .map(|_| (rng.next(1_000) as f64 * 0.5, 7.0))
        .collect();
    vec![spread, clustered, duplicated, line]
}

#[test]
fn euclidean_grid_neighbours_match_the_ranked_scan_exactly() {
    for seed in 0..12 {
        for count in [1, 2, 3, 17, 160, 300] {
            for coordinates in shapes(seed, count) {
                let metric = EuclideanMetric::new(coordinates, 1.0);
                let mut locations: Vec<usize> = (0..count).collect();
                // Permute so location order differs from index order.
                locations.rotate_left(seed as usize % count);
                for k in [1, 2, 5, 40, count + 2] {
                    let mut grid = Vec::new();
                    metric.nearest(&locations, k, &mut grid);
                    let mut scan = Vec::new();
                    crate::metric::nearest_by_rank(&metric, &locations, k, &mut scan);
                    assert_eq!(grid, scan, "seed {seed}, {count} locations, k = {k}");
                    assert_eq!(grid.len(), count * k.min(count.saturating_sub(1)));
                }
            }
        }
    }
}

#[test]
fn euclidean_grid_neighbours_match_where_many_legs_round_alike() {
    for seed in 0..40 {
        let mut rng = Lcg(seed);
        let count = 50 + (seed as usize * 7) % 300;
        let coordinates = (0..count)
            .map(|_| {
                (
                    rng.next(1_000) as f64 * 0.0137,
                    rng.next(1_000) as f64 * 0.0113,
                )
            })
            .collect();
        let metric = EuclideanMetric::new(coordinates, 1.0);
        let locations: Vec<usize> = (0..count).collect();
        for k in [3, 10, 40] {
            let (mut grid, mut scan) = (Vec::new(), Vec::new());
            metric.nearest(&locations, k, &mut grid);
            crate::metric::nearest_by_rank(&metric, &locations, k, &mut scan);
            assert_eq!(grid, scan, "seed {seed}, {count} locations, k = {k}");
        }
    }
}

#[test]
fn ranked_neighbours_order_by_savings_distance_then_index() {
    let metric = DenseMatrix::new(
        vec![
            vec![0, 4, 4, UNREACHABLE],
            vec![4, 0, -1, 2],
            vec![4, 1, 0, 9],
            vec![1, 2, 3, 0],
        ],
        vec![vec![0; 4]; 4],
    );
    let mut out = Vec::new();
    metric.nearest(&[0, 1, 2, 3], 3, &mut out);
    assert_eq!(out, vec![1, 2, 3, 3, 0, 2, 1, 0, 3, 0, 1, 2]);
    out.clear();
    metric.nearest(&[3, 0], 5, &mut out);
    assert_eq!(out, vec![1, 0]);
}

#[test]
fn savings_neighbors_reads_the_vehicles_metric() {
    let fleet = Fleet {
        data: problem_data(EuclideanMetric::new(coordinates(), 1.0)),
        routes: vec![Vec::new(), Vec::new()],
    };
    let mut out = Vec::new();
    savings_neighbors(&fleet, 1, &[1, 2, 3], 1, &mut out);
    assert_eq!(out, vec![1, 0, 0]);
    out.clear();
    savings_neighbors(&fleet, 2, &[1, 2, 3], 1, &mut out);
    assert!(out.is_empty());
}
