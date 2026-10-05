/* Leaf source operator: collection extraction into retained handle rows.

Adapts a `CollectionExtract` into stable per-entity handles with an
authored `BindingId` and descriptor localization. Static sources
initialize but ignore descriptor updates; unknown sources keep the
checked-in contract (react to every descriptor, panic when they cannot
localize). Filtered extractors keep their `contains` membership gate:
only accepted entities enter the handle store and the key index.
*/

use std::hash::Hash;
use std::marker::PhantomData;

use super::super::collection_extract::{ChangeSource, CollectionExtract};
use super::{BindingId, DenseRowStore, HashIndex, Leaf, RowHandle};

/* One retained source: handles, key index, and localization metadata.

`K` is this source's key type for its join; `KF` extracts it from the
entity. Handles are 1:1 with accepted entities in traversal order, so a
source slice index always resolves to the same handle within a build.
The store payload is the slice index itself, keeping reverse lookups free.
*/
pub(crate) struct Source<S, A, E, K, KF> {
    extractor: E,
    key_fn: KF,
    binding: BindingId,
    change_source: ChangeSource,
    index: HashIndex<K>,
    handle_of: Vec<Option<RowHandle>>,
    store: DenseRowStore<usize>,
    _phantom: PhantomData<fn() -> (S, A)>,
}

impl<S, A, E, K, KF> Source<S, A, E, K, KF>
where
    E: CollectionExtract<S, Item = A>,
    K: Eq + Hash + Clone,
    KF: Fn(&A) -> K,
{
    pub(crate) fn new(
        extractor: E,
        key_fn: KF,
        binding: BindingId,
        change_source: ChangeSource,
    ) -> Source<S, A, E, K, KF> {
        Source {
            extractor,
            key_fn,
            binding,
            change_source,
            index: HashIndex::new(),
            handle_of: Vec::new(),
            store: DenseRowStore::new(),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn binding(&self) -> BindingId {
        self.binding
    }

    pub(crate) fn extractor_ref(&self) -> &E {
        &self.extractor
    }

    pub(crate) fn key_ref(&self) -> &KF {
        &self.key_fn
    }

    /* Slice index retained under a handle (the store payload). */
    pub(crate) fn index_of(&self, handle: RowHandle) -> Option<usize> {
        self.store.get(handle).copied()
    }

    /* Derives the current key for an accepted entity without retaining. */
    pub(crate) fn key_at(&self, solution: &S, idx: usize) -> Option<K> {
        let leaf = self.leaf(solution, idx)?;
        Some((self.key_fn)(leaf.entity))
    }

    /* Mints a handle and key for one entity without touching the rest.

    Returns None when the entity is absent, rejected by membership, or
    already live (duplicate notification). Targeted inserts keep every
    other handle stable; only `refresh` rebuilds the whole source.
    */
    pub(crate) fn insert_idx(&mut self, solution: &S, idx: usize) -> Option<RowHandle> {
        if self.handle_for(idx).is_some() {
            return None;
        }
        let entities = self.extractor.extract(solution);
        let entity = entities.get(idx)?;
        if !self.extractor.contains(solution, entity) {
            return None;
        }
        let handle = self.store.insert(idx);
        let key = (self.key_fn)(entity);
        self.index.insert(handle, key);
        if let Some(slot) = self.handle_of.get_mut(idx) {
            *slot = Some(handle);
        } else {
            while self.handle_of.len() < idx {
                self.handle_of.push(None);
            }
            self.handle_of.push(Some(handle));
        }
        Some(handle)
    }

    /* Forgets one entity's handle and key after its outputs retracted. */
    pub(crate) fn forget(&mut self, idx: usize) {
        if let Some(handle) = self.handle_of.get(idx).copied().flatten() {
            self.index.remove(handle);
            self.store.retract(handle);
            if let Some(slot) = self.handle_of.get_mut(idx) {
                *slot = None;
            }
        }
    }

    /* Rebuilds handles and the key index from the current solution.

    Called on initialization and on any notification this source owns.
    Membership (`contains`) gates entry; accepted entities keep traversal
    order so slice indexes stay stable across rebuilds.
    */
    pub(crate) fn refresh(&mut self, solution: &S) {
        self.index.clear();
        self.store.clear();
        self.handle_of.clear();
        let entities = self.extractor.extract(solution);
        self.handle_of.reserve(entities.len());
        for (idx, entity) in entities.iter().enumerate() {
            if !self.extractor.contains(solution, entity) {
                self.handle_of.push(None);
                continue;
            }
            let handle = self.store.insert(idx);
            let key = (self.key_fn)(entity);
            self.index.insert(handle, key);
            self.handle_of.push(Some(handle));
        }
    }

    pub(crate) fn handle_for(&self, idx: usize) -> Option<RowHandle> {
        self.handle_of.get(idx).copied().flatten()
    }

    pub(crate) fn lookup(&self, key: &K) -> &[RowHandle] {
        self.index.lookup(key)
    }

    /* Borrows one accepted entity as a leaf row with its semantic index. */
    pub(crate) fn leaf<'s>(&self, solution: &'s S, idx: usize) -> Option<Leaf<'s, A>> {
        let entities = self.extractor.extract(solution);
        let entity = entities.get(idx)?;
        self.handle_for(idx)?;
        Some(Leaf::new(entity, idx))
    }

    pub(crate) fn assert_localizes(&self, descriptor: usize, name: &str) -> bool {
        self.change_source.assert_localizes(descriptor, name)
    }
}
