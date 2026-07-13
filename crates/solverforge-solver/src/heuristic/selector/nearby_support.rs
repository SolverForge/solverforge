use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub(crate) fn truncate_nearby_candidates<T>(
    candidates: &mut Vec<(T, f64, usize)>,
    max_nearby: usize,
) {
    candidates.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.2.cmp(&right.2))
    });
    candidates.truncate(max_nearby);
}

/// One source candidate ranked by nearby distance, then source order, then
/// its logical index. The final index makes equal-distance ordering explicit
/// even when a host source is not otherwise deterministic.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RankedNearbyCandidate {
    pub candidate: usize,
    pub distance: f64,
    pub order: usize,
}

impl PartialEq for RankedNearbyCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.candidate == other.candidate
            && self.distance.total_cmp(&other.distance) == Ordering::Equal
            && self.order == other.order
    }
}

impl Eq for RankedNearbyCandidate {}

impl PartialOrd for RankedNearbyCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RankedNearbyCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.distance
            .total_cmp(&other.distance)
            .then_with(|| self.order.cmp(&other.order))
            .then_with(|| self.candidate.cmp(&other.candidate))
    }
}

/// Bounded stable nearby selection that retains only the best `limit` source
/// candidates while a host callback is being visited. The heap head is the
/// current worst selected candidate, so this avoids materializing an arbitrary
/// callback result merely to truncate it later.
pub(crate) struct NearbyTopK {
    selected: BinaryHeap<RankedNearbyCandidate>,
    limit: usize,
}

impl NearbyTopK {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            selected: BinaryHeap::new(),
            limit,
        }
    }

    pub(crate) fn push(&mut self, candidate: RankedNearbyCandidate) {
        if self.limit == 0 || !candidate.distance.is_finite() {
            return;
        }
        if self.selected.len() < self.limit {
            self.selected.push(candidate);
            return;
        }
        if self.selected.peek().is_some_and(|worst| candidate < *worst) {
            let _ = self.selected.pop();
            self.selected.push(candidate);
        }
    }

    pub(crate) fn finish(self) -> Vec<usize> {
        let mut selected = self.selected.into_vec();
        selected.sort_unstable();
        selected
            .into_iter()
            .map(|candidate| candidate.candidate)
            .collect()
    }
}
