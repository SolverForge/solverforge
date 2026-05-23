use std::hash::{DefaultHasher, Hash, Hasher};

pub(super) fn key_hash<K: Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
