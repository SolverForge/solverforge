//! Joint groups: elements computed together because each waits on the
//! others' inputs.

use super::CascadePropagation;
use crate::domain::PlanningSolution;

impl CascadePropagation {
    /// Joins every element with its declared peers, transitively, and lays
    /// the groups out as compressed rows keyed by each group's lowest
    /// element. Leaves the group index empty when no element declares a
    /// peer, so a model without groups maps every element to itself.
    pub(super) fn rebuild_groups<S: PlanningSolution>(&mut self, solution: &S, count: usize) {
        self.node.clear();
        self.member_offsets.clear();
        self.members.clear();
        // A union-find forest whose roots are the lowest element of their
        // tree, allocated on the first declared peer.
        let mut parent: Vec<usize> = Vec::new();
        for element in 0..count {
            self.scratch.clear();
            solution.cascading_peers(element, &mut self.scratch);
            for &peer in &self.scratch {
                assert!(
                    peer < count,
                    "cascading element {element} declares peer {peer}, but there are only {count} elements"
                );
                if parent.is_empty() {
                    parent = (0..count).collect();
                }
                let (a, b) = (root(&mut parent, element), root(&mut parent, peer));
                parent[a.max(b)] = a.min(b);
            }
        }
        self.scratch.clear();
        if parent.is_empty() {
            return;
        }
        self.node = (0..count)
            .map(|element| root(&mut parent, element))
            .collect();
        self.member_offsets.resize(count + 1, 0);
        for &node in &self.node {
            self.member_offsets[node + 1] += 1;
        }
        for node in 0..count {
            self.member_offsets[node + 1] += self.member_offsets[node];
        }
        self.members.resize(count, 0);
        let mut fill = self.member_offsets.clone();
        for element in 0..count {
            let node = self.node[element];
            self.members[fill[node]] = element;
            fill[node] += 1;
        }
    }

    /// The node holding `element`: its group's lowest element, or itself.
    /// `GROUPED` is whether any element declares peers.
    #[inline(always)]
    pub(super) fn node_of<const GROUPED: bool>(&self, element: usize) -> usize {
        if !GROUPED {
            element
        } else {
            self.node[element]
        }
    }

    /// Number of elements of `node`.
    #[inline(always)]
    pub(super) fn member_count<const GROUPED: bool>(&self, node: usize) -> usize {
        if !GROUPED {
            1
        } else {
            self.member_offsets[node + 1] - self.member_offsets[node]
        }
    }

    /// The `index`th element of `node`, in ascending element order.
    #[inline(always)]
    pub(super) fn member<const GROUPED: bool>(&self, node: usize, index: usize) -> usize {
        if !GROUPED {
            node
        } else {
            self.members[self.member_offsets[node] + index]
        }
    }
}

fn root(parent: &mut [usize], mut element: usize) -> usize {
    while parent[element] != element {
        parent[element] = parent[parent[element]];
        element = parent[element];
    }
    element
}
