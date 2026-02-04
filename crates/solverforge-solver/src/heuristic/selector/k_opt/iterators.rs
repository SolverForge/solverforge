//! Iterators for k-opt cut combinations.

use crate::heuristic::r#move::CutPoint;

/// Iterator over all valid k-cut combinations for a route of given length.
pub struct CutCombinationIterator {
    k: usize,
    len: usize,
    min_seg: usize,
    entity_idx: usize,
    /// Current cut positions.
    positions: Vec<usize>,
    /// Whether we've exhausted all combinations.
    done: bool,
}

impl CutCombinationIterator {
    pub fn new(k: usize, len: usize, min_seg: usize, entity_idx: usize) -> Self {
        // Minimum length required: k cuts need k+1 segments of min_seg each
        let min_len = (k + 1) * min_seg;

        if len < min_len {
            return Self {
                k,
                len,
                min_seg,
                entity_idx,
                positions: vec![],
                done: true,
            };
        }

        // Initialize with first valid combination
        // Cuts must be at positions that leave min_seg elements between them
        let mut positions = Vec::with_capacity(k);
        for i in 0..k {
            positions.push(min_seg * (i + 1));
        }

        Self {
            k,
            len,
            min_seg,
            entity_idx,
            positions,
            done: false,
        }
    }

    fn advance(&mut self) -> bool {
        if self.done || self.positions.is_empty() {
            return false;
        }

        // Find the rightmost position that can be incremented
        let k = self.k;
        let len = self.len;
        let min_seg = self.min_seg;

        for i in (0..k).rev() {
            // Maximum position for cut i:
            // Need to leave room for (k - i - 1) more cuts after this one,
            // each separated by min_seg, plus min_seg at the end
            let max_pos = len - min_seg * (k - i);

            if self.positions[i] < max_pos {
                self.positions[i] += 1;
                // Reset all positions after i
                for j in (i + 1)..k {
                    self.positions[j] = self.positions[j - 1] + min_seg;
                }
                return true;
            }
        }

        self.done = true;
        false
    }
}

impl Iterator for CutCombinationIterator {
    type Item = Vec<CutPoint>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let cuts: Vec<CutPoint> = self
            .positions
            .iter()
            .map(|&pos| CutPoint::new(self.entity_idx, pos))
            .collect();

        self.advance();

        Some(cuts)
    }
}

/// Counts the number of valid k-cut combinations for a route of length len.
pub fn count_cut_combinations(k: usize, len: usize, min_seg: usize) -> usize {
    // This is equivalent to C(n - (k+1)*min_seg + k, k)
    // where we're choosing k positions from the "free" slots
    let min_len = (k + 1) * min_seg;
    if len < min_len {
        return 0;
    }

    let free_slots = len - min_len + k;
    binomial(free_slots, k)
}

/// Compute binomial coefficient C(n, k).
pub fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }

    let k = k.min(n - k); // Use symmetry
    let mut result = 1;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}
