//! Reconnection patterns for k-opt moves.
//!
//! K-opt removes k edges from a tour, creating k+1 segments. These segments
//! can be reconnected in different ways, with optional reversal of segments.
//!
//! # Zero-Erasure Design
//!
//! All data structures use plain arrays for compile-time construction:
//! - `[u8; 6]` for segment order (max 6 segments for 5-opt)
//! - `u8` bitmask for reversal flags
//! - `const fn` constructors
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::r#move::k_opt_reconnection::{
//!     KOptReconnection, THREE_OPT_RECONNECTIONS,
//! };
//!
//! // 3-opt has 7 non-trivial reconnection patterns
//! assert_eq!(THREE_OPT_RECONNECTIONS.len(), 7);
//!
//! // Each pattern specifies segment order and reversals
//! let pattern = &THREE_OPT_RECONNECTIONS[0];
//! assert_eq!(pattern.k(), 3);
//! assert_eq!(pattern.segment_count(), 4);
//! ```

/// A reconnection pattern for k-opt.
///
/// Specifies how to reconnect segments after removing k edges.
/// Uses plain arrays for zero-erasure compile-time construction.
///
/// # Fields
///
/// - `segment_order`: Permutation of segment indices `[0..segment_count)`.
///   Index 0 is always 0 (first segment stays first).
///   Unused slots (beyond `len`) are 0.
///
/// - `reverse_mask`: Bitmask indicating which segments to reverse.
///   Bit i is set if segment i should be reversed.
///
/// - `len`: Number of valid segments (k+1 for k-opt).
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::k_opt_reconnection::KOptReconnection;
///
/// // 3-opt reconnection: swap B and C, reverse both
/// // Original: A -> B -> C -> D
/// // Result:   A -> C' -> B' -> D
/// let pattern = KOptReconnection::new([0, 2, 1, 3, 0, 0], 0b0110, 4);
///
/// assert_eq!(pattern.k(), 3);
/// assert!(pattern.should_reverse(1));  // B reversed
/// assert!(pattern.should_reverse(2));  // C reversed
/// assert!(!pattern.should_reverse(0)); // A not reversed
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KOptReconnection {
    /// Segment order as fixed array.
    segment_order: [u8; 6],
    /// Bitmask of segments to reverse.
    reverse_mask: u8,
    /// Number of valid segments.
    len: u8,
}

impl KOptReconnection {
    /// Creates a new reconnection pattern (const for compile-time use).
    ///
    /// # Arguments
    ///
    /// * `segment_order` - Permutation of segment indices
    /// * `reverse_mask` - Bitmask of segments to reverse
    /// * `len` - Number of valid segments (k+1)
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::heuristic::r#move::k_opt_reconnection::KOptReconnection;
    ///
    /// const PATTERN: KOptReconnection = KOptReconnection::new(
    ///     [0, 2, 1, 3, 0, 0],
    ///     0b0000,
    ///     4,
    /// );
    /// assert_eq!(PATTERN.segment_at(1), 2);
    /// ```
    #[inline]
    pub const fn new(segment_order: [u8; 6], reverse_mask: u8, len: u8) -> Self {
        Self {
            segment_order,
            reverse_mask,
            len,
        }
    }

    /// Returns the k value (number of edges removed).
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::heuristic::r#move::k_opt_reconnection::THREE_OPT_RECONNECTIONS;
    ///
    /// assert_eq!(THREE_OPT_RECONNECTIONS[0].k(), 3);
    /// ```
    #[inline]
    pub const fn k(&self) -> usize {
        (self.len - 1) as usize
    }

    /// Returns the number of segments (k+1).
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::heuristic::r#move::k_opt_reconnection::THREE_OPT_RECONNECTIONS;
    ///
    /// assert_eq!(THREE_OPT_RECONNECTIONS[0].segment_count(), 4);
    /// ```
    #[inline]
    pub const fn segment_count(&self) -> usize {
        self.len as usize
    }

    /// Returns the segment index at position `pos` in the reconnected order.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::heuristic::r#move::k_opt_reconnection::KOptReconnection;
    ///
    /// // Pattern swaps segments 1 and 2: [A, C, B, D]
    /// let pattern = KOptReconnection::new([0, 2, 1, 3, 0, 0], 0, 4);
    /// assert_eq!(pattern.segment_at(0), 0); // A
    /// assert_eq!(pattern.segment_at(1), 2); // C (was at position 2)
    /// assert_eq!(pattern.segment_at(2), 1); // B (was at position 1)
    /// assert_eq!(pattern.segment_at(3), 3); // D
    /// ```
    #[inline]
    pub const fn segment_at(&self, pos: usize) -> usize {
        self.segment_order[pos] as usize
    }

    /// Returns true if segment at index `idx` should be reversed.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::heuristic::r#move::k_opt_reconnection::KOptReconnection;
    ///
    /// let pattern = KOptReconnection::new([0, 1, 2, 3, 0, 0], 0b0110, 4);
    /// assert!(!pattern.should_reverse(0));
    /// assert!(pattern.should_reverse(1));
    /// assert!(pattern.should_reverse(2));
    /// assert!(!pattern.should_reverse(3));
    /// ```
    #[inline]
    pub const fn should_reverse(&self, idx: usize) -> bool {
        (self.reverse_mask >> idx) & 1 == 1
    }

    /// Returns the segment order as a slice.
    #[inline]
    pub fn segment_order(&self) -> &[u8] {
        &self.segment_order[..self.len as usize]
    }

    /// Returns true if this is the identity reconnection (no change).
    const fn is_identity(&self) -> bool {
        if self.reverse_mask != 0 {
            return false;
        }
        let mut i = 0;
        while i < self.len as usize {
            if self.segment_order[i] != i as u8 {
                return false;
            }
            i += 1;
        }
        true
    }
}

/// Pre-computed 3-opt reconnection patterns (7 non-trivial patterns).
///
/// Given segments `[A, B, C, D]` where edges are cut between each:
///
/// | # | Order      | Reverse | Description              |
/// |---|------------|---------|--------------------------|
/// | 0 | A-B'-C-D   | B       | Reverse B only           |
/// | 1 | A-B-C'-D   | C       | Reverse C only           |
/// | 2 | A-B'-C'-D  | B,C     | Reverse both (no swap)   |
/// | 3 | A-C-B-D    | -       | Swap B and C             |
/// | 4 | A-C'-B-D   | C       | Swap, reverse C          |
/// | 5 | A-C-B'-D   | B       | Swap, reverse B          |
/// | 6 | A-C'-B'-D  | B,C     | Swap, reverse both       |
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::k_opt_reconnection::THREE_OPT_RECONNECTIONS;
///
/// assert_eq!(THREE_OPT_RECONNECTIONS.len(), 7);
///
/// // Pattern 3 swaps B and C without reversal
/// let swap_pattern = &THREE_OPT_RECONNECTIONS[3];
/// assert_eq!(swap_pattern.segment_at(1), 2); // C moved to position 1
/// assert_eq!(swap_pattern.segment_at(2), 1); // B moved to position 2
/// assert!(!swap_pattern.should_reverse(1));
/// assert!(!swap_pattern.should_reverse(2));
/// ```
pub static THREE_OPT_RECONNECTIONS: &[KOptReconnection] = &[
    // No swap, with reversals
    KOptReconnection::new([0, 1, 2, 3, 0, 0], 0b0010, 4), // Reverse B
    KOptReconnection::new([0, 1, 2, 3, 0, 0], 0b0100, 4), // Reverse C
    KOptReconnection::new([0, 1, 2, 3, 0, 0], 0b0110, 4), // Reverse B and C
    // Swap B and C
    KOptReconnection::new([0, 2, 1, 3, 0, 0], 0b0000, 4), // No reversal
    KOptReconnection::new([0, 2, 1, 3, 0, 0], 0b0010, 4), // Reverse B (now at pos 2)
    KOptReconnection::new([0, 2, 1, 3, 0, 0], 0b0100, 4), // Reverse C (now at pos 1)
    KOptReconnection::new([0, 2, 1, 3, 0, 0], 0b0110, 4), // Reverse both
];

/// Generates all non-trivial reconnection patterns for k-opt.
///
/// Enumerates all valid reconnections by:
/// 1. Generating all permutations of middle segments
/// 2. For each permutation, generating all reversal combinations
/// 3. Filtering out the identity reconnection
///
/// # Arguments
///
/// * `k` - Number of edges to remove (2-5)
///
/// # Panics
///
/// Panics if k < 2 or k > 5.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::k_opt_reconnection::enumerate_reconnections;
///
/// let patterns_2opt = enumerate_reconnections(2);
/// assert_eq!(patterns_2opt.len(), 1); // Only reverse middle
///
/// let patterns_3opt = enumerate_reconnections(3);
/// assert_eq!(patterns_3opt.len(), 7);
///
/// let patterns_4opt = enumerate_reconnections(4);
/// assert_eq!(patterns_4opt.len(), 47); // 3! * 2^3 - 1
/// ```
pub fn enumerate_reconnections(k: usize) -> Vec<KOptReconnection> {
    assert!((2..=5).contains(&k), "k must be between 2 and 5");

    let num_segments = k + 1;
    let num_middle = k - 1;

    let mut result = Vec::new();

    // Generate all permutations of middle segment indices [1..k]
    let middle_indices: Vec<u8> = (1..k as u8).collect();
    let permutations = generate_permutations(&middle_indices);

    for perm in permutations {
        // Build full segment order: [0, perm..., k]
        let mut segment_order = [0u8; 6];
        segment_order[0] = 0;
        for (i, &idx) in perm.iter().enumerate() {
            segment_order[i + 1] = idx;
        }
        segment_order[num_segments - 1] = (num_segments - 1) as u8;

        // Generate all reversal combinations for middle segments
        for mask in 0..(1u8 << num_middle) {
            // Shift mask left by 1 since segment 0 is never reversed
            let reverse_mask = mask << 1;

            let reconnection =
                KOptReconnection::new(segment_order, reverse_mask, num_segments as u8);

            if !reconnection.is_identity() {
                result.push(reconnection);
            }
        }
    }

    result
}

/// Generates all permutations of a slice.
fn generate_permutations(items: &[u8]) -> Vec<Vec<u8>> {
    if items.is_empty() {
        return vec![vec![]];
    }
    if items.len() == 1 {
        return vec![vec![items[0]]];
    }

    let mut result = Vec::new();
    for i in 0..items.len() {
        let mut rest: Vec<u8> = items.to_vec();
        let item = rest.remove(i);
        for mut perm in generate_permutations(&rest) {
            perm.insert(0, item);
            result.push(perm);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Public API tests (enumerate_*_count, enumerate_matches_static_3opt)
    // are in heuristic/move/tests/k_opt.rs
    // These tests verify internal invariants using private methods.

    #[test]
    fn no_identity_in_patterns() {
        for k in 2..=5 {
            let patterns = enumerate_reconnections(k);
            for p in &patterns {
                assert!(!p.is_identity(), "Found identity in {}-opt patterns", k);
            }
        }
    }

    #[test]
    fn static_patterns_not_identity() {
        for p in THREE_OPT_RECONNECTIONS {
            assert!(!p.is_identity());
        }
    }
}
