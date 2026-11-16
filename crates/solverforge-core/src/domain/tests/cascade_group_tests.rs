// Joint groups in cascading shadow propagation: visits that two or three
// technicians start together, each in its own technician's list. Checked
// against an independent fixed point of the per-visit compute.

use crate::domain::{CascadePropagation, PlanningSolution};
use crate::score::SoftScore;

#[derive(Clone, Debug, Default)]
struct Visit {
    duration: i64,
    waits_on: Vec<usize>,
    peers: Vec<usize>,
    technician: Option<usize>,
    previous: Option<usize>,
    next: Option<usize>,
    start: Option<i64>,
}

#[derive(Clone, Debug)]
struct Crew {
    technicians: Vec<Vec<usize>>,
    visits: Vec<Visit>,
    computes: usize,
    score: Option<SoftScore>,
}

impl Crew {
    fn new(
        technicians: Vec<Vec<usize>>,
        durations: &[i64],
        groups: &[&[usize]],
        waits_on: &[(usize, usize)],
    ) -> Self {
        let mut visits: Vec<Visit> = durations
            .iter()
            .map(|&duration| Visit {
                duration,
                ..Visit::default()
            })
            .collect();
        for group in groups {
            for &member in group.iter() {
                visits[member].peers = group.iter().copied().filter(|&p| p != member).collect();
            }
        }
        for &(visit, dependency) in waits_on {
            visits[visit].waits_on.push(dependency);
        }
        let mut crew = Self {
            technicians,
            visits,
            computes: 0,
            score: None,
        };
        crew.refresh_structure_all();
        CascadePropagation::refresh_all(&mut crew);
        crew
    }

    fn end(&self, visit: usize) -> Option<i64> {
        self.visits[visit]
            .start
            .map(|start| start + self.visits[visit].duration)
    }

    // When the visit's technician can be there: after the previous visit,
    // one unit of travel, and every visit it waits on.
    fn arrival(&self, visit: usize) -> Option<i64> {
        let mut arrival = 0;
        if let Some(previous) = self.visits[visit].previous {
            arrival = arrival.max(self.end(previous)? + 1);
        }
        for &dependency in &self.visits[visit].waits_on {
            arrival = arrival.max(self.end(dependency)?);
        }
        Some(arrival)
    }

    // A visit starts when the last assigned technician of its group arrives.
    // It reads its peers' inputs, never their starts.
    fn compute_start(&self, visit: usize) -> Option<i64> {
        let mut start = self.arrival(visit)?;
        for &peer in &self.visits[visit].peers {
            if self.visits[peer].technician.is_some() {
                start = start.max(self.arrival(peer)?);
            }
        }
        Some(start)
    }

    fn refresh_structure(&mut self, technician: usize) {
        let list = self.technicians[technician].clone();
        for (position, &visit) in list.iter().enumerate() {
            let visit = &mut self.visits[visit];
            visit.technician = Some(technician);
            visit.previous = position.checked_sub(1).map(|previous| list[previous]);
            visit.next = list.get(position + 1).copied();
        }
    }

    fn refresh_structure_all(&mut self) {
        for visit in &mut self.visits {
            visit.technician = None;
            visit.previous = None;
            visit.next = None;
        }
        for technician in 0..self.technicians.len() {
            self.refresh_structure(technician);
        }
    }

    fn starts(&self) -> Vec<Option<i64>> {
        self.visits.iter().map(|visit| visit.start).collect()
    }

    // The least fixed point of the compute from all-unset values: what the
    // propagation must produce, found without it. A visit on a cycle never
    // gets a value.
    fn fixed_point(&self) -> Vec<Option<i64>> {
        let mut crew = self.clone();
        crew.refresh_structure_all();
        for visit in &mut crew.visits {
            visit.start = None;
        }
        loop {
            let starts: Vec<Option<i64>> = (0..crew.visits.len())
                .map(|visit| {
                    crew.visits[visit]
                        .technician
                        .and_then(|_| crew.compute_start(visit))
                })
                .collect();
            if starts == crew.starts() {
                return starts;
            }
            for (visit, start) in starts.into_iter().enumerate() {
                crew.visits[visit].start = start;
            }
        }
    }
}

impl PlanningSolution for Crew {
    type Score = SoftScore;

    fn score(&self) -> Option<SoftScore> {
        self.score
    }

    fn set_score(&mut self, score: Option<SoftScore>) {
        self.score = score;
    }

    const HAS_CASCADING_SHADOWS: bool = true;

    fn cascading_element_count(&self) -> usize {
        self.visits.len()
    }

    fn cascading_assigned(&self, element: usize) -> bool {
        self.visits[element].technician.is_some()
    }

    fn cascading_next(&self, element: usize) -> Option<usize> {
        self.visits[element].next
    }

    fn cascading_dependencies(&self, element: usize, dependencies: &mut Vec<usize>) {
        dependencies.extend_from_slice(&self.visits[element].waits_on);
    }

    fn cascading_peers(&self, element: usize, peers: &mut Vec<usize>) {
        peers.extend_from_slice(&self.visits[element].peers);
    }

    fn cascading_unset(&self, element: usize) -> bool {
        self.visits[element].start.is_none()
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
        if self.visits[element].start == start {
            return false;
        }
        before_write(self);
        self.visits[element].start = start;
        true
    }
}

/// Applies a list change to `technicians`, refreshes their structure,
/// clears visits no list holds any more, and propagates from every visit the
/// changed technicians held before or after. Returns the elements written.
fn change(
    crew: &mut Crew,
    propagation: &mut CascadePropagation,
    technicians: &[usize],
    edit: impl FnOnce(&mut Vec<Vec<usize>>),
) -> Vec<usize> {
    let mut changed: Vec<usize> = technicians
        .iter()
        .flat_map(|&technician| crew.technicians[technician].clone())
        .collect();
    edit(&mut crew.technicians);
    for &technician in technicians {
        changed.extend(crew.technicians[technician].iter().copied());
    }
    changed.sort_unstable();
    changed.dedup();
    for &visit in &changed {
        if !crew.technicians.iter().any(|list| list.contains(&visit)) {
            let visit = &mut crew.visits[visit];
            visit.technician = None;
            visit.previous = None;
            visit.next = None;
        }
    }
    for &technician in technicians {
        crew.refresh_structure(technician);
    }
    let mut written = Vec::new();
    propagation.propagate(crew, changed, |_, element| written.push(element));
    written
}

fn assert_fixed_point(crew: &Crew) {
    assert_eq!(
        crew.starts(),
        crew.fixed_point(),
        "technicians {:?}",
        crew.technicians
    );
}

// Ann: visit 0 (4 long), then the joint visit 2. Bob: the joint visit 3,
// then visit 1. Visits 2 and 3 start together.
fn ann_and_bob() -> Crew {
    Crew::new(vec![vec![0, 2], vec![3, 1]], &[4, 3, 6, 6], &[&[2, 3]], &[])
}

#[test]
fn a_joint_visit_starts_when_the_last_technician_arrives() {
    let crew = ann_and_bob();
    // Ann arrives at 5, Bob at 0: both start at 5 and Bob's next visit
    // follows at 12.
    assert_eq!(crew.starts(), vec![Some(0), Some(12), Some(5), Some(5)]);
    assert_fixed_point(&crew);
}

#[test]
fn a_move_on_one_route_retimes_the_partner_and_the_partners_route() {
    let mut crew = ann_and_bob();
    let mut propagation = CascadePropagation::new(&crew);
    // Ann goes to the joint visit first: both start at 0 and Bob's next
    // visit at 7.
    let mut written = change(&mut crew, &mut propagation, &[0], |technicians| {
        technicians[0].swap(0, 1)
    });
    assert_eq!(crew.starts(), vec![Some(7), Some(7), Some(0), Some(0)]);
    assert_fixed_point(&crew);
    written.sort_unstable();
    assert_eq!(written, vec![0, 1, 2, 3]);
}

#[test]
fn an_unchanged_group_stops_propagation() {
    // Ann's first visit is already late enough to hold the group back.
    let mut crew = Crew::new(
        vec![vec![0, 4, 2], vec![3, 1]],
        &[4, 3, 6, 6, 1],
        &[&[2, 3]],
        &[],
    );
    let mut propagation = CascadePropagation::new(&crew);
    assert_eq!(crew.visits[2].start, Some(7));
    // Swapping Ann's first two visits leaves her arrival at the joint
    // visit at 7: the group is recomputed, unchanged, and Bob's next visit
    // is not.
    crew.computes = 0;
    let mut written = change(&mut crew, &mut propagation, &[0], |technicians| {
        technicians[0].swap(0, 1)
    });
    assert_fixed_point(&crew);
    written.sort_unstable();
    assert_eq!(written, vec![0, 4]);
    // Ann's three visits and the partner visit 3.
    assert_eq!(crew.computes, 4);
}

#[test]
fn a_group_on_one_route_is_a_cycle_and_is_unset() {
    // Both members on Ann's route: the second waits on the first's end and
    // the first on the second's arrival.
    let crew = Crew::new(vec![vec![0, 2, 3, 1]], &[4, 3, 6, 6], &[&[2, 3]], &[]);
    assert_eq!(crew.starts(), vec![Some(0), None, None, None]);
    assert_fixed_point(&crew);

    let mut crew = crew;
    crew.technicians.push(Vec::new());
    let mut propagation = CascadePropagation::new(&crew);
    change(&mut crew, &mut propagation, &[0, 1], |technicians| {
        technicians[0].retain(|&visit| visit != 3);
        technicians[1].push(3);
    });
    assert_eq!(crew.starts(), vec![Some(0), Some(12), Some(5), Some(5)]);
    assert_fixed_point(&crew);
}

#[test]
fn a_member_left_unassigned_frees_its_partner() {
    let mut crew = ann_and_bob();
    let mut propagation = CascadePropagation::new(&crew);
    change(&mut crew, &mut propagation, &[0], |technicians| {
        technicians[0].retain(|&visit| visit != 2)
    });
    assert_eq!(crew.starts(), vec![Some(0), Some(7), None, Some(0)]);
    assert_fixed_point(&crew);
}

#[test]
fn propagation_without_peers_keeps_every_element_its_own_node() {
    let crew = Crew::new(vec![vec![0, 1], vec![2]], &[4, 3, 2], &[], &[(2, 1)]);
    assert_eq!(crew.starts(), vec![Some(0), Some(5), Some(8)]);
    assert_fixed_point(&crew);
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
fn random_moves_with_groups_and_dependencies_match_the_fixed_point() {
    let durations: Vec<i64> = (0..24).map(|visit| (visit % 4 + 1) as i64).collect();
    // Pairs and a triple across routes, plus visits that wait on others.
    let groups: &[&[usize]] = &[&[0, 8], &[3, 11, 19], &[5, 14], &[9, 20]];
    let waits = [(16, 2), (12, 0), (21, 3), (6, 17), (10, 5)];
    let technicians = vec![
        (0..6).collect::<Vec<_>>(),
        (6..12).collect(),
        (12..18).collect(),
        (18..22).collect(),
    ];
    for seed in [7, 21, 1234] {
        let mut crew = Crew::new(technicians.clone(), &durations, groups, &waits);
        assert_fixed_point(&crew);
        let mut propagation = CascadePropagation::new(&crew);
        let mut random = Lcg(seed);
        for step in 0..500 {
            let from = random.below(4);
            let to = random.below(4);
            let touched: Vec<usize> = if from == to {
                vec![from]
            } else {
                vec![from, to]
            };
            let computes_before = crew.computes;
            match random.below(3) {
                0 if !crew.technicians[from].is_empty() => {
                    let visit = crew.technicians[from][random.below(crew.technicians[from].len())];
                    let position = random.below(crew.technicians[to].len() + 1);
                    change(&mut crew, &mut propagation, &touched, |technicians| {
                        technicians[from].retain(|&held| held != visit);
                        let position = position.min(technicians[to].len());
                        technicians[to].insert(position, visit);
                    });
                }
                1 if !crew.technicians[from].is_empty() => {
                    let position = random.below(crew.technicians[from].len());
                    change(&mut crew, &mut propagation, &[from], |technicians| {
                        technicians[from].remove(position);
                    });
                }
                _ => {
                    let unassigned: Vec<usize> = (0..crew.visits.len())
                        .filter(|&visit| crew.visits[visit].technician.is_none())
                        .collect();
                    if let Some(&visit) = unassigned.get(random.below(unassigned.len().max(1))) {
                        let position = random.below(crew.technicians[to].len() + 1);
                        change(&mut crew, &mut propagation, &[to], |technicians| {
                            technicians[to].insert(position, visit)
                        });
                    }
                }
            }
            assert!(
                crew.computes - computes_before <= crew.visits.len(),
                "seed {seed} step {step}: an element was computed more than once"
            );
            assert_fixed_point(&crew);
        }
    }
}
