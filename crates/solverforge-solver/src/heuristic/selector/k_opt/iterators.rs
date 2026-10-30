//! Iterators for k-opt cut combinations.

use crate::heuristic::r#move::CutPoint;

/// Counts the number of valid k-cut combinations for a route of length len.
pub(crate) fn count_cut_combinations(k: usize, len: usize, min_seg: usize) -> usize {
    // This is equivalent to C(n - (k+1)*min_seg + k, k)
    // where we're choosing k positions from the "free" slots
    let min_len = (k + 1) * min_seg;
    if len < min_len {
        return 0;
    }

    let free_slots = len - min_len + k;
    binomial(free_slots, k)
}

/// Returns one cut combination in the same lexicographic order as
/// [`CutCombinationIterator`] without advancing through the preceding
/// combinations. This keeps shuffled and random K-opt traversal lazy.
pub(crate) fn cut_combination_at(
    k: usize,
    len: usize,
    min_seg: usize,
    entity_idx: usize,
    mut rank: usize,
) -> Option<Vec<CutPoint>> {
    if k == 0 || min_seg == 0 {
        return None;
    }
    let choice_count = len.checked_sub((k + 1).checked_mul(min_seg)?)? + k;
    if rank >= binomial(choice_count, k) {
        return None;
    }

    let mut choices = Vec::with_capacity(k);
    let mut start = 0;
    for position in 0..k {
        let remaining = k - position - 1;
        let maximum = choice_count - (k - position);
        let mut selected = None;
        for candidate in start..=maximum {
            let suffix_count = binomial(choice_count - candidate - 1, remaining);
            if rank < suffix_count {
                selected = Some(candidate);
                break;
            }
            rank -= suffix_count;
        }
        let selected = selected?;
        choices.push(selected);
        start = selected + 1;
    }

    Some(
        choices
            .into_iter()
            .enumerate()
            .map(|(position, choice)| {
                CutPoint::new(entity_idx, choice + min_seg + position * (min_seg - 1))
            })
            .collect(),
    )
}

/// Compute binomial coefficient C(n, k).
pub(crate) fn binomial(n: usize, k: usize) -> usize {
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
