// Tests for ListChangeMove operations.

use super::*;

#[derive(Clone, Debug)]
struct Vehicle {
    visits: Vec<i32>,
}

#[derive(Clone, Debug)]
struct RoutingSolution {
    vehicles: Vec<Vehicle>,
    score: Option<SoftScore>,
}

impl PlanningSolution for RoutingSolution {
    type Score = SoftScore;
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
        .and_then(|v| v.visits.get(pos))
        .copied()
}
fn list_remove(s: &mut RoutingSolution, entity_idx: usize, pos: usize) -> Option<i32> {
    s.vehicles.get_mut(entity_idx).map(|v| v.visits.remove(pos))
}
fn list_insert(s: &mut RoutingSolution, entity_idx: usize, pos: usize, val: i32) {
    if let Some(v) = s.vehicles.get_mut(entity_idx) {
        v.visits.insert(pos, val);
    }
}

fn create_director(vehicles: Vec<Vehicle>) -> ScoreDirector<RoutingSolution, ()> {
    let solution = RoutingSolution {
        vehicles,
        score: None,
    };
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Vehicle",
        "vehicles",
        get_vehicles,
        get_vehicles_mut,
    ));
    let entity_desc = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
        .with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("RoutingSolution", TypeId::of::<RoutingSolution>())
        .with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.vehicles.len())
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
        list_get,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    assert!(m.is_doable(&director));

    {
        let mut recording = SnapshotDirector::new(&mut director);
        m.do_move(&mut recording);

        let visits = &director.working_solution().vehicles[0].visits;
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
        list_get,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    assert!(m.is_doable(&director));

    {
        let mut recording = SnapshotDirector::new(&mut director);
        m.do_move(&mut recording);

        let visits = &director.working_solution().vehicles[0].visits;
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
        list_get,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    assert!(m.is_doable(&director));

    {
        let mut recording = SnapshotDirector::new(&mut director);
        m.do_move(&mut recording);

        let sol = director.working_solution();
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
        list_get,
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
        list_get,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    assert!(!m.is_doable(&director));
}

// T13: list-state consistency through the mutation lifecycle. The solution
// maintains an inverse element->(entity,position) supply; every list change
// through the kernel must leave the supply exact after do and after undo.
#[derive(Clone, Debug)]
struct ShadowVehicle {
    visits: Vec<i32>,
}

#[derive(Clone, Debug)]
struct ShadowRoutingSolution {
    vehicles: Vec<ShadowVehicle>,
    inverse: Vec<Option<(usize, usize)>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for ShadowRoutingSolution {
    type Score = SoftScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
    fn update_all_shadows(&mut self) {
        for entity_index in 0..self.vehicles.len() {
            ShadowRoutingSolution::update_entity_shadows(self, 0, entity_index);
        }
    }
    fn update_entity_shadows(&mut self, _descriptor_index: usize, entity_index: usize) {
        // element ids are global (0..n inverse slots); clear this entity's
        // claims, then re-record positions.
        for position in self.inverse.iter_mut() {
            if let Some((owner, _)) = *position {
                if owner == entity_index {
                    *position = None;
                }
            }
        }
        if let Some(vehicle) = self.vehicles.get(entity_index) {
            for (position, element) in vehicle.visits.iter().copied().enumerate() {
                if let Some(slot) = self.inverse.get_mut(element as usize) {
                    *slot = Some((entity_index, position));
                }
            }
        }
    }
}

impl ShadowRoutingSolution {
    fn is_consistent(&self) -> bool {
        let mut replayed = vec![None; self.inverse.len()];
        for (entity_index, vehicle) in self.vehicles.iter().enumerate() {
            for (position, element) in vehicle.visits.iter().copied().enumerate() {
                let slot = &mut replayed[element as usize];
                if slot.is_some() {
                    return false; // element recorded twice
                }
                *slot = Some((entity_index, position));
            }
        }
        replayed == self.inverse
    }
}

fn shadow_list_len(s: &ShadowRoutingSolution, entity_idx: usize) -> usize {
    s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
}
fn shadow_list_get(s: &ShadowRoutingSolution, entity_idx: usize, pos: usize) -> Option<i32> {
    s.vehicles
        .get(entity_idx)
        .and_then(|v| v.visits.get(pos))
        .copied()
}
fn shadow_list_remove(s: &mut ShadowRoutingSolution, entity_idx: usize, pos: usize) -> Option<i32> {
    let removed = s.vehicles.get_mut(entity_idx).and_then(|v| {
        if pos < v.visits.len() {
            Some(v.visits.remove(pos))
        } else {
            None
        }
    });
    if removed.is_some() {
        ShadowRoutingSolution::update_entity_shadows(s, 0, entity_idx);
    }
    removed
}
fn shadow_list_insert(s: &mut ShadowRoutingSolution, entity_idx: usize, pos: usize, val: i32) {
    if let Some(v) = s.vehicles.get_mut(entity_idx) {
        v.visits.insert(pos, val);
    }
    ShadowRoutingSolution::update_entity_shadows(s, 0, entity_idx);
}

fn create_shadow_director(
    vehicle_visits: Vec<Vec<i32>>,
) -> ScoreDirector<ShadowRoutingSolution, ()> {
    let element_count = vehicle_visits
        .iter()
        .flatten()
        .copied()
        .max()
        .map_or(0, |max| max + 1) as usize;
    let solution = ShadowRoutingSolution {
        vehicles: vehicle_visits
            .into_iter()
            .map(|visits| ShadowVehicle { visits })
            .collect(),
        inverse: vec![None; element_count],
        score: None,
    };
    let mut solution = solution;
    PlanningSolution::update_all_shadows(&mut solution);
    let extractor = Box::new(EntityCollectionExtractor::new(
        "ShadowVehicle",
        "visits",
        get_shadow_vehicles,
        get_shadow_vehicles_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("ShadowVehicle", TypeId::of::<ShadowVehicle>(), "visits")
            .with_extractor(extractor);
    let descriptor = SolutionDescriptor::new(
        "ShadowRoutingSolution",
        TypeId::of::<ShadowRoutingSolution>(),
    )
    .with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.vehicles.len())
}

fn get_shadow_vehicles(s: &ShadowRoutingSolution) -> &Vec<ShadowVehicle> {
    &s.vehicles
}
fn get_shadow_vehicles_mut(s: &mut ShadowRoutingSolution) -> &mut Vec<ShadowVehicle> {
    &mut s.vehicles
}

#[test]
fn list_change_keeps_inverse_supply_exact_through_do_and_undo() {
    let mut director = create_shadow_director(vec![vec![0, 1, 2], vec![3]]);
    let base = director.working_solution().clone();
    assert!(base.is_consistent());

    let m = ListChangeMove::<ShadowRoutingSolution, i32>::new(
        1,
        0,
        0,
        1,
        shadow_list_len,
        shadow_list_get,
        shadow_list_remove,
        shadow_list_insert,
        "visits",
        0,
    );

    assert!(m.is_doable(&director));
    {
        let mut recording = SnapshotDirector::new(&mut director);
        m.do_move(&mut recording);

        let solution = director.working_solution();
        assert!(solution.is_consistent(), "supply exact after do");
        assert_eq!(solution.inverse[3], Some((0, 1)));
        assert_eq!(solution.vehicles[1].visits, Vec::<i32>::new());
        assert_eq!(solution.vehicles[0].visits, vec![0, 3, 1, 2]);

        recording.undo_changes();
    }

    let restored = director.working_solution();
    assert!(
        restored.is_consistent(),
        "supply exact after undo through the recording snapshot restore"
    );
    assert_eq!(restored.vehicles[0].visits, vec![0, 1, 2]);
    assert_eq!(restored.vehicles[1].visits, vec![3]);
    assert_eq!(restored.inverse[3], Some((1, 0)));
}
