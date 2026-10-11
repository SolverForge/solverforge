/* Key-indexed membership state for count-backed existence relations.

An existence event touches O(children) right-side keys: each key's count
must move, and every left row carrying one of those keys must land on the
key's current membership. One compact touched list per event drives the
membership pass, so a key removed and re-added inside one event simply ends
on its final count — no born/died sets, no direction tracking.

The dense shape stores counts and buckets indexed directly by `usize` keys
(planning ids), where hashing costs more than the index. Dispatch happens
once at construction; the event path calls one enum method per key and
nothing else branches.
*/

use super::{FastMap, FastSet, RowHandle};

pub enum KeyState<K> {
    Dense(DenseKeys),
    Hashed {
        counts: FastMap<K, usize>,
        left_buckets: FastMap<K, Vec<RowHandle>>,
        /// Keys whose count changed in the current event, first-touch order.
        touched: Vec<K>,
        /// Membership marker for `touched`, so duplicates never enter the list.
        marked: FastSet<K>,
    },
}

impl<K: Eq + std::hash::Hash + Clone + 'static> KeyState<K> {
    /// Picks the dense shape exactly when the key is `usize`, the planning-id
    /// case where the table beats the hash.
    pub fn new() -> Self {
        if std::any::TypeId::of::<K>() == std::any::TypeId::of::<usize>() {
            // Safety of the cast pair below: every K-typed value stored in
            // the dense shape is repr-transparent here because this arm only
            // constructs when K IS usize. Methods uphold it by converting at
            // the boundary.
            Self::Dense(DenseKeys::new())
        } else {
            Self::Hashed {
                counts: FastMap::default(),
                left_buckets: FastMap::default(),
                touched: Vec::new(),
                marked: FastSet::default(),
            }
        }
    }

    /// Bump one key's count, marking it touched. Returns the fresh count.
    pub fn add(&mut self, key: &K, n: usize) -> usize {
        match self {
            Self::Dense(state) => {
                let raw = dense_raw(key);
                state.add(raw, n)
            }
            Self::Hashed {
                counts,
                touched,
                marked,
                ..
            } => {
                if !marked.contains(key) {
                    marked.insert(key.clone());
                    touched.push(key.clone());
                }
                let entry = counts.entry(key.clone()).or_insert(0);
                *entry += n;
                *entry
            }
        }
    }

    /// Drop one key's count by `n` (count must be at least `n`).
    pub fn take(&mut self, key: &K, n: usize) {
        match self {
            Self::Dense(state) => state.take(dense_raw(key), n),
            Self::Hashed {
                counts,
                touched,
                marked,
                ..
            } => {
                if !marked.contains(key) {
                    marked.insert(key.clone());
                    touched.push(key.clone());
                }
                if let Some(entry) = counts.get_mut(key) {
                    *entry -= n;
                    if *entry == 0 {
                        counts.remove(key);
                    }
                }
            }
        }
    }

    /// Whether the key is currently alive (count > 0).
    pub fn alive(&self, key: &K) -> bool {
        match self {
            Self::Dense(state) => state.alive(dense_raw(key)),
            Self::Hashed { counts, .. } => counts.contains_key(key),
        }
    }

    /// Register one left row under its key's bucket.
    pub fn left_insert(&mut self, key: &K, handle: RowHandle) {
        match self {
            Self::Dense(state) => state.left_insert(dense_raw(key), handle),
            Self::Hashed { left_buckets, .. } => {
                left_buckets.entry(key.clone()).or_default().push(handle)
            }
        }
    }

    /// Remove one left row from its key's bucket (bucket empties are dropped).
    pub fn left_remove(&mut self, key: &K, handle: RowHandle) {
        match self {
            Self::Dense(state) => state.left_remove(dense_raw(key), handle),
            Self::Hashed { left_buckets, .. } => {
                if let Some(bucket) = left_buckets.get_mut(key) {
                    if let Some(pos) = bucket.iter().position(|h| *h == handle) {
                        bucket.swap_remove(pos);
                    }
                    if bucket.is_empty() {
                        left_buckets.remove(key);
                    }
                }
            }
        }
    }

    /// The live left rows under one key.
    pub fn left_bucket<'a>(&'a self, key: &'a K) -> Option<&'a [RowHandle]> {
        match self {
            Self::Dense(state) => state.left_bucket(dense_raw(key)),
            Self::Hashed { left_buckets, .. } => left_buckets.get(key).map(Vec::as_slice),
        }
    }

    /// Begin a fresh event: clears the touched list only.
    pub fn begin_event(&mut self) {
        match self {
            Self::Dense(state) => state.begin_event(),
            Self::Hashed { .. } => {}
        }
    }

    /// Every key whose count changed during the current event, in first-touch
    /// order. The list drains; the next event starts unmarked.
    pub fn drain_touched(&mut self, out: &mut Vec<K>) {
        match self {
            Self::Dense(state) => {
                for raw in state.touched.drain(..) {
                    state.in_touched[raw] = false;
                    out.push(key_from_raw::<K>(raw));
                }
            }
            Self::Hashed {
                touched, marked, ..
            } => {
                marked.clear();
                out.append(touched);
            }
        }
    }

    pub fn clear(&mut self) {
        match self {
            Self::Dense(state) => state.clear(),
            Self::Hashed {
                counts,
                left_buckets,
                ..
            } => {
                counts.clear();
                left_buckets.clear();
            }
        }
    }
}

/// Dense `usize`-keyed membership state.
pub struct DenseKeys {
    counts: Vec<u32>,
    /// Whether each key is already in the current event's touched list.
    in_touched: Vec<bool>,
    /// The event currently accumulating.
    touched: Vec<usize>,
    left_buckets: Vec<Vec<RowHandle>>,
}

impl DenseKeys {
    pub fn new() -> Self {
        Self {
            counts: Vec::new(),
            in_touched: Vec::new(),
            touched: Vec::new(),
            left_buckets: Vec::new(),
        }
    }
    fn grow(&mut self, raw: usize) {
        if self.counts.len() <= raw {
            self.counts.resize(raw + 1, 0);
            self.in_touched.resize(raw + 1, false);
            self.left_buckets.resize_with(raw + 1, Vec::new);
        }
    }
    /* add/take are per-key-per-event hot calls: in-Touched check inlined
    (no separate `touch` frame), and the grow branch is the cheap len compare
    — `grow` itself only runs during warmup, never again. */
    #[inline]
    pub fn add(&mut self, raw: usize, n: usize) -> usize {
        if self.counts.len() <= raw {
            self.grow(raw);
        }
        self.counts[raw] += n as u32;
        if !self.in_touched[raw] {
            self.in_touched[raw] = true;
            self.touched.push(raw);
        }
        self.counts[raw] as usize
    }
    #[inline]
    pub fn take(&mut self, raw: usize, n: usize) {
        if self.counts.len() <= raw {
            self.grow(raw);
        }
        debug_assert!(self.counts[raw] >= n as u32, "count underflow");
        self.counts[raw] -= n as u32;
        if !self.in_touched[raw] {
            self.in_touched[raw] = true;
            self.touched.push(raw);
        }
    }
    #[inline]
    pub fn alive(&self, raw: usize) -> bool {
        self.counts.get(raw).copied().unwrap_or(0) > 0
    }
    #[inline]
    pub fn left_insert(&mut self, raw: usize, handle: RowHandle) {
        self.grow(raw);
        self.left_buckets[raw].push(handle);
    }
    #[inline]
    pub fn left_remove(&mut self, raw: usize, handle: RowHandle) {
        if raw >= self.left_buckets.len() {
            return;
        }
        if let Some(pos) = self.left_buckets[raw].iter().position(|h| *h == handle) {
            self.left_buckets[raw].swap_remove(pos);
        }
    }
    #[inline]
    pub fn left_bucket(&self, raw: usize) -> Option<&[RowHandle]> {
        self.left_buckets.get(raw).map(Vec::as_slice)
    }
    pub fn begin_event(&mut self) {
        for raw in self.touched.drain(..) {
            self.in_touched[raw] = false;
        }
    }
    pub fn clear(&mut self) {
        self.counts.clear();
        self.in_touched.clear();
        self.touched.clear();
        self.left_buckets.clear();
    }
}

/// View a generic key as its `usize` representation. Only called inside the
/// `K == usize` arm of `KeyState::new`, where the conversion is the identity.
fn dense_raw<K: 'static>(key: &K) -> usize {
    debug_assert_eq!(
        std::any::TypeId::of::<K>(),
        std::any::TypeId::of::<usize>(),
        "dense counts are usize-keyed"
    );
    // SAFETY: K is usize in this arm; the reference reinterprets the same
    // non-null, aligned, initialized 8 bytes.
    unsafe { *(key as *const K as *const usize) }
}

/// Inverse of `dense_raw`, same `K == usize` arm.
fn key_from_raw<K: 'static>(raw: usize) -> K {
    debug_assert_eq!(
        std::any::TypeId::of::<K>(),
        std::any::TypeId::of::<usize>(),
        "dense counts are usize-keyed"
    );
    // SAFETY: K is usize in this arm; usize is repr-transparent `K` here and
    // the pointer is valid, aligned, initialized.
    unsafe { std::ptr::read(&raw as *const usize as *const K) }
}
