//! Arena allocator for moves.
//!
//! Provides O(1) step cleanup by reusing a pre-allocated buffer.
//! Instead of allocating a new Vec each step, the arena is reset
//! and reused.

use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::ptr;

use rand::seq::SliceRandom;
use rand::Rng;

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

    /// Shuffles the initialized moves in-place using Fisher-Yates.
    ///
    /// This avoids re-generating and re-collecting all moves each step.
    /// The arena keeps its existing storage and just randomises evaluation order.
    #[inline]
    pub fn shuffle<R: Rng>(&mut self, rng: &mut R) {
        if self.len < 2 {
            return;
        }
        // Fisher-Yates on the initialized portion of storage
        let slice = &mut self.storage[..self.len];
        slice.shuffle(rng);
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
