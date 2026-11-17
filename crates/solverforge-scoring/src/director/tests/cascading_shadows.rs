// Cascading list shadows scored through ScoreDirector: tasks on several
// machines wait on tasks in other machines' lists, and the incremental score
// must equal a fresh score after every move and every undo.

use solverforge_core::domain::{CascadePropagation, PlanningSolution};
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::incremental::IncrementalUniConstraint;
use crate::director::score_director::ScoreDirector;
use crate::stream::collection_extract::{source, ChangeSource, SourceExtract};

const MACHINES: usize = 0;
const TASKS: usize = 1;

#[derive(Clone, Debug, Default)]
struct Task {
    duration: i64,
    waits_on: Option<usize>,
    machine: Option<usize>,
    previous: Option<usize>,
    next: Option<usize>,
    start: Option<i64>,
}

#[derive(Clone, Debug)]
struct Shop {
    machines: Vec<Vec<usize>>,
    tasks: Vec<Task>,
    score: Option<SoftScore>,
}

impl Shop {
    fn end(&self, task: usize) -> Option<i64> {
        self.tasks[task]
            .start
            .map(|start| start + self.tasks[task].duration)
    }

    fn compute_start(&self, task: usize) -> Option<i64> {
        let mut start = 0;
        if let Some(previous) = self.tasks[task].previous {
            start = start.max(self.end(previous)?);
        }
        if let Some(dependency) = self.tasks[task].waits_on {
            start = start.max(self.end(dependency)?);
        }
        Some(start)
    }
}

impl PlanningSolution for Shop {
    type Score = SoftScore;

    fn score(&self) -> Option<SoftScore> {
        self.score
    }

    fn set_score(&mut self, score: Option<SoftScore>) {
        self.score = score;
    }

    fn update_entity_shadows(&mut self, descriptor_index: usize, entity_index: usize) {
        self.update_list_structure_shadows(descriptor_index, entity_index);
        if descriptor_index == MACHINES {
            let changed = self.machines[entity_index].clone();
            CascadePropagation::new(self).propagate(self, changed, |_, _| {});
        }
    }

    fn update_all_shadows(&mut self) {
        for task in 0..self.tasks.len() {
            self.clear_list_shadows(task);
        }
        for machine in 0..self.machines.len() {
            self.update_list_structure_shadows(MACHINES, machine);
        }
        CascadePropagation::refresh_all(self);
    }

    fn list_shadow_elements(
        &self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Option<(usize, &[usize])> {
        (descriptor_index == MACHINES).then(|| (TASKS, self.machines[entity_index].as_slice()))
    }

    fn clear_list_shadows(&mut self, element: usize) {
        let task = &mut self.tasks[element];
        task.machine = None;
        task.previous = None;
        task.next = None;
    }

    const HAS_CASCADING_SHADOWS: bool = true;

    fn cascading_element_count(&self) -> usize {
        self.tasks.len()
    }

    fn cascading_assigned(&self, element: usize) -> bool {
        self.tasks[element].machine.is_some()
    }

    fn cascading_next(&self, element: usize) -> Option<usize> {
        self.tasks[element].next
    }

    fn cascading_dependencies(&self, element: usize, dependencies: &mut Vec<usize>) {
        dependencies.extend(self.tasks[element].waits_on);
    }

    fn list_structure_unchanged(
        &self,
        element: usize,
        owner_descriptor: usize,
        owner_index: usize,
        _index: usize,
        previous: Option<usize>,
        next: Option<usize>,
    ) -> bool {
        let task = &self.tasks[element];
        owner_descriptor == MACHINES
            && task.machine == Some(owner_index)
            && task.previous == previous
            && task.next == next
    }

    fn cascading_unset(&self, element: usize) -> bool {
        self.tasks[element].start.is_none()
    }

    fn update_cascading_shadow<W: FnOnce(&Self)>(
        &mut self,
        element: usize,
        computable: bool,
        before_write: W,
    ) -> bool {
        let start = if computable {
            self.compute_start(element)
        } else {
            None
        };
        if self.tasks[element].start == start {
            return false;
        }
        before_write(self);
        self.tasks[element].start = start;
        true
    }

    fn update_list_structure_shadows(&mut self, descriptor_index: usize, entity_index: usize) {
        if descriptor_index != MACHINES {
            return;
        }
        let list = &self.machines[entity_index];
        for (position, &task) in list.iter().enumerate() {
            let task = &mut self.tasks[task];
            task.machine = Some(entity_index);
            task.previous = position.checked_sub(1).map(|previous| list[previous]);
            task.next = list.get(position + 1).copied();
        }
    }
}

type StartCost = IncrementalUniConstraint<
    Shop,
    Task,
    SourceExtract<fn(&Shop) -> &[Task]>,
    fn(&Shop, &Task) -> bool,
    fn(&Task) -> SoftScore,
    SoftScore,
>;

thread_local! {
    // Rows weighed by `start_cost`, to count which rows a move rescores.
    static COST_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

// Each task costs its start, with its position and machine folded in so a
// stale previous pointer would also show. An unset start costs 1000 in no
// list and 5000 on a machine (a dependency cycle), so a row retracted after
// its machine was written would also show.
fn start_cost() -> StartCost {
    fn tasks(shop: &Shop) -> &[Task] {
        &shop.tasks
    }
    fn every(_: &Shop, _: &Task) -> bool {
        true
    }
    fn cost(task: &Task) -> SoftScore {
        COST_CALLS.with(|calls| calls.set(calls.get() + 1));
        match task.start {
            Some(start) => SoftScore::of(
                start * 10
                    + task.machine.map_or(0, |machine| machine as i64 + 1)
                    + task.previous.map_or(0, |previous| previous as i64 * 100),
            ),
            None if task.machine.is_some() => SoftScore::of(5000),
            None => SoftScore::of(1000),
        }
    }
    IncrementalUniConstraint::new(
        ConstraintRef::new("", "start cost"),
        ImpactType::Penalty,
        source(
            tasks as fn(&Shop) -> &[Task],
            ChangeSource::Descriptor(TASKS),
        ),
        every as fn(&Shop, &Task) -> bool,
        cost as fn(&Task) -> SoftScore,
        false,
    )
}

type Director = ScoreDirector<Shop, (StartCost,)>;

fn director(machines: Vec<Vec<usize>>, durations: &[i64], waits_on: &[(usize, usize)]) -> Director {
    let mut tasks: Vec<Task> = durations
        .iter()
        .map(|&duration| Task {
            duration,
            ..Task::default()
        })
        .collect();
    for &(task, dependency) in waits_on {
        tasks[task].waits_on = Some(dependency);
    }
    let mut director = ScoreDirector::new(
        Shop {
            machines,
            tasks,
            score: None,
        },
        (start_cost(),),
    );
    director.calculate_score();
    director
}

fn assert_matches_fresh(director: &mut Director) {
    let mut fresh = director.working_solution().clone();
    fresh.update_all_shadows();
    let starts = |shop: &Shop| shop.tasks.iter().map(|task| task.start).collect::<Vec<_>>();
    assert_eq!(
        starts(director.working_solution()),
        starts(&fresh),
        "machines {:?}",
        director.working_solution().machines
    );
    assert_eq!(
        director.calculate_score(),
        start_cost().evaluate(&fresh),
        "machines {:?}",
        director.working_solution().machines
    );
}

/// Rewrites the lists of `owners` through the variable-change protocol.
fn change(director: &mut Director, owners: &[usize], edit: impl FnOnce(&mut Vec<Vec<usize>>)) {
    for &owner in owners {
        director.before_variable_changed(MACHINES, owner);
    }
    edit(&mut director.working_solution_mut().machines);
    for &owner in owners {
        director.after_variable_changed(MACHINES, owner);
    }
}

#[test]
fn a_task_pushed_later_on_the_saw_retimes_its_milling_on_the_mill() {
    // Saw (0): cut 0. Mill (1): milling 1, which waits on the cut, then 2.
    // Task 3 is a long unassigned task.
    let mut director = director(vec![vec![0], vec![1, 2]], &[4, 3, 2, 10], &[(1, 0)]);
    assert_matches_fresh(&mut director);
    assert_eq!(director.working_solution().tasks[1].start, Some(4));

    change(&mut director, &[0], |machines| machines[0].insert(0, 3));
    assert_eq!(director.working_solution().tasks[1].start, Some(14));
    assert_eq!(director.working_solution().tasks[2].start, Some(17));
    assert_matches_fresh(&mut director);

    // Undo: the same path restores every start.
    change(&mut director, &[0], |machines| {
        machines[0].remove(0);
    });
    assert_eq!(director.working_solution().tasks[1].start, Some(4));
    assert_matches_fresh(&mut director);
}

#[test]
fn a_move_that_closes_a_dependency_cycle_unsets_it_and_its_undo_restores_it() {
    // Task 0 waits on task 2. Moving 2 after 0 on the saw closes a cycle.
    let mut director = director(vec![vec![0], vec![2, 1]], &[4, 3, 5], &[(0, 2), (1, 0)]);
    assert_matches_fresh(&mut director);

    change(&mut director, &[1, 0], |machines| {
        machines[1].retain(|&task| task != 2);
        machines[0].push(2);
    });
    let starts: Vec<_> = director
        .working_solution()
        .tasks
        .iter()
        .map(|task| task.start)
        .collect();
    assert_eq!(starts, vec![None, None, None]);
    assert_matches_fresh(&mut director);

    change(&mut director, &[0, 1], |machines| {
        machines[0].retain(|&task| task != 2);
        machines[1].insert(0, 2);
    });
    assert_matches_fresh(&mut director);
}

#[test]
fn a_task_appended_to_a_route_rescores_only_the_rows_whose_shadows_change() {
    // Saw (0): tasks 0 to 5. Task 6 is unassigned.
    let mut director = director(vec![(0..6).collect()], &[2, 3, 2, 4, 1, 3, 5], &[]);
    assert_matches_fresh(&mut director);
    COST_CALLS.with(|calls| calls.set(0));

    // Only task 5 (a new next) and task 6 (placed) change; tasks 0 to 4
    // keep their structure and start, so their rows stay.
    change(&mut director, &[0], |machines| machines[0].push(6));
    let rescored = COST_CALLS.with(|calls| calls.get());
    assert!(
        rescored <= 4,
        "{rescored} row weighings for two changed rows"
    );
    assert_matches_fresh(&mut director);
}

// A deterministic pseudo-random sequence (64-bit LCG).
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % bound
    }
}

#[test]
fn random_moves_and_undos_on_three_machines_keep_the_score_fresh() {
    let durations: Vec<i64> = (0..15).map(|task| (task % 4 + 1) as i64).collect();
    let waits: Vec<(usize, usize)> = (5..15)
        .map(|task| (task, (task * 3) % 5 + task / 5 - 1))
        .collect();
    let mut director = director(
        vec![(0..5).collect(), (5..10).collect(), (10..13).collect()],
        &durations,
        &waits,
    );
    assert_matches_fresh(&mut director);
    let mut random = Lcg(11);

    for _ in 0..300 {
        let before = director.working_solution().machines.clone();
        let from = random.below(3);
        let to = random.below(3);
        let owners: Vec<usize> = if from == to {
            vec![from]
        } else {
            vec![from, to]
        };
        let unassigned: Vec<usize> = (0..15)
            .filter(|&task| director.working_solution().tasks[task].machine.is_none())
            .collect();
        match random.below(3) {
            0 if !before[from].is_empty() => {
                let task = before[from][random.below(before[from].len())];
                let position = random.below(before[to].len() + 1);
                change(&mut director, &owners, |machines| {
                    machines[from].retain(|&held| held != task);
                    let position = position.min(machines[to].len());
                    machines[to].insert(position, task);
                });
            }
            1 if !before[from].is_empty() => {
                let position = random.below(before[from].len());
                change(&mut director, &[from], |machines| {
                    machines[from].remove(position);
                });
            }
            _ if !unassigned.is_empty() => {
                let task = unassigned[random.below(unassigned.len())];
                let position = random.below(before[to].len() + 1);
                change(&mut director, &[to], |machines| {
                    machines[to].insert(position, task)
                });
            }
            _ => continue,
        }
        assert_matches_fresh(&mut director);

        // Undo every other move by restoring the touched lists.
        if random.below(2) == 0 {
            change(&mut director, &[0, 1, 2], |machines| *machines = before);
            assert_matches_fresh(&mut director);
        }
    }
}
