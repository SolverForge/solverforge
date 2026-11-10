//! Undo storage for list moves with arbitrary-length edit sets.

/// Inlined undo journal for short list edit sets (K-opt route restorations
/// and similar arbitrary-length undo payloads). Capacity is inline up to 24
/// elements; longer routes spill to the heap and rely on the move's
/// per-trial journal pool so retained capacity is reused across trials.
pub(crate) type ListUndoJournal<T> = smallvec::SmallVec<[T; 24]>;
