/* KeyExtract trait and adapters for zero-erasure join key extraction.

The `KeyExtract` trait replaces bare `Fn(&S, &A, usize) -> K` bounds,
allowing named adapter structs to be used as associated types in `JoinTarget`.
*/

use std::marker::PhantomData;

/* Extracts a join key from a solution and entity.

Blanket impl for `Fn(&S, &A, usize) -> K` preserves all existing usage.
`EntityKeyAdapter` wraps entity-only key functions for the self-join case.
*/
pub trait KeyExtract<S, A, K>: Send + Sync {
    // Extracts the key from the entity.
    fn extract(&self, s: &S, a: &A, idx: usize) -> K;
}

impl<S, A, K, F> KeyExtract<S, A, K> for F
where
    F: Fn(&S, &A, usize) -> K + Send + Sync,
{
    #[inline]
    fn extract(&self, s: &S, a: &A, idx: usize) -> K {
        self(s, a, idx)
    }
}

/* Wraps an entity-only key function `Fn(&A) -> K` as a `KeyExtract`.

Used in the self-join case where the user passes `equal(|a: &A| key_fn(a))`.
The solution and index parameters are ignored.
*/
pub struct EntityKeyAdapter<KA> {
    key_fn: KA,
}

impl<KA> EntityKeyAdapter<KA> {
    // Creates a new `EntityKeyAdapter` from an entity-only key function.
    pub fn new(key_fn: KA) -> Self {
        Self { key_fn }
    }
}

impl<S, A, K, KA> KeyExtract<S, A, K> for EntityKeyAdapter<KA>
where
    KA: Fn(&A) -> K + Send + Sync,
{
    #[inline]
    fn extract(&self, _s: &S, a: &A, _idx: usize) -> K {
        (self.key_fn)(a)
    }
}

impl<KA: Clone> Clone for EntityKeyAdapter<KA> {
    fn clone(&self) -> Self {
        Self {
            key_fn: self.key_fn.clone(),
        }
    }
}

impl<KA: Copy> Copy for EntityKeyAdapter<KA> {}

// Phantom type parameter usage to avoid requiring Clone/Send on D.
pub struct PhantomKey<T>(PhantomData<fn() -> T>);
