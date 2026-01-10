//! Shared utilities for typed constraints.

use std::hash::{Hash, Hasher};

/// Computes a hash of a value for tracking in join indices.
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::shared::compute_hash;
///
/// let hash1 = compute_hash(&42);
/// let hash2 = compute_hash(&42);
/// assert_eq!(hash1, hash2);
///
/// let hash3 = compute_hash(&43);
/// assert_ne!(hash1, hash3);
/// ```
#[inline]
pub fn compute_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
