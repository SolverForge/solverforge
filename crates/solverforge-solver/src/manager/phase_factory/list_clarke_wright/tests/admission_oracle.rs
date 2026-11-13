//! Differential check of the incremental merge admission against the
//! whole-route-set recomputation it replaced, on seeded random instances.

use std::cell::Cell;

use super::super::owner_admission::{MergeProposal, OwnerAdmission, OwnerQuery};
use super::super::owner_assignment::{
    feasible_owners_for_scored_elements, match_route_owners, owner_slots, OwnerSlot,
    SavingsFeasibility,
};
use super::super::route_state::{route_elements_into, route_values, ConstructedRoute};
use super::*;
use crate::builder::context::bind_runtime_list_source;

thread_local! {
    static SEED: Cell<u64> = const { Cell::new(0) };
    static CLASSES: Cell<usize> = const { Cell::new(1) };
}

fn route_elements<'source, A>(
    source_index: &'source crate::builder::context::RuntimeListSourceIndex<A::Element>,
    route: &[usize],
) -> Vec<&'source A::Element>
where
    A: ClarkeWrightAccess<Plan>,
{
    let mut elements = Vec::new();
    route_elements_into::<Plan, A>(source_index, route, &mut elements);
    elements
}

fn mix(mut value: u64) -> u64 {
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51_afd7_ed55_8ccd);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    value ^ (value >> 33)
}

fn route_key(route: &[usize]) -> u64 {
    let mut sorted = route.to_vec();
    sorted.sort_unstable();
    sorted
        .iter()
        .fold(SEED.with(Cell::get), |acc, &value| mix(acc ^ value as u64))
}

fn random_class(_: &Plan, entity_idx: usize) -> usize {
    entity_idx % CLASSES.with(Cell::get)
}

/// Feasibility that varies per owner: short routes pass more often.
fn per_owner_feasible(_: &Plan, entity_idx: usize, route: &[usize]) -> bool {
    mix(route_key(route) ^ (entity_idx as u64) << 40) % (route.len() as u64 + 2) < 2
}

/// Feasibility shared by every owner of one metric class.
fn per_class_feasible(plan: &Plan, entity_idx: usize, route: &[usize]) -> bool {
    let class = random_class(plan, entity_idx) as u64;
    mix(route_key(route) ^ class << 40) % (route.len() as u64 + 2) < 2
}

fn sometimes_fixed_owner(_: &Plan, element: &usize) -> Option<usize> {
    let roll = mix(SEED.with(Cell::get) ^ (*element as u64) << 20);
    roll.is_multiple_of(5).then_some((roll >> 8) as usize % 4)
}

fn hash_distance(_: &Plan, _: usize, a: usize, b: usize) -> i64 {
    (mix((a.min(b) as u64) << 32 | b.max(a) as u64) % 100) as i64
}

/// The admission rule as the merge loop evaluated it before it kept state:
/// derive every live route's owners against every owner on each attempt.
#[allow(clippy::too_many_arguments)]
fn recomputed_admission<A>(
    access: &A,
    plan: &Plan,
    source_index: &crate::builder::context::RuntimeListSourceIndex<A::Element>,
    routes: &[ConstructedRoute],
    proposal: &MergeProposal<'_>,
    candidate_route: &[usize],
    owner_slots: &[OwnerSlot],
    entity_count: usize,
) -> bool
where
    A: ClarkeWrightAccess<Plan>,
{
    let mut route_count_by_class = std::collections::BTreeMap::new();
    let mut owner_count_by_class = std::collections::BTreeMap::new();
    for slot in owner_slots {
        *owner_count_by_class
            .entry(slot.metric_class)
            .or_insert(0usize) += 1;
    }
    let mut live = 0usize;
    let mut fast = true;
    for (route_idx, route) in routes.iter().enumerate() {
        if route_idx == proposal.removed || route.visits.is_empty() {
            continue;
        }
        live += 1;
        let (class, all_class) = if route_idx == proposal.merged {
            (
                Some(proposal.metric_class),
                proposal.feasible_for_all_metric_class_owners,
            )
        } else {
            (
                route.scored_metric_class,
                route.feasible_for_all_metric_class_owners,
            )
        };
        match class {
            Some(class) if all_class => *route_count_by_class.entry(class).or_insert(0usize) += 1,
            Some(_) => fast = false,
            None if route.feasible_for_all_owners => {}
            None => fast = false,
        }
    }
    if fast
        && live <= owner_slots.len()
        && route_count_by_class.iter().all(|(class, count)| {
            owner_count_by_class
                .get(class)
                .is_some_and(|owners| count <= owners)
        })
    {
        return true;
    }

    let mut feasibility = SavingsFeasibility::new(access, owner_slots);
    let mut sets = Vec::new();
    for (route_idx, route) in routes.iter().enumerate() {
        let (visits, class) = if route_idx == proposal.merged {
            (candidate_route, Some(proposal.metric_class))
        } else if route_idx == proposal.removed || route.visits.is_empty() {
            continue;
        } else {
            (route.visits.as_slice(), route.scored_metric_class)
        };
        let values = route_values(access, source_index, visits);
        let elements = route_elements::<A>(source_index, visits);
        let owners = feasible_owners_for_scored_elements(
            access,
            plan,
            owner_slots,
            &mut feasibility,
            &values,
            &elements,
            class,
            entity_count,
        );
        if owners.is_empty() {
            return false;
        }
        sets.push(owners);
    }
    if sets.len() > owner_slots.len() {
        return true;
    }
    match_route_owners(&sets).iter().all(Option::is_some)
}

fn run_case(seed: u64, shared_by_class: bool) -> (usize, usize) {
    SEED.with(|cell| cell.set(seed));
    let roll = mix(seed);
    let n_elements = 3 + (roll % 12) as usize;
    let owner_count = 1 + ((roll >> 8) % 5) as usize;
    CLASSES.with(|cell| cell.set(1 + ((roll >> 16) % 3) as usize));
    let plan = Plan {
        customer_values: (0..n_elements).collect(),
        routes: vec![Route { visits: Vec::new() }; owner_count],
        score: None,
    };
    let feasible = if shared_by_class {
        per_class_feasible
    } else {
        per_owner_feasible
    };
    let access = ListClarkeWrightPhase::new(
        element_count,
        get_assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        |_, element| *element,
        depot,
        hash_distance,
        feasible,
        0,
    )
    .with_metric_class_fn(random_class)
    .with_element_owner_fn(Some(sometimes_fixed_owner))
    .with_savings_feasible_shared_by_metric_class(shared_by_class);
    let source_index = bind_runtime_list_source(&access, &plan)
        .expect("test source binds")
        .into_source_index();
    let slots = owner_slots(&access, &plan, &(0..owner_count).collect::<Vec<_>>());
    let mut feasibility = SavingsFeasibility::new(&access, &slots);
    let mut routes = (0..n_elements)
        .map(|element| {
            let singleton = [element];
            let all = slots.iter().all(|slot| {
                feasible(&plan, slot.owner_idx, &singleton)
                    && super::super::element_restriction(
                        &access,
                        &plan,
                        owner_count,
                        source_index.element(element),
                    )
                    .allows(slot.owner_idx)
            });
            ConstructedRoute::singleton(element, all)
        })
        .collect::<Vec<_>>();
    let mut admission = OwnerAdmission::new(&routes, &slots);
    let classes = CLASSES.with(Cell::get);
    let (mut attempts, mut admitted) = (0, 0);
    for step in 0..200u64 {
        let pick = mix(seed ^ step << 24);
        let live = (0..routes.len())
            .filter(|&route_idx| !routes[route_idx].visits.is_empty())
            .collect::<Vec<_>>();
        if live.len() < 2 {
            break;
        }
        let merged = live[pick as usize % live.len()];
        let removed = live[(pick >> 16) as usize % live.len()];
        let metric_class = (pick >> 32) as usize % classes;
        if merged == removed
            || !routes[merged].can_merge_for_metric_class(metric_class)
            || !routes[removed].can_merge_for_metric_class(metric_class)
            || !slots.iter().any(|slot| slot.metric_class == metric_class)
        {
            continue;
        }
        let candidate = routes[merged]
            .visits
            .iter()
            .chain(&routes[removed].visits)
            .copied()
            .collect::<Vec<_>>();
        let values = route_values(&access, &source_index, &candidate);
        let elements =
            route_elements::<ListClarkeWrightPhase<Plan, usize>>(&source_index, &candidate);
        let owners = feasible_owners_for_scored_elements(
            &access,
            &plan,
            &slots,
            &mut feasibility,
            &values,
            &elements,
            Some(metric_class),
            owner_count,
        );
        if owners.is_empty() {
            continue;
        }
        let proposal = MergeProposal {
            merged,
            removed,
            metric_class,
            feasible_for_all_metric_class_owners: owners.len()
                == admission.class_owner_count(metric_class),
            feasible_owners: &owners,
        };
        let expected = recomputed_admission(
            &access,
            &plan,
            &source_index,
            &routes,
            &proposal,
            &candidate,
            &slots,
            owner_count,
        );
        let query = OwnerQuery {
            access: &access,
            solution: &plan,
            source_index: &source_index,
            owner_slots: &slots,
            entity_count: owner_count,
        };
        let actual = admission.admits(&query, &routes, &mut feasibility, &proposal);
        assert_eq!(actual, expected, "seed {seed}, step {step}");
        attempts += 1;
        if actual {
            admitted += 1;
            admission.commit(&routes, &proposal);
            let all_class = proposal.feasible_for_all_metric_class_owners;
            routes[merged].visits = candidate;
            routes[merged].scored_metric_class = Some(metric_class);
            routes[merged].feasible_for_all_owners = false;
            routes[merged].feasible_for_all_metric_class_owners = all_class;
            routes[removed].visits.clear();
            routes[removed].scored_metric_class = None;
            routes[removed].feasible_for_all_owners = false;
            routes[removed].feasible_for_all_metric_class_owners = false;
        }
    }

    let query = OwnerQuery {
        access: &access,
        solution: &plan,
        source_index: &source_index,
        owner_slots: &slots,
        entity_count: owner_count,
    };
    let kept = admission.into_owner_sets(&query, &routes, &mut feasibility);
    let mut fresh = SavingsFeasibility::new(&access, &slots);
    let recomputed = routes
        .iter()
        .filter(|route| !route.visits.is_empty())
        .map(|route| {
            feasible_owners_for_scored_elements(
                &access,
                &plan,
                &slots,
                &mut fresh,
                &route_values(&access, &source_index, &route.visits),
                &route_elements::<ListClarkeWrightPhase<Plan, usize>>(&source_index, &route.visits),
                route.scored_metric_class,
                owner_count,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(kept, recomputed, "seed {seed}: final owner sets");
    (attempts, admitted)
}

#[test]
fn incremental_admission_matches_whole_set_recomputation() {
    let (mut attempts, mut admitted) = (0, 0);
    for seed in 0..400 {
        for shared_by_class in [false, true] {
            let (case_attempts, case_admitted) = run_case(seed, shared_by_class);
            attempts += case_attempts;
            admitted += case_admitted;
        }
    }
    // The instances must exercise both outcomes, not only one of them.
    assert!(attempts > 2_000, "attempts {attempts}");
    assert!(
        admitted > 200 && admitted < attempts,
        "{admitted}/{attempts}"
    );
}
