use std::hash::{Hash, Hasher};

use smallvec::SmallVec;
use solverforge_core::fast_hash::{FastHasher, FastMap};

type EditKey = (usize, usize, usize, Option<usize>);

#[derive(Debug)]
pub(crate) struct AssignmentMoveKey {
    edits: SmallVec<[EditKey; 8]>,
    fingerprint: u64,
}

impl AssignmentMoveKey {
    pub(super) fn new(edits: impl IntoIterator<Item = EditKey>) -> Self {
        let mut key = Self {
            edits: SmallVec::new(),
            fingerprint: 0,
        };
        for edit in edits {
            let mut hasher = FastHasher::default();
            edit.hash(&mut hasher);
            key.fingerprint = key.fingerprint.wrapping_add(hasher.finish());
            key.edits.push(edit);
        }
        key
    }
}

impl Hash for AssignmentMoveKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.edits.len().hash(state);
        self.fingerprint.hash(state);
    }
}

impl PartialEq for AssignmentMoveKey {
    fn eq(&self, other: &Self) -> bool {
        if self.fingerprint != other.fingerprint || self.edits.len() != other.edits.len() {
            return false;
        }
        if self.edits == other.edits {
            return true;
        }
        // Identity is an edit multiset, not a hash alone. Preserve duplicate
        // multiplicity even for manually constructed candidate recipes.
        if self.edits.len() <= 8 {
            return self.edits.iter().all(|edit| {
                self.edits.iter().filter(|value| *value == edit).count()
                    == other.edits.iter().filter(|value| *value == edit).count()
            });
        }
        let mut counts = FastMap::default();
        for edit in &self.edits {
            *counts.entry(*edit).or_insert(0_usize) += 1;
        }
        for edit in &other.edits {
            let Some(count) = counts.get_mut(edit) else {
                return false;
            };
            if *count == 0 {
                return false;
            }
            *count -= 1;
        }
        true
    }
}

impl Eq for AssignmentMoveKey {}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::fast_hash::FastSet;

    #[test]
    fn identity_matches_sorted_edit_multisets_including_long_permutations() {
        for len in [0, 1, 2, 8, 9, 31, 128] {
            let edits: Vec<_> = (0..len)
                .map(|row| {
                    (
                        row % 2,
                        row % 11,
                        row % 3,
                        (row % 4 != 0).then_some(row % 7),
                    )
                })
                .collect();
            for offset in 0..len.max(1) {
                let mut reordered = edits.clone();
                if len > 0 {
                    reordered.rotate_left(offset);
                    if offset % 2 == 0 {
                        reordered.reverse();
                    }
                }
                let key = AssignmentMoveKey::new(edits.iter().copied());
                let other = AssignmentMoveKey::new(reordered);
                assert_eq!(key, other);
                assert_eq!(key.edits.spilled(), len > 8);
                let mut seen = FastSet::default();
                assert!(seen.insert(key));
                assert!(!seen.insert(other));
            }
        }
    }

    #[test]
    fn identity_matches_sorted_reference_for_changed_and_reordered_recipes() {
        for seed in 0..512_usize {
            let len = seed % 65;
            let edits: Vec<_> = (0..len)
                .map(|row| (row % 3, row % 7, row % 2, Some((row + seed) % 5)))
                .collect();
            let mut changed = edits.clone();
            if len > 0 {
                changed.rotate_left(seed % len);
                changed.reverse();
                if seed % 3 == 0 {
                    changed[seed % len].3 = None;
                }
            }
            let actual = AssignmentMoveKey::new(edits.iter().copied());
            let other = AssignmentMoveKey::new(changed.iter().copied());
            let mut expected = edits;
            expected.sort_unstable();
            changed.sort_unstable();
            assert_eq!(actual == other, expected == changed, "recipe {seed}");
        }
    }

    #[test]
    fn hash_collisions_never_collapse_distinct_edits_or_multiplicities() {
        for len in [2, 8, 9, 32] {
            let edits = vec![(0, 1, 0, Some(2)); len];
            for changed in [
                (1, 1, 0, Some(2)),
                (0, 3, 0, Some(2)),
                (0, 1, 1, Some(2)),
                (0, 1, 0, None),
            ] {
                let first = AssignmentMoveKey::new(edits.iter().copied());
                let mut second_edits = edits.clone();
                second_edits[0] = changed;
                let mut second = AssignmentMoveKey::new(second_edits);
                second.fingerprint = first.fingerprint;
                let mut seen = FastSet::default();
                assert!(seen.insert(first));
                assert!(seen.insert(second));
            }
        }
    }
}
