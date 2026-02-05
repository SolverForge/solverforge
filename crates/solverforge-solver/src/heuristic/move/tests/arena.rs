//! MoveArena tests.

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
    assert_eq!(arena.capacity(), capacity_before);
}

#[test]
fn test_arena_reuse_after_reset() {
    let mut arena: MoveArena<i32> = MoveArena::new();

    arena.push(1);
    arena.push(2);
    assert_eq!(arena.len(), 2);

    arena.reset();

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

    arena.reset();
    assert!(arena.is_empty());

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
    arena.take(1);
}
