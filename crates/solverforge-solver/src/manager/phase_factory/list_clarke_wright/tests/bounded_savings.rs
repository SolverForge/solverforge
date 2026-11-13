use std::cell::{Cell, RefCell};

use crate::phase::Phase;
use crate::scope::solver::SolverScope;
use std::any::TypeId;

use solverforge_core::domain::SolutionDescriptor;
use solverforge_scoring::ScoreDirector;

use super::*;

thread_local! {
    static COORDINATES: RefCell<Vec<(i64, i64)>> = const { RefCell::new(Vec::new()) };
    static CAPACITY: Cell<usize> = const { Cell::new(0) };
    /// Set by the neighbour hook, which runs right before savings scoring,
    /// and cleared by the first feasibility check after it, in the merge loop.
    static IN_SAVINGS: Cell<bool> = const { Cell::new(false) };
    static SCORED_PAIRS: RefCell<Vec<(usize, usize)>> = const { RefCell::new(Vec::new()) };
}

fn leg(from: usize, to: usize) -> i64 {
    COORDINATES.with(|coordinates| {
        let coordinates = coordinates.borrow();
        let (a, b) = (coordinates[from], coordinates[to]);
        (a.0 - b.0).abs() + (a.1 - b.1).abs()
    })
}

fn grid_distance(_: &Plan, _: usize, from: usize, to: usize) -> i64 {
    if IN_SAVINGS.with(Cell::get) && from != 0 && to != 0 {
        // Recorded as asked: a scored pair is measured left to right.
        SCORED_PAIRS.with(|pairs| pairs.borrow_mut().push((from, to)));
    }
    leg(from, to)
}

fn capacity_feasible(_: &Plan, _: usize, route: &[usize]) -> bool {
    IN_SAVINGS.with(|flag| flag.set(false));
    route.len() <= CAPACITY.with(Cell::get)
}

/// Brute-force nearest neighbours by leg, ties by lower index.
fn nearest(_: &Plan, _: usize, values: &[usize], k: usize, out: &mut Vec<usize>) {
    IN_SAVINGS.with(|flag| flag.set(true));
    let stride = k.min(values.len().saturating_sub(1));
    for (index, &value) in values.iter().enumerate() {
        let mut others: Vec<usize> = (0..values.len()).filter(|&o| o != index).collect();
        others.sort_by_key(|&other| (leg(value, values[other]), other));
        out.extend_from_slice(&others[..stride]);
    }
}

fn short_neighbors(_: &Plan, _: usize, _: &[usize], _: usize, out: &mut Vec<usize>) {
    out.push(1);
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

/// Seeds a random instance: `customers` values on a grid plus the depot 0.
fn seed_instance(seed: u64, customers: usize, vehicles: usize) -> Plan {
    let mut rng = Lcg(seed);
    COORDINATES.with(|coordinates| {
        *coordinates.borrow_mut() = (0..=customers)
            .map(|_| (rng.next(100) as i64, rng.next(100) as i64))
            .collect();
    });
    CAPACITY.with(|capacity| capacity.set(customers.div_ceil(vehicles) + 1));
    Plan {
        customer_values: (1..=customers).collect(),
        routes: (0..vehicles)
            .map(|_| Route { visits: Vec::new() })
            .collect(),
        score: None,
    }
}

/// Solves and returns the routes plus the customer pairs savings scored
/// (recorded for bounded runs only).
fn solve(plan: Plan, savings_neighbors: Option<usize>) -> (Vec<Vec<usize>>, Vec<(usize, usize)>) {
    SCORED_PAIRS.with(|pairs| pairs.borrow_mut().clear());
    let director = ScoreDirector::simple(
        plan,
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
        |s, descriptor_index| {
            if descriptor_index == 0 {
                s.routes.len()
            } else {
                0
            }
        },
    );
    let mut solver_scope = SolverScope::new(director);
    let phase = ListClarkeWrightPhase::new(
        element_count,
        get_assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        crate::builder::context::list::usize_element_source_key,
        depot,
        grid_distance,
        capacity_feasible,
        0,
    );
    let mut phase = match savings_neighbors {
        Some(k) => phase.with_savings_neighbors(nearest, k),
        None => phase,
    };
    phase.solve(&mut solver_scope);
    let routes = solver_scope
        .working_solution()
        .routes
        .iter()
        .map(|route| route.visits.clone())
        .collect();
    (routes, SCORED_PAIRS.with(|pairs| pairs.take()))
}

#[test]
fn a_bound_covering_every_pair_reproduces_unbounded_clarke_wright() {
    for seed in 0..40 {
        let customers = 3 + (seed as usize % 23);
        let vehicles = 1 + (seed as usize % 4);
        let (unbounded, _) = solve(seed_instance(seed, customers, vehicles), None);
        for k in [customers - 1, customers + 3] {
            let (bounded, _) = solve(seed_instance(seed, customers, vehicles), Some(k));
            assert_eq!(
                bounded, unbounded,
                "seed {seed}, {customers} customers, k = {k}"
            );
        }
    }
}

#[test]
fn bounded_savings_score_each_neighbouring_pair_once() {
    for seed in 0..20 {
        let customers = 12 + (seed as usize % 9);
        let vehicles = 1 + (seed as usize % 3);
        for k in [1, 2, 4] {
            let plan = seed_instance(seed, customers, vehicles);
            let mut rows = Vec::new();
            nearest(&plan, 0, &plan.customer_values, k, &mut rows);
            let mut expected = std::collections::BTreeSet::new();
            for (a, row) in rows.chunks(k).enumerate() {
                for &b in row {
                    // Values are customers 1..=n at positions 0..n.
                    expected.insert((a.min(b) + 1, a.max(b) + 1));
                }
            }
            let (routes, scored) = solve(plan, Some(k));
            // Every vehicle is its own metric class here; each scores the pairs.
            assert_eq!(
                scored.len(),
                expected.len() * vehicles,
                "each pair once per class: seed {seed}, k = {k}"
            );
            let scored: std::collections::BTreeSet<_> = scored.into_iter().collect();
            assert_eq!(scored, expected, "seed {seed}, k = {k}");
            let mut assigned: Vec<usize> = routes.concat();
            assigned.sort_unstable();
            assert_eq!(
                assigned,
                (1..=customers).collect::<Vec<_>>(),
                "seed {seed}, k = {k}"
            );
        }
    }
}

#[test]
#[should_panic(expected = "savings_neighbors_fn must append")]
fn a_neighbour_hook_with_the_wrong_row_length_is_rejected() {
    let plan = seed_instance(7, 6, 2);
    let director = ScoreDirector::simple(
        plan,
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
        |s, descriptor_index| {
            if descriptor_index == 0 {
                s.routes.len()
            } else {
                0
            }
        },
    );
    let mut solver_scope = SolverScope::new(director);
    let mut phase = ListClarkeWrightPhase::new(
        element_count,
        get_assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        crate::builder::context::list::usize_element_source_key,
        depot,
        grid_distance,
        capacity_feasible,
        0,
    )
    .with_savings_neighbors(short_neighbors, 3);
    phase.solve(&mut solver_scope);
}
