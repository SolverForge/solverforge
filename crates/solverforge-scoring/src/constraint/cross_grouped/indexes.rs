use std::hash::{DefaultHasher, Hash, Hasher};

/* Hash for a group key. Match/join indexing now belongs to the common join
operator, so this is the only index helper this family still needs. */
pub(super) fn key_hash<K: Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
