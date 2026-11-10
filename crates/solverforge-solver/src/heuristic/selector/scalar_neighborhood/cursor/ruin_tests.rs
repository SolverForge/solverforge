//! Ruin batch generation against the full-reset reference algorithm.

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use super::ruin_batches;

/// The reference: resets the whole permutation to the identity before every
/// batch.
fn full_reset_batches(
    entity_count: usize,
    min_ruin_count: usize,
    max_ruin_count: usize,
    moves_per_step: usize,
    rng: &mut SmallRng,
) -> Vec<Vec<usize>> {
    let min = min_ruin_count.min(entity_count);
    let max = max_ruin_count.min(entity_count);
    let mut permutation: Vec<usize> = (0..entity_count).collect();
    (0..moves_per_step)
        .map(|_| {
            if entity_count == 0 {
                return Vec::new();
            }
            for (index, entity) in permutation.iter_mut().enumerate() {
                *entity = index;
            }
            let ruin_count = if min == max {
                min
            } else {
                rng.random_range(min..=max)
            };
            for index in 0..ruin_count {
                let swap_index = rng.random_range(index..entity_count);
                permutation.swap(index, swap_index);
            }
            permutation[..ruin_count].to_vec()
        })
        .collect()
}

#[test]
fn ruin_batches_match_the_full_reset_reference() {
    for entity_count in 0..40 {
        for (min, max) in [(0, 0), (1, 1), (1, 3), (2, 8), (5, 5), (0, 40), (39, 60)] {
            for seed in 0..25u64 {
                let mut fast = SmallRng::seed_from_u64(seed);
                let mut reference = SmallRng::seed_from_u64(seed);
                assert_eq!(
                    ruin_batches(entity_count, min, max, 7, &mut fast),
                    full_reset_batches(entity_count, min, max, 7, &mut reference),
                    "entity_count {entity_count}, ruin {min}..={max}, seed {seed}"
                );
                assert_eq!(
                    fast.random::<u64>(),
                    reference.random::<u64>(),
                    "both consume the same random draws"
                );
            }
        }
    }
}
