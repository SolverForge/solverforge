// Cascading shadow propagation across list owners, checked against a full
// refresh: a two-machine shop whose tasks wait on tasks in the other
// machine's list.

use crate::domain::{CascadePropagation, PlanningSolution};
use crate::score::SoftScore;

#[derive(Clone, Debug, Default)]
struct Task {
    duration: i64,
    waits_on: Vec<usize>,
    machine: Option<usize>,
    previous: Option<usize>,
    next: Option<usize>,
    start: Option<i64>,
}

#[derive(Clone, Debug)]
struct Shop {
    machines: Vec<Vec<usize>>,
    tasks: Vec<Task>,
    computes: usize,
    score: Option<SoftScore>,
}

impl Shop {
    fn new(machines: Vec<Vec<usize>>, durations: &[i64], waits_on: &[(usize, usize)]) -> Self {
        let mut tasks: Vec<Task> = durations
            .iter()
            .map(|&duration| Task {
                duration,
                ..Task::default()
            })
            .collect();
        for &(task, dependency) in waits_on {
            tasks[task].waits_on.push(dependency);
        }
        let mut shop = Self {
            machines,
            tasks,
            computes: 0,
            score: None,
        };
        shop.refresh();
        shop
    }

    fn end(&self, task: usize) -> Option<i64> {
        self.tasks[task]
            .start
            .map(|start| start + self.tasks[task].duration)
    }

    // The later of the previous task's end and every dependency's end;
    // unset when any of them is unset.
    fn compute_start(&self, task: usize) -> Option<i64> {
        let mut start = 0;
        if let Some(previous) = self.tasks[task].previous {
            start = start.max(self.end(previous)?);
        }
        for &dependency in &self.tasks[task].waits_on {
            start = start.max(self.end(dependency)?);
        }
        Some(start)
    }

    fn refresh_structure(&mut self, machine: usize) {
        let list = self.machines[machine].clone();
        for (position, &task) in list.iter().enumerate() {
            let task = &mut self.tasks[task];
            task.machine = Some(machine);
            task.previous = position.checked_sub(1).map(|previous| list[previous]);
            task.next = list.get(position + 1).copied();
        }
    }

    fn refresh(&mut self) {
        for task in &mut self.tasks {
            task.machine = None;
            task.previous = None;
            task.next = None;
        }
        for machine in 0..self.machines.len() {
            self.refresh_structure(machine);
        }
        CascadePropagation::refresh_all(self);
    }

    fn starts(&self) -> Vec<Option<i64>> {
        self.tasks.iter().map(|task| task.start).collect()
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
        dependencies.extend_from_slice(&self.tasks[element].waits_on);
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
            self.computes += 1;
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
}

/// Applies a list change to `machines`, refreshes their structure, clears
/// tasks no list holds any more, and propagates from every task the changed
/// machines held before or after. Returns the elements written.
fn change(
    shop: &mut Shop,
    propagation: &mut CascadePropagation,
    machines: &[usize],
    edit: impl FnOnce(&mut Vec<Vec<usize>>),
) -> Vec<usize> {
    let mut changed: Vec<usize> = machines
        .iter()
        .flat_map(|&machine| shop.machines[machine].clone())
        .collect();
    edit(&mut shop.machines);
    for &machine in machines {
        changed.extend(shop.machines[machine].iter().copied());
    }
    changed.sort_unstable();
    changed.dedup();
    for &task in &changed {
        if !shop.machines.iter().any(|list| list.contains(&task)) {
            let task = &mut shop.tasks[task];
            task.machine = None;
            task.previous = None;
            task.next = None;
        }
    }
    for &machine in machines {
        shop.refresh_structure(machine);
    }
    let mut written = Vec::new();
    propagation.propagate(shop, changed, |_, element| written.push(element));
    written
}

fn assert_fresh(shop: &Shop) {
    let mut fresh = shop.clone();
    fresh.refresh();
    assert_eq!(
        shop.starts(),
        fresh.starts(),
        "machines {:?}",
        shop.machines
    );
}

// saw = machine 0, mill = machine 1. Task 1 (milling) waits on task 0
// (cutting).
fn saw_and_mill() -> Shop {
    Shop::new(vec![vec![0, 2], vec![1, 3]], &[4, 3, 5, 2], &[(1, 0)])
}

#[test]
fn a_full_refresh_follows_dependencies_across_lists() {
    let shop = saw_and_mill();
    // Saw: cut 0..4, task 2 4..9. Mill: milling waits on the cut, 4..7, task 3 7..9.
    assert_eq!(shop.starts(), vec![Some(0), Some(4), Some(4), Some(7)]);
}

#[test]
fn a_change_in_one_list_retimes_the_dependents_in_another() {
    let mut shop = saw_and_mill();
    let mut propagation = CascadePropagation::new(&shop);
    shop.tasks.push(Task {
        duration: 10,
        ..Task::default()
    });
    propagation.rebuild_dependencies(&shop);

    // A long task 4 goes before the cut: the cut, the milling and the task
    // after the milling all start later.
    let written = change(&mut shop, &mut propagation, &[0], |machines| {
        machines[0].insert(0, 4)
    });
    assert_eq!(
        shop.starts(),
        vec![Some(10), Some(14), Some(14), Some(17), Some(0)]
    );
    assert_fresh(&shop);
    let mut written = written;
    written.sort_unstable();
    assert_eq!(written, vec![0, 1, 2, 3, 4]);
}

#[test]
fn propagation_stops_where_a_value_does_not_change() {
    // Mill: task 3 (long) then the milling task 1, which waits on the cut 0
    // but is already held back by task 3.
    let mut shop = Shop::new(
        vec![vec![0, 2], vec![3, 1, 4]],
        &[4, 3, 5, 20, 2],
        &[(1, 0)],
    );
    let mut propagation = CascadePropagation::new(&shop);
    assert_eq!(shop.tasks[1].start, Some(20));

    // Swapping the saw's tasks moves the cut's end from 4 to 9, still before
    // 20: the milling task is recomputed, unchanged, and task 4 is not.
    shop.computes = 0;
    let written = change(&mut shop, &mut propagation, &[0], |machines| {
        machines[0].swap(0, 1)
    });
    assert_fresh(&shop);
    assert_eq!(written, vec![2, 0]);
    // Tasks 0 and 2 (changed) and task 1 (an input changed).
    assert_eq!(shop.computes, 3);
}

#[test]
fn a_dependency_cycle_is_unset_and_its_dependents_follow() {
    // Saw: 0 then 2. Task 0 waits on task 2, which comes after it.
    let shop = Shop::new(vec![vec![0, 2], vec![1]], &[4, 3, 5], &[(0, 2), (1, 2)]);
    assert_eq!(shop.starts(), vec![None, None, None]);

    // Breaking the cycle by moving task 2 to the mill's front gives every
    // task a start again.
    let mut shop = shop;
    let mut propagation = CascadePropagation::new(&shop);
    change(&mut shop, &mut propagation, &[0, 1], |machines| {
        machines[0].retain(|&task| task != 2);
        machines[1].insert(0, 2);
    });
    assert_eq!(shop.starts(), vec![Some(5), Some(5), Some(0)]);
    assert_fresh(&shop);

    // And creating it again unsets them.
    change(&mut shop, &mut propagation, &[0, 1], |machines| {
        machines[1].retain(|&task| task != 2);
        machines[0].push(2);
    });
    assert_eq!(shop.starts(), vec![None, None, None]);
    assert_fresh(&shop);
}

#[test]
fn an_element_that_depends_on_itself_is_unset() {
    let shop = Shop::new(vec![vec![0, 1]], &[4, 3], &[(0, 0)]);
    assert_eq!(shop.starts(), vec![None, None]);
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
fn random_moves_across_three_machines_match_a_full_refresh() {
    let durations: Vec<i64> = (0..18).map(|task| (task % 5 + 1) as i64).collect();
    // Each task after the first six waits on one or two earlier tasks.
    let waits: Vec<(usize, usize)> = (6..18)
        .flat_map(|task| [(task, task - 6), (task, (task * 7 + 3) % task)])
        .filter(|&(task, dependency)| task != dependency)
        .collect();
    let machines = vec![
        (0..6).collect::<Vec<_>>(),
        (6..12).collect(),
        (12..16).collect(),
    ];
    let mut shop = Shop::new(machines, &durations, &waits);
    let mut propagation = CascadePropagation::new(&shop);
    let mut random = Lcg(7);

    for step in 0..400 {
        let from = random.below(3);
        let to = random.below(3);
        let touched: Vec<usize> = if from == to {
            vec![from]
        } else {
            vec![from, to]
        };
        let written_before = shop.computes;
        match random.below(3) {
            // Move a task between (or within) machines.
            0 if !shop.machines[from].is_empty() => {
                let task = shop.machines[from][random.below(shop.machines[from].len())];
                let position = random.below(shop.machines[to].len() + 1);
                change(&mut shop, &mut propagation, &touched, |machines| {
                    machines[from].retain(|&held| held != task);
                    let position = position.min(machines[to].len());
                    machines[to].insert(position, task);
                });
            }
            // Unassign a task.
            1 if !shop.machines[from].is_empty() => {
                let position = random.below(shop.machines[from].len());
                change(&mut shop, &mut propagation, &[from], |machines| {
                    machines[from].remove(position);
                });
            }
            // Assign an unassigned task.
            _ => {
                let unassigned: Vec<usize> = (0..shop.tasks.len())
                    .filter(|&task| shop.tasks[task].machine.is_none())
                    .collect();
                if let Some(&task) = unassigned.get(random.below(unassigned.len().max(1))) {
                    let position = random.below(shop.machines[to].len() + 1);
                    change(&mut shop, &mut propagation, &[to], |machines| {
                        machines[to].insert(position, task)
                    });
                }
            }
        }
        assert!(
            shop.computes - written_before <= shop.tasks.len(),
            "step {step}: an element was computed more than once"
        );
        assert_fresh(&shop);
    }
}
