//! Arena allocator for moves.
//!
//! Provides O(1) step cleanup by reusing a pre-allocated buffer.
//! Instead of allocating a new Vec each step, the arena is reset
//! and reused.

use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::ptr;

/// Arena allocator for moves with O(1) reset.
///
/// Instead of allocating a new Vec<M> each step and letting it drop,
/// the arena maintains a reusable buffer. Calling `reset()` simply
/// sets the length to 0 without running destructors (moves are Copy-like
/// in practice since they contain only primitives and small inline data).
///
/// # Performance
///
/// | Operation | Vec per step | MoveArena |
/// |-----------|--------------|-----------|
/// | Alloc     | O(n) heap    | O(1) bump |
/// | Cleanup   | O(n) drop    | O(1) reset|
/// | Memory    | n * size_of  | Reused    |
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::MoveArena;
///
/// let mut arena: MoveArena<i32> = MoveArena::new();
///
/// // Step 1: generate and evaluate moves
/// arena.push(1);
/// arena.push(2);
/// arena.push(3);
/// assert_eq!(arena.len(), 3);
///
/// // Step 2: reset and reuse (O(1)!)
/// arena.reset();
/// assert!(arena.is_empty());
///
/// arena.push(10);
/// arena.push(20);
/// for mov in arena.iter() {
///     assert!(*mov >= 10);
/// }
/// ```
pub struct MoveArena<M> {
    /// Storage for moves. We use MaybeUninit to avoid requiring Default.
    storage: Vec<MaybeUninit<M>>,
    /// Number of valid moves currently in the arena.
    len: usize,
    /// Index of the taken move (if any). Only one move can be taken per step.
    taken: Option<usize>,
}

impl<M> MoveArena<M> {
    /// Creates a new empty arena.
    #[inline]
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
            len: 0,
            taken: None,
        }
    }

    /// Creates a new arena with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            storage: Vec::with_capacity(capacity),
            len: 0,
            taken: None,
        }
    }

    /// Resets the arena, making it empty.
    ///
    /// Drops all moves except the one that was taken (if any).
    #[inline]
    pub fn reset(&mut self) {
        // Drop existing moves except the taken one (already moved out)
        for i in 0..self.len {
            if self.taken != Some(i) {
                unsafe {
                    ptr::drop_in_place(self.storage[i].as_mut_ptr());
                }
            }
        }
        self.len = 0;
        self.taken = None;
    }

    /// Adds a move to the arena.
    #[inline]
    pub fn push(&mut self, m: M) {
        if self.len < self.storage.len() {
            // Reuse existing slot
            self.storage[self.len] = MaybeUninit::new(m);
        } else {
            // Need to grow
            self.storage.push(MaybeUninit::new(m));
        }
        self.len += 1;
    }

    /// Returns the number of moves in the arena.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the arena is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns an iterator over the moves.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &M> {
        self.storage[..self.len]
            .iter()
            .map(|slot| unsafe { slot.assume_init_ref() })
    }

    /// Returns a mutable iterator over the moves.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut M> {
        self.storage[..self.len]
            .iter_mut()
            .map(|slot| unsafe { slot.assume_init_mut() })
    }

    /// Gets a move by index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&M> {
        if index < self.len {
            Some(unsafe { self.storage[index].assume_init_ref() })
        } else {
            None
        }
    }

    /// Takes ownership of a move by index.
    ///
    /// Only one move can be taken per step. Call `reset()` before taking another.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len` or if a move was already taken.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::heuristic::r#move::MoveArena;
    ///
    /// let mut arena: MoveArena<String> = MoveArena::new();
    /// arena.push("first".to_string());
    /// arena.push("second".to_string());
    ///
    /// // Take ownership of the move at index 1
    /// let taken = arena.take(1);
    /// assert_eq!(taken, "second");
    ///
    /// // Reset before next step
    /// arena.reset();
    /// ```
    #[inline]
    pub fn take(&mut self, index: usize) -> M {
        assert!(index < self.len, "index out of bounds");
        assert!(self.taken.is_none(), "move already taken this step");
        self.taken = Some(index);
        unsafe { self.storage[index].assume_init_read() }
    }

    /// Extends the arena from an iterator.
    #[inline]
    pub fn extend<I: IntoIterator<Item = M>>(&mut self, iter: I) {
        for m in iter {
            self.push(m);
        }
    }

    /// Returns the capacity of the arena.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.storage.capacity()
    }
}

impl<M> Default for MoveArena<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Debug> Debug for MoveArena<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoveArena")
            .field("len", &self.len)
            .field("capacity", &self.storage.capacity())
            .finish()
    }
}

impl<M> Drop for MoveArena<M> {
    fn drop(&mut self) {
        // Drop all initialized moves except taken one
        for i in 0..self.len {
            if self.taken != Some(i) {
                unsafe {
                    ptr::drop_in_place(self.storage[i].as_mut_ptr());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_basic() {
        let mut arena: MoveArena<i32> = MoveArena::new();
        assert!(arena.is_empty());

        arena.push(1);
        arena.push(2);
        arena.push(3);

        assert_eq!(arena.len(), 3);
        assert_eq!(arena.get(0), Some(&1));
        assert_eq!(arena.get(1), Some(&2));
        assert_eq!(arena.get(2), Some(&3));
        assert_eq!(arena.get(3), None);
    }

    #[test]
    fn test_arena_reset() {
        let mut arena: MoveArena<i32> = MoveArena::new();
        arena.push(1);
        arena.push(2);
        arena.push(3);

        let capacity_before = arena.capacity();

        arena.reset();

        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
        // Capacity is preserved
        assert_eq!(arena.capacity(), capacity_before);
    }

    #[test]
    fn test_arena_reuse_after_reset() {
        let mut arena: MoveArena<i32> = MoveArena::new();

        // First step
        arena.push(1);
        arena.push(2);
        assert_eq!(arena.len(), 2);

        arena.reset();

        // Second step - reuses storage
        arena.push(10);
        arena.push(20);
        arena.push(30);
        assert_eq!(arena.len(), 3);
        assert_eq!(arena.get(0), Some(&10));
        assert_eq!(arena.get(1), Some(&20));
        assert_eq!(arena.get(2), Some(&30));
    }

    #[test]
    fn test_arena_iter() {
        let mut arena: MoveArena<i32> = MoveArena::new();
        arena.push(1);
        arena.push(2);
        arena.push(3);

        let collected: Vec<_> = arena.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn test_arena_extend() {
        let mut arena: MoveArena<i32> = MoveArena::new();
        arena.extend(vec![1, 2, 3]);
        assert_eq!(arena.len(), 3);

        let collected: Vec<_> = arena.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn test_arena_with_capacity() {
        let arena: MoveArena<i32> = MoveArena::with_capacity(100);
        assert!(arena.is_empty());
        assert!(arena.capacity() >= 100);
    }

    #[test]
    fn test_arena_take() {
        let mut arena: MoveArena<String> = MoveArena::new();
        arena.push("a".to_string());
        arena.push("b".to_string());
        arena.push("c".to_string());

        let taken = arena.take(1);
        assert_eq!(taken, "b");

        // Reset clears everything including taken tracking
        arena.reset();
        assert!(arena.is_empty());

        // Can take again after reset
        arena.push("x".to_string());
        let taken = arena.take(0);
        assert_eq!(taken, "x");
    }

    #[test]
    #[should_panic(expected = "move already taken")]
    fn test_arena_double_take_panics() {
        let mut arena: MoveArena<i32> = MoveArena::new();
        arena.push(1);
        arena.push(2);
        arena.take(0);
        arena.take(1); // Should panic
    }
}
