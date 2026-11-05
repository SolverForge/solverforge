//! Route-local k-opt polishing for k from 3 to 5.
//!
//! A route `r[0..n]` sits between its depot at both ends. k cuts at edge
//! positions `e_1 < … < e_k` in `0..=n` (edge `e` joins `r[e - 1]` and `r[e]`,
//! the depot standing in at `-1` and `n`) split it into a fixed head
//! `r[0..e_1]`, `k - 1` non-empty middle segments, and a fixed tail
//! `r[e_k..n]`. Every reconnection pattern of `enumerate_reconnections(k)`
//! reorders and reverses the middle segments. A candidate's distance delta
//! is exact in O(k), also for asymmetric distances: the boundary edges are
//! read directly and each middle segment's inner cost comes from forward and
//! backward prefix sums.
//!
//! Cut sets are scanned in lexicographic order and patterns in their
//! enumeration order. The first improving, feasible reconnection is applied,
//! and the scan restarts until no improving reconnection is left.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::kernel::ListKOptAccess;
use crate::heuristic::r#move::k_opt_reconnection::{enumerate_reconnections, KOptReconnection};
use crate::phase::construction::{record_construction_candidate, PendingConstructionMoveTelemetry};
use crate::scope::{PhaseScope, ProgressCallback, StepControlPolicy};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTraceSource,
};

/// Polishes `route` with k-opt and returns `(changed, interrupted)`.
#[allow(clippy::too_many_arguments)]
pub(super) fn polish_route<S, A, D, BestCb>(
    access: &A,
    k: usize,
    control_policy: StepControlPolicy,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    entity_idx: usize,
    depot: usize,
    route: &mut Vec<usize>,
    pending_move_telemetry: &mut PendingConstructionMoveTelemetry,
) -> (bool, bool)
where
    S: PlanningSolution,
    A: ListKOptAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let patterns = enumerate_reconnections(k);
    let descriptor_index = access.descriptor_index();
    let mut changed = false;
    let mut candidate_index = 0usize;
    'scan: loop {
        let n = route.len();
        let costs = {
            let solution = phase_scope.score_director().working_solution();
            RouteCosts::new(depot, route, |from, to| {
                access.route_distance(solution, entity_idx, from, to)
            })
        };
        let mut cuts: Vec<usize> = (0..k).collect();
        if k > n + 1 {
            return (changed, false);
        }
        loop {
            for (pattern_index, pattern) in patterns.iter().enumerate() {
                if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
                    return (changed, true);
                }
                let components = std::iter::once(entity_idx as u64)
                    .chain(cuts.iter().map(|&cut| cut as u64))
                    .chain(std::iter::once(pattern_index as u64));
                let trace_token = phase_scope.record_candidate_operation(
                    CandidateTraceSource::ListKOptReconnection,
                    None,
                    candidate_index,
                    Some(CandidateTraceConstructionTarget {
                        descriptor_index,
                        entity_index: entity_idx,
                    }),
                    descriptor_index,
                    "list_k_opt_reconnection",
                    components,
                );
                candidate_index += 1;
                let delta = costs.delta(&cuts, pattern);
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::Evaluated,
                    );
                }
                record_construction_candidate(
                    phase_scope,
                    std::time::Duration::ZERO,
                    std::time::Duration::ZERO,
                );
                if delta < 0 {
                    let candidate = reconnect(route, &cuts, pattern);
                    if access.route_feasible(
                        phase_scope.score_director().working_solution(),
                        entity_idx,
                        &candidate,
                    ) {
                        *route = candidate;
                        pending_move_telemetry.record_accepted(phase_scope, trace_token);
                        changed = true;
                        continue 'scan;
                    }
                }
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
            }
            if !next_cut_set(&mut cuts, n) {
                return (changed, false);
            }
        }
    }
}

/// Advances `cuts` to the next strictly increasing k-subset of `0..=n` in
/// lexicographic order; `false` after the last one.
fn next_cut_set(cuts: &mut [usize], n: usize) -> bool {
    let k = cuts.len();
    for position in (0..k).rev() {
        let ceiling = n + 1 - (k - position);
        if cuts[position] < ceiling {
            cuts[position] += 1;
            for next in position + 1..k {
                cuts[next] = cuts[next - 1] + 1;
            }
            return true;
        }
    }
    false
}

/// The route after applying `pattern` at `cuts`.
fn reconnect(route: &[usize], cuts: &[usize], pattern: &KOptReconnection) -> Vec<usize> {
    let mut bounds = Vec::with_capacity(cuts.len() + 2);
    bounds.push(0);
    bounds.extend_from_slice(cuts);
    bounds.push(route.len());
    let mut result = Vec::with_capacity(route.len());
    for position in 0..pattern.segment_count() {
        let segment = pattern.segment_at(position);
        let slice = &route[bounds[segment]..bounds[segment + 1]];
        if pattern.should_reverse(segment) {
            result.extend(slice.iter().rev());
        } else {
            result.extend_from_slice(slice);
        }
    }
    result
}

/// Distances of one route: the endpoints, and forward and backward prefix
/// sums of its inner edges.
struct RouteCosts {
    depot: usize,
    route: Vec<usize>,
    forward: Vec<i128>,
    backward: Vec<i128>,
    distance: Vec<Vec<i128>>,
}

impl RouteCosts {
    /// `distance(from, to)` is the distance between two elements, the depot
    /// included.
    fn new(depot: usize, route: &[usize], distance: impl Fn(usize, usize) -> i64) -> Self {
        // Nodes: the route elements, then the depot at index `n`.
        let n = route.len();
        let node = |index: usize| if index == n { depot } else { route[index] };
        let distance = (0..=n)
            .map(|from| {
                (0..=n)
                    .map(|to| i128::from(distance(node(from), node(to))))
                    .collect()
            })
            .collect::<Vec<Vec<i128>>>();
        let mut forward = vec![0i128; n.max(1)];
        let mut backward = vec![0i128; n.max(1)];
        for t in 1..n {
            forward[t] = forward[t - 1] + distance[t - 1][t];
            backward[t] = backward[t - 1] + distance[t][t - 1];
        }
        Self {
            depot: n,
            route: route.to_vec(),
            forward,
            backward,
            distance,
        }
    }

    /// Inner cost of `route[start..end]` (non-empty), forward or reversed.
    fn inner(&self, start: usize, end: usize, reversed: bool) -> i128 {
        let prefix = if reversed {
            &self.backward
        } else {
            &self.forward
        };
        prefix[end - 1] - prefix[start]
    }

    /// Distance change of `pattern` at `cuts`; negative improves.
    fn delta(&self, cuts: &[usize], pattern: &KOptReconnection) -> i128 {
        let k = cuts.len();
        let n = self.route.len();
        debug_assert_eq!(self.depot, n);
        // The head ends at `cuts[0] - 1` (or the depot), the tail starts at
        // `cuts[k - 1]` (or the depot); both stay in place.
        let head_end = if cuts[0] == 0 {
            self.depot
        } else {
            cuts[0] - 1
        };
        let tail_start = if cuts[k - 1] == n {
            self.depot
        } else {
            cuts[k - 1]
        };
        let middle = |segment: usize| (cuts[segment - 1], cuts[segment]);
        let cost = |order: &mut dyn Iterator<Item = (usize, bool)>| {
            let mut total = 0i128;
            let mut previous = head_end;
            for (segment, reversed) in order {
                let (start, end) = middle(segment);
                let (first, last) = if reversed {
                    (end - 1, start)
                } else {
                    (start, end - 1)
                };
                total += self.distance[previous][first] + self.inner(start, end, reversed);
                previous = last;
            }
            total + self.distance[previous][tail_start]
        };
        let current = cost(&mut (1..k).map(|segment| (segment, false)));
        let proposed = cost(&mut (1..k).map(|position| {
            let segment = pattern.segment_at(position);
            (segment, pattern.should_reverse(segment))
        }));
        proposed - current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_sets_cover_every_increasing_subset_once() {
        let mut cuts = vec![0, 1, 2];
        let mut seen = vec![cuts.clone()];
        while next_cut_set(&mut cuts, 4) {
            seen.push(cuts.clone());
        }
        // C(5, 3) subsets of 0..=4.
        assert_eq!(seen.len(), 10);
        assert!(seen.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn reconnect_reorders_and_reverses_middle_segments() {
        // A=[0] B=[1,2] C=[3] D=[4]; A-C'-B'-D.
        let pattern = KOptReconnection::new([0, 2, 1, 3, 0, 0], 0b0110, 4);
        assert_eq!(
            reconnect(&[0, 1, 2, 3, 4], &[1, 3, 4], &pattern),
            [0, 3, 2, 1, 4]
        );
    }

    /// Deterministic asymmetric distances: going "down" costs extra.
    fn asymmetric(from: usize, to: usize) -> i64 {
        let base = ((from * 7 + to * 13) % 11) as i64;
        if from > to {
            base + 5
        } else {
            base
        }
    }

    fn route_cost(depot: usize, route: &[usize]) -> i128 {
        let mut previous = depot;
        let mut total = 0i128;
        for &element in route {
            total += i128::from(asymmetric(previous, element));
            previous = element;
        }
        total + i128::from(asymmetric(previous, depot))
    }

    #[test]
    fn deltas_are_exact_for_every_cut_set_and_pattern() {
        let depot = 0;
        let route = [4, 1, 6, 2, 7, 3, 5];
        let costs = RouteCosts::new(depot, &route, asymmetric);
        let base = route_cost(depot, &route);
        let mut checked = 0;
        for k in 3..=5 {
            let patterns = enumerate_reconnections(k);
            let mut cuts: Vec<usize> = (0..k).collect();
            loop {
                for pattern in &patterns {
                    let candidate = reconnect(&route, &cuts, pattern);
                    let mut sorted = candidate.clone();
                    sorted.sort_unstable();
                    assert_eq!(sorted, [1, 2, 3, 4, 5, 6, 7], "{cuts:?}");
                    assert_eq!(
                        costs.delta(&cuts, pattern),
                        route_cost(depot, &candidate) - base,
                        "k {k}, cuts {cuts:?}, candidate {candidate:?}"
                    );
                    checked += 1;
                }
                if !next_cut_set(&mut cuts, route.len()) {
                    break;
                }
            }
        }
        assert!(checked > 0);
    }
}
