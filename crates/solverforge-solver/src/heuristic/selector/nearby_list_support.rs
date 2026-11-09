//! Bounded nearest-destination selection shared by the nearby list cursors.

use std::cmp::Ordering;

/// One nearby destination: `(entity, position, distance)`.
pub(crate) type NearbyCandidate = (usize, usize, f64);

/// The `limit` nearest destinations offered so far, in ascending distance.
///
/// Destinations at equal distance keep the order they were offered in, so the
/// retained set is exactly what a stable sort by distance followed by a
/// truncation to `limit` would keep. The buffer never grows past `limit` and
/// keeps its capacity across [`Self::clear`], so a cursor reuses one buffer for
/// every source position instead of collecting every reachable destination.
#[derive(Debug)]
pub(crate) struct BoundedNearbyCandidates {
    retained: Vec<NearbyCandidate>,
    limit: usize,
}

impl BoundedNearbyCandidates {
    /// Creates an empty buffer that retains at most `limit` destinations.
    ///
    /// Capacity grows on demand to the number of destinations actually
    /// retained, so an unbounded `limit` reserves nothing up front.
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            retained: Vec::new(),
            limit,
        }
    }

    /// Drops every retained destination while keeping the capacity.
    pub(crate) fn clear(&mut self) {
        self.retained.clear();
    }

    /// Offers one destination, keeping it only if it ranks among the nearest
    /// `limit` destinations offered since the last [`Self::clear`].
    #[inline]
    pub(crate) fn offer(&mut self, candidate: NearbyCandidate) {
        let insertion = self.retained.partition_point(|existing| {
            existing
                .2
                .partial_cmp(&candidate.2)
                .unwrap_or(Ordering::Equal)
                != Ordering::Greater
        });
        if insertion >= self.limit {
            return;
        }
        if self.retained.len() == self.limit {
            self.retained.pop();
        }
        self.retained.insert(insertion, candidate);
    }

    /// Number of retained destinations.
    pub(crate) fn len(&self) -> usize {
        self.retained.len()
    }

    /// Whether no destination is retained.
    pub(crate) fn is_empty(&self) -> bool {
        self.retained.is_empty()
    }

    /// The `(entity, position)` of the retained destination at `index`.
    #[inline]
    pub(crate) fn destination(&self, index: usize) -> (usize, usize) {
        let (entity, position, _) = self.retained[index];
        (entity, position)
    }
}

#[cfg(test)]
#[path = "nearby_list_support_tests.rs"]
mod tests;
