//! Tests for the move module.

use super::*;
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::{RecordingScoreDirector, ScoreDirector, SimpleScoreDirector};
use std::any::TypeId;

// =============================================================================
// MoveArena tests
// =============================================================================

mod arena_tests {
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

// =============================================================================
// ChangeMove tests
// =============================================================================

mod change_tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Task {
        id: usize,
        priority: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct TaskSolution {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TaskSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_priority(s: &TaskSolution, i: usize) -> Option<i32> {
        s.tasks.get(i).and_then(|t| t.priority)
    }

    fn set_priority(s: &mut TaskSolution, i: usize, v: Option<i32>) {
        if let Some(task) = s.tasks.get_mut(i) {
            task.priority = v;
        }
    }

    fn create_director(
        tasks: Vec<Task>,
    ) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
        let solution = TaskSolution { tasks, score: None };
        let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>());
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn test_change_move_is_doable() {
        let tasks = vec![
            Task {
                id: 0,
                priority: Some(1),
            },
            Task {
                id: 1,
                priority: Some(2),
            },
        ];
        let director = create_director(tasks);

        // Different value - doable
        let m = ChangeMove::<_, i32>::new(0, Some(5), get_priority, set_priority, "priority", 0);
        assert!(m.is_doable(&director));

        // Same value - not doable
        let m = ChangeMove::<_, i32>::new(0, Some(1), get_priority, set_priority, "priority", 0);
        assert!(!m.is_doable(&director));
    }

    #[test]
    fn test_change_move_do_move() {
        let tasks = vec![Task {
            id: 0,
            priority: Some(1),
        }];
        let mut director = create_director(tasks);

        let m = ChangeMove::<_, i32>::new(0, Some(5), get_priority, set_priority, "priority", 0);
        m.do_move(&mut director);

        let val = get_priority(director.working_solution(), 0);
        assert_eq!(val, Some(5));
    }

    #[test]
    fn test_change_move_to_none() {
        let tasks = vec![Task {
            id: 0,
            priority: Some(5),
        }];
        let mut director = create_director(tasks);

        let m = ChangeMove::<_, i32>::new(0, None, get_priority, set_priority, "priority", 0);
        assert!(m.is_doable(&director));

        m.do_move(&mut director);

        let val = get_priority(director.working_solution(), 0);
        assert_eq!(val, None);
    }

    #[test]
    fn test_change_move_entity_indices() {
        let m = ChangeMove::<TaskSolution, i32>::new(
            3,
            Some(5),
            get_priority,
            set_priority,
            "priority",
            0,
        );
        assert_eq!(m.entity_indices(), &[3]);
    }

    #[test]
    fn test_change_move_copy() {
        let m1 = ChangeMove::<TaskSolution, i32>::new(
            0,
            Some(5),
            get_priority,
            set_priority,
            "priority",
            0,
        );
        let m2 = m1;
        assert_eq!(m1.entity_index(), m2.entity_index());
        assert_eq!(m1.to_value(), m2.to_value());
    }
}

// =============================================================================
// SwapMove tests
// =============================================================================

mod swap_tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Task {
        id: usize,
        priority: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct TaskSolution {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TaskSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
        &s.tasks
    }

    fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
        &mut s.tasks
    }

    fn get_priority(s: &TaskSolution, idx: usize) -> Option<i32> {
        s.tasks.get(idx).and_then(|t| t.priority)
    }

    fn set_priority(s: &mut TaskSolution, idx: usize, v: Option<i32>) {
        if let Some(task) = s.tasks.get_mut(idx) {
            task.priority = v;
        }
    }

    fn create_director(
        tasks: Vec<Task>,
    ) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
        let solution = TaskSolution { tasks, score: None };

        let extractor = Box::new(TypedEntityExtractor::new(
            "Task",
            "tasks",
            get_tasks,
            get_tasks_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);

        let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>())
            .with_entity(entity_desc);

        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn test_swap_move_do_and_undo() {
        let tasks = vec![
            Task {
                id: 0,
                priority: Some(1),
            },
            Task {
                id: 1,
                priority: Some(5),
            },
        ];
        let mut director = create_director(tasks);

        let m = SwapMove::<TaskSolution, i32>::new(0, 1, get_priority, set_priority, "priority", 0);
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            assert_eq!(get_priority(recording.working_solution(), 0), Some(5));
            assert_eq!(get_priority(recording.working_solution(), 1), Some(1));

            recording.undo_changes();
        }

        assert_eq!(get_priority(director.working_solution(), 0), Some(1));
        assert_eq!(get_priority(director.working_solution(), 1), Some(5));

        let solution = director.working_solution();
        assert_eq!(solution.tasks[0].id, 0);
        assert_eq!(solution.tasks[1].id, 1);
    }

    #[test]
    fn test_swap_same_value_not_doable() {
        let tasks = vec![
            Task {
                id: 0,
                priority: Some(5),
            },
            Task {
                id: 1,
                priority: Some(5),
            },
        ];
        let director = create_director(tasks);

        let m = SwapMove::<TaskSolution, i32>::new(0, 1, get_priority, set_priority, "priority", 0);
        assert!(
            !m.is_doable(&director),
            "swapping same values should not be doable"
        );
    }

    #[test]
    fn test_swap_self_not_doable() {
        let tasks = vec![Task {
            id: 0,
            priority: Some(1),
        }];
        let director = create_director(tasks);

        let m = SwapMove::<TaskSolution, i32>::new(0, 0, get_priority, set_priority, "priority", 0);
        assert!(!m.is_doable(&director), "self-swap should not be doable");
    }

    #[test]
    fn test_swap_entity_indices() {
        let m = SwapMove::<TaskSolution, i32>::new(2, 5, get_priority, set_priority, "priority", 0);
        assert_eq!(m.entity_indices(), &[2, 5]);
    }
}

// =============================================================================
// PillarChangeMove tests
// =============================================================================

mod pillar_change_tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Employee {
        id: usize,
        shift: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct ScheduleSolution {
        employees: Vec<Employee>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for ScheduleSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_shift(s: &ScheduleSolution, idx: usize) -> Option<i32> {
        s.employees.get(idx).and_then(|e| e.shift)
    }

    fn set_shift(s: &mut ScheduleSolution, idx: usize, v: Option<i32>) {
        if let Some(e) = s.employees.get_mut(idx) {
            e.shift = v;
        }
    }

    fn create_director(
        employees: Vec<Employee>,
    ) -> SimpleScoreDirector<ScheduleSolution, impl Fn(&ScheduleSolution) -> SimpleScore> {
        let solution = ScheduleSolution {
            employees,
            score: None,
        };

        let extractor = Box::new(TypedEntityExtractor::new(
            "Employee",
            "employees",
            |s: &ScheduleSolution| &s.employees,
            |s: &mut ScheduleSolution| &mut s.employees,
        ));
        let entity_desc = EntityDescriptor::new("Employee", TypeId::of::<Employee>(), "employees")
            .with_extractor(extractor);

        let descriptor =
            SolutionDescriptor::new("ScheduleSolution", TypeId::of::<ScheduleSolution>())
                .with_entity(entity_desc);

        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn test_pillar_change_all_entities() {
        let mut director = create_director(vec![
            Employee {
                id: 0,
                shift: Some(1),
            },
            Employee {
                id: 1,
                shift: Some(1),
            },
            Employee {
                id: 2,
                shift: Some(2),
            },
        ]);

        let m = PillarChangeMove::<ScheduleSolution, i32>::new(
            vec![0, 1],
            Some(5),
            get_shift,
            set_shift,
            "shift",
            0,
        );

        assert!(m.is_doable(&director));
        assert_eq!(m.pillar_size(), 2);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            assert_eq!(get_shift(recording.working_solution(), 0), Some(5));
            assert_eq!(get_shift(recording.working_solution(), 1), Some(5));
            assert_eq!(get_shift(recording.working_solution(), 2), Some(2));

            recording.undo_changes();
        }

        assert_eq!(get_shift(director.working_solution(), 0), Some(1));
        assert_eq!(get_shift(director.working_solution(), 1), Some(1));
        assert_eq!(get_shift(director.working_solution(), 2), Some(2));

        let solution = director.working_solution();
        assert_eq!(solution.employees[0].id, 0);
        assert_eq!(solution.employees[1].id, 1);
        assert_eq!(solution.employees[2].id, 2);
    }

    #[test]
    fn test_pillar_change_same_value_not_doable() {
        let director = create_director(vec![
            Employee {
                id: 0,
                shift: Some(5),
            },
            Employee {
                id: 1,
                shift: Some(5),
            },
        ]);

        let m = PillarChangeMove::<ScheduleSolution, i32>::new(
            vec![0, 1],
            Some(5),
            get_shift,
            set_shift,
            "shift",
            0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn test_pillar_change_empty_pillar_not_doable() {
        let director = create_director(vec![Employee {
            id: 0,
            shift: Some(1),
        }]);

        let m = PillarChangeMove::<ScheduleSolution, i32>::new(
            vec![],
            Some(5),
            get_shift,
            set_shift,
            "shift",
            0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn test_pillar_change_entity_indices() {
        let m = PillarChangeMove::<ScheduleSolution, i32>::new(
            vec![1, 3, 5],
            Some(5),
            get_shift,
            set_shift,
            "shift",
            0,
        );
        assert_eq!(m.entity_indices(), &[1, 3, 5]);
    }
}

// =============================================================================
// PillarSwapMove tests
// =============================================================================

mod pillar_swap_tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Employee {
        id: usize,
        shift: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct Solution {
        employees: Vec<Employee>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for Solution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_shift(s: &Solution, idx: usize) -> Option<i32> {
        s.employees.get(idx).and_then(|e| e.shift)
    }

    fn set_shift(s: &mut Solution, idx: usize, v: Option<i32>) {
        if let Some(e) = s.employees.get_mut(idx) {
            e.shift = v;
        }
    }

    fn create_director(
        employees: Vec<Employee>,
    ) -> SimpleScoreDirector<Solution, impl Fn(&Solution) -> SimpleScore> {
        let solution = Solution {
            employees,
            score: None,
        };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Employee",
            "employees",
            |s: &Solution| &s.employees,
            |s: &mut Solution| &mut s.employees,
        ));
        let entity_desc = EntityDescriptor::new("Employee", TypeId::of::<Employee>(), "employees")
            .with_extractor(extractor);
        let descriptor =
            SolutionDescriptor::new("Solution", TypeId::of::<Solution>()).with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn test_pillar_swap_all_entities() {
        let mut director = create_director(vec![
            Employee {
                id: 0,
                shift: Some(1),
            },
            Employee {
                id: 1,
                shift: Some(1),
            },
            Employee {
                id: 2,
                shift: Some(2),
            },
            Employee {
                id: 3,
                shift: Some(2),
            },
        ]);

        let m = PillarSwapMove::<Solution, i32>::new(
            vec![0, 1],
            vec![2, 3],
            get_shift,
            set_shift,
            "shift",
            0,
        );
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            assert_eq!(get_shift(recording.working_solution(), 0), Some(2));
            assert_eq!(get_shift(recording.working_solution(), 1), Some(2));
            assert_eq!(get_shift(recording.working_solution(), 2), Some(1));
            assert_eq!(get_shift(recording.working_solution(), 3), Some(1));

            recording.undo_changes();
        }

        assert_eq!(get_shift(director.working_solution(), 0), Some(1));
        assert_eq!(get_shift(director.working_solution(), 1), Some(1));
        assert_eq!(get_shift(director.working_solution(), 2), Some(2));
        assert_eq!(get_shift(director.working_solution(), 3), Some(2));

        let solution = director.working_solution();
        assert_eq!(solution.employees[0].id, 0);
        assert_eq!(solution.employees[1].id, 1);
        assert_eq!(solution.employees[2].id, 2);
        assert_eq!(solution.employees[3].id, 3);
    }

    #[test]
    fn test_pillar_swap_same_value_not_doable() {
        let director = create_director(vec![
            Employee {
                id: 0,
                shift: Some(1),
            },
            Employee {
                id: 1,
                shift: Some(1),
            },
        ]);
        let m = PillarSwapMove::<Solution, i32>::new(
            vec![0],
            vec![1],
            get_shift,
            set_shift,
            "shift",
            0,
        );
        assert!(!m.is_doable(&director));
    }

    #[test]
    fn test_pillar_swap_empty_pillar_not_doable() {
        let director = create_director(vec![Employee {
            id: 0,
            shift: Some(1),
        }]);
        let m =
            PillarSwapMove::<Solution, i32>::new(vec![], vec![0], get_shift, set_shift, "shift", 0);
        assert!(!m.is_doable(&director));
    }
}

// =============================================================================
// RuinMove tests
// =============================================================================

mod ruin_tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Task {
        assigned_to: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct Schedule {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for Schedule {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_tasks(s: &Schedule) -> &Vec<Task> {
        &s.tasks
    }
    fn get_tasks_mut(s: &mut Schedule) -> &mut Vec<Task> {
        &mut s.tasks
    }

    fn get_assigned(s: &Schedule, idx: usize) -> Option<i32> {
        s.tasks.get(idx).and_then(|t| t.assigned_to)
    }
    fn set_assigned(s: &mut Schedule, idx: usize, v: Option<i32>) {
        if let Some(t) = s.tasks.get_mut(idx) {
            t.assigned_to = v;
        }
    }

    fn create_director(
        assignments: &[Option<i32>],
    ) -> SimpleScoreDirector<Schedule, impl Fn(&Schedule) -> SimpleScore> {
        let tasks: Vec<Task> = assignments
            .iter()
            .map(|&a| Task { assigned_to: a })
            .collect();
        let solution = Schedule { tasks, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Task",
            "tasks",
            get_tasks,
            get_tasks_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
        let descriptor =
            SolutionDescriptor::new("Schedule", TypeId::of::<Schedule>()).with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn ruin_single_entity() {
        let mut director = create_director(&[Some(1), Some(2), Some(3)]);

        let m = RuinMove::<Schedule, i32>::new(&[1], get_assigned, set_assigned, "assigned_to", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            assert_eq!(get_assigned(recording.working_solution(), 0), Some(1));
            assert_eq!(get_assigned(recording.working_solution(), 1), None);
            assert_eq!(get_assigned(recording.working_solution(), 2), Some(3));

            recording.undo_changes();
        }

        assert_eq!(get_assigned(director.working_solution(), 1), Some(2));
    }

    #[test]
    fn ruin_multiple_entities() {
        let mut director = create_director(&[Some(1), Some(2), Some(3), Some(4)]);

        let m = RuinMove::<Schedule, i32>::new(
            &[0, 2, 3],
            get_assigned,
            set_assigned,
            "assigned_to",
            0,
        );

        assert!(m.is_doable(&director));
        assert_eq!(m.ruin_count(), 3);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            assert_eq!(get_assigned(recording.working_solution(), 0), None);
            assert_eq!(get_assigned(recording.working_solution(), 1), Some(2));
            assert_eq!(get_assigned(recording.working_solution(), 2), None);
            assert_eq!(get_assigned(recording.working_solution(), 3), None);

            recording.undo_changes();
        }

        assert_eq!(get_assigned(director.working_solution(), 0), Some(1));
        assert_eq!(get_assigned(director.working_solution(), 2), Some(3));
        assert_eq!(get_assigned(director.working_solution(), 3), Some(4));
    }

    #[test]
    fn ruin_already_unassigned_is_doable() {
        let director = create_director(&[Some(1), None]);

        let m =
            RuinMove::<Schedule, i32>::new(&[0, 1], get_assigned, set_assigned, "assigned_to", 0);

        assert!(m.is_doable(&director));
    }

    #[test]
    fn ruin_all_unassigned_not_doable() {
        let director = create_director(&[None, None]);

        let m =
            RuinMove::<Schedule, i32>::new(&[0, 1], get_assigned, set_assigned, "assigned_to", 0);

        assert!(!m.is_doable(&director));
    }
}

// =============================================================================
// ListChangeMove tests
// =============================================================================

mod list_change_tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Vehicle {
        visits: Vec<i32>,
    }

    #[derive(Clone, Debug)]
    struct RoutingSolution {
        vehicles: Vec<Vehicle>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for RoutingSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_vehicles(s: &RoutingSolution) -> &Vec<Vehicle> {
        &s.vehicles
    }
    fn get_vehicles_mut(s: &mut RoutingSolution) -> &mut Vec<Vehicle> {
        &mut s.vehicles
    }

    fn list_len(s: &RoutingSolution, entity_idx: usize) -> usize {
        s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
    }
    fn list_remove(s: &mut RoutingSolution, entity_idx: usize, pos: usize) -> Option<i32> {
        s.vehicles.get_mut(entity_idx).map(|v| v.visits.remove(pos))
    }
    fn list_insert(s: &mut RoutingSolution, entity_idx: usize, pos: usize, val: i32) {
        if let Some(v) = s.vehicles.get_mut(entity_idx) {
            v.visits.insert(pos, val);
        }
    }

    fn create_director(
        vehicles: Vec<Vehicle>,
    ) -> SimpleScoreDirector<RoutingSolution, impl Fn(&RoutingSolution) -> SimpleScore> {
        let solution = RoutingSolution {
            vehicles,
            score: None,
        };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Vehicle",
            "vehicles",
            get_vehicles,
            get_vehicles_mut,
        ));
        let entity_desc = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
            .with_extractor(extractor);
        let descriptor =
            SolutionDescriptor::new("RoutingSolution", TypeId::of::<RoutingSolution>())
                .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn intra_list_move_forward() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let mut director = create_director(vehicles);

        let m = ListChangeMove::<RoutingSolution, i32>::new(
            0,
            1,
            0,
            3,
            list_len,
            list_remove,
            list_insert,
            "visits",
            0,
        );

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let visits = &recording.working_solution().vehicles[0].visits;
            assert_eq!(visits, &[1, 3, 2, 4, 5]);

            recording.undo_changes();
        }

        let visits = &director.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn intra_list_move_backward() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let mut director = create_director(vehicles);

        let m = ListChangeMove::<RoutingSolution, i32>::new(
            0,
            3,
            0,
            1,
            list_len,
            list_remove,
            list_insert,
            "visits",
            0,
        );

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let visits = &recording.working_solution().vehicles[0].visits;
            assert_eq!(visits, &[1, 4, 2, 3, 5]);

            recording.undo_changes();
        }

        let visits = &director.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn inter_list_move() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3],
            },
            Vehicle {
                visits: vec![10, 20],
            },
        ];
        let mut director = create_director(vehicles);

        let m = ListChangeMove::<RoutingSolution, i32>::new(
            0,
            1,
            1,
            1,
            list_len,
            list_remove,
            list_insert,
            "visits",
            0,
        );

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let sol = recording.working_solution();
            assert_eq!(sol.vehicles[0].visits, vec![1, 3]);
            assert_eq!(sol.vehicles[1].visits, vec![10, 2, 20]);

            recording.undo_changes();
        }

        let sol = director.working_solution();
        assert_eq!(sol.vehicles[0].visits, vec![1, 2, 3]);
        assert_eq!(sol.vehicles[1].visits, vec![10, 20]);
    }

    #[test]
    fn same_position_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = ListChangeMove::<RoutingSolution, i32>::new(
            0,
            1,
            0,
            1,
            list_len,
            list_remove,
            list_insert,
            "visits",
            0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn invalid_source_position_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = ListChangeMove::<RoutingSolution, i32>::new(
            0,
            10,
            0,
            0,
            list_len,
            list_remove,
            list_insert,
            "visits",
            0,
        );

        assert!(!m.is_doable(&director));
    }
}

// =============================================================================
// ListSwapMove tests
// =============================================================================

mod list_swap_tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Vehicle {
        visits: Vec<i32>,
    }

    #[derive(Clone, Debug)]
    struct RoutingSolution {
        vehicles: Vec<Vehicle>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for RoutingSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_vehicles(s: &RoutingSolution) -> &Vec<Vehicle> {
        &s.vehicles
    }
    fn get_vehicles_mut(s: &mut RoutingSolution) -> &mut Vec<Vehicle> {
        &mut s.vehicles
    }

    fn list_len(s: &RoutingSolution, entity_idx: usize) -> usize {
        s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
    }
    fn list_get(s: &RoutingSolution, entity_idx: usize, pos: usize) -> Option<i32> {
        s.vehicles
            .get(entity_idx)
            .and_then(|v| v.visits.get(pos).copied())
    }
    fn list_set(s: &mut RoutingSolution, entity_idx: usize, pos: usize, val: i32) {
        if let Some(v) = s.vehicles.get_mut(entity_idx) {
            if let Some(elem) = v.visits.get_mut(pos) {
                *elem = val;
            }
        }
    }

    fn create_director(
        vehicles: Vec<Vehicle>,
    ) -> SimpleScoreDirector<RoutingSolution, impl Fn(&RoutingSolution) -> SimpleScore> {
        let solution = RoutingSolution {
            vehicles,
            score: None,
        };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Vehicle",
            "vehicles",
            get_vehicles,
            get_vehicles_mut,
        ));
        let entity_desc = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
            .with_extractor(extractor);
        let descriptor =
            SolutionDescriptor::new("RoutingSolution", TypeId::of::<RoutingSolution>())
                .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn intra_list_swap() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let mut director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 1, 0, 3, list_len, list_get, list_set, "visits", 0,
        );

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let visits = &recording.working_solution().vehicles[0].visits;
            assert_eq!(visits, &[1, 4, 3, 2, 5]);

            recording.undo_changes();
        }

        let visits = &director.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn inter_list_swap() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3],
            },
            Vehicle {
                visits: vec![10, 20, 30],
            },
        ];
        let mut director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 1, 1, 2, list_len, list_get, list_set, "visits", 0,
        );

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let sol = recording.working_solution();
            assert_eq!(sol.vehicles[0].visits, vec![1, 30, 3]);
            assert_eq!(sol.vehicles[1].visits, vec![10, 20, 2]);

            recording.undo_changes();
        }

        let sol = director.working_solution();
        assert_eq!(sol.vehicles[0].visits, vec![1, 2, 3]);
        assert_eq!(sol.vehicles[1].visits, vec![10, 20, 30]);
    }

    #[test]
    fn same_position_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 1, 0, 1, list_len, list_get, list_set, "visits", 0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn same_values_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![5, 5, 5],
        }];
        let director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 0, 0, 2, list_len, list_get, list_set, "visits", 0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn invalid_position_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 1, 0, 10, list_len, list_get, list_set, "visits", 0,
        );

        assert!(!m.is_doable(&director));
    }
}
