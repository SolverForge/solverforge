use super::*;

#[test]
fn test_task_creation() {
    let t1 = Task::new(Some(5));
    assert_eq!(t1.priority, Some(5));

    let t2 = Task::with_priority(10);
    assert_eq!(t2.priority, Some(10));

    let t3 = Task::unassigned();
    assert_eq!(t3.priority, None);
}

#[test]
fn test_solution_creation() {
    let s1 = TaskSolution::unassigned(3);
    assert_eq!(s1.tasks.len(), 3);
    assert!(s1.tasks.iter().all(|t| t.priority.is_none()));

    let s2 = TaskSolution::new(vec![Task::with_priority(1), Task::with_priority(2)]);
    assert_eq!(s2.tasks.len(), 2);
}

#[test]
fn test_get_set_priority() {
    let mut solution = TaskSolution::new(vec![Task::unassigned(), Task::unassigned()]);

    assert_eq!(get_priority(&solution, 0), None);
    set_priority(&mut solution, 0, Some(5));
    assert_eq!(get_priority(&solution, 0), Some(5));
}
