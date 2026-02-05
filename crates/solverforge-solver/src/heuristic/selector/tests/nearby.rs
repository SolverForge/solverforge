//! Tests for nearby entity selector.

use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::mimic::{MimicRecorder, MimicRecordingEntitySelector};
use crate::heuristic::selector::nearby::{
    DynDistanceMeter, NearbyEntitySelector, NearbySelectionConfig,
};
use crate::heuristic::selector::{EntityReference, EntitySelector};
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Location {
    id: i64,
    x: f64,
    y: f64,
}

#[derive(Clone, Debug)]
struct RoutingSolution {
    locations: Vec<Location>,
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

fn get_locations(s: &RoutingSolution) -> &Vec<Location> {
    &s.locations
}

fn get_locations_mut(s: &mut RoutingSolution) -> &mut Vec<Location> {
    &mut s.locations
}

/// Distance meter that uses Euclidean distance.
#[derive(Debug)]
struct EuclideanDistanceMeter {
    /// Cached locations for quick lookup.
    locations: Vec<(f64, f64)>,
}

impl EuclideanDistanceMeter {
    fn new(locations: &[Location]) -> Self {
        Self {
            locations: locations.iter().map(|l| (l.x, l.y)).collect(),
        }
    }
}

impl DynDistanceMeter for EuclideanDistanceMeter {
    fn distance_between<S: PlanningSolution>(
        &self,
        _score_director: &dyn ScoreDirector<S>,
        origin: EntityReference,
        destination: EntityReference,
    ) -> f64 {
        let (ox, oy) = self.locations[origin.entity_index];
        let (dx, dy) = self.locations[destination.entity_index];
        let delta_x = ox - dx;
        let delta_y = oy - dy;
        (delta_x * delta_x + delta_y * delta_y).sqrt()
    }
}

fn create_test_director(
) -> SimpleScoreDirector<RoutingSolution, impl Fn(&RoutingSolution) -> SimpleScore> {
    // Create a grid of locations: (0,0), (1,0), (2,0), (0,1), (1,1), (2,1)
    let locations = vec![
        Location {
            id: 0,
            x: 0.0,
            y: 0.0,
        },
        Location {
            id: 1,
            x: 1.0,
            y: 0.0,
        },
        Location {
            id: 2,
            x: 2.0,
            y: 0.0,
        },
        Location {
            id: 3,
            x: 0.0,
            y: 1.0,
        },
        Location {
            id: 4,
            x: 1.0,
            y: 1.0,
        },
        Location {
            id: 5,
            x: 2.0,
            y: 1.0,
        },
    ];

    let solution = RoutingSolution {
        locations,
        score: None,
    };

    let extractor = Box::new(TypedEntityExtractor::new(
        "Location",
        "locations",
        get_locations,
        get_locations_mut,
    ));
    let entity_desc = EntityDescriptor::new("Location", TypeId::of::<Location>(), "locations")
        .with_extractor(extractor);

    let descriptor = SolutionDescriptor::new("RoutingSolution", TypeId::of::<RoutingSolution>())
        .with_entity(entity_desc);

    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn test_nearby_selector_sorts_by_distance() {
    let director = create_test_director();

    // Verify entity IDs match indices
    let solution = director.working_solution();
    for (i, loc) in solution.locations.iter().enumerate() {
        assert_eq!(loc.id, i as i64);
    }

    // Create mimic recorder and recording selector for origin
    let recorder = MimicRecorder::new("origin");
    let origin_child = FromSolutionEntitySelector::new(0);
    let origin_selector = MimicRecordingEntitySelector::new(origin_child, recorder.clone());

    // Create nearby selector for destinations
    let dest_child = FromSolutionEntitySelector::new(0);
    let distance_meter = EuclideanDistanceMeter::new(&director.working_solution().locations);
    let nearby_config = NearbySelectionConfig::default();
    let nearby_selector =
        NearbyEntitySelector::new(dest_child, recorder.clone(), distance_meter, nearby_config);

    // Select origin entity (location 0 at 0,0)
    let mut origin_iter = origin_selector.iter(&director);
    let _origin = origin_iter.next().unwrap();

    // Get nearby entities (should be sorted by distance from 0,0)
    let nearby: Vec<_> = nearby_selector.iter(&director).collect();

    // Expected order: 1 (dist 1), 3 (dist 1), 2 (dist 2), 4 (dist √2 ≈ 1.41), 5 (dist √5 ≈ 2.24)
    // Actually: 1 at (1,0) dist 1, 3 at (0,1) dist 1, 4 at (1,1) dist √2, 2 at (2,0) dist 2, 5 at (2,1) dist √5
    assert_eq!(nearby.len(), 5); // 6 locations - 1 (origin) = 5

    // First two should be at distance 1 (locations 1 and 3)
    assert!(
        nearby[0].entity_index == 1 || nearby[0].entity_index == 3,
        "Expected location 1 or 3, got {}",
        nearby[0].entity_index
    );
}

#[test]
fn test_nearby_selector_with_max_size() {
    let director = create_test_director();

    let recorder = MimicRecorder::new("origin");
    let origin_child = FromSolutionEntitySelector::new(0);
    let origin_selector = MimicRecordingEntitySelector::new(origin_child, recorder.clone());

    let dest_child = FromSolutionEntitySelector::new(0);
    let distance_meter = EuclideanDistanceMeter::new(&director.working_solution().locations);
    let nearby_config = NearbySelectionConfig::default().with_max_nearby_size(2);
    let nearby_selector =
        NearbyEntitySelector::new(dest_child, recorder.clone(), distance_meter, nearby_config);

    // Select origin
    let mut origin_iter = origin_selector.iter(&director);
    origin_iter.next();

    // Should only get 2 nearest
    let nearby: Vec<_> = nearby_selector.iter(&director).collect();
    assert_eq!(nearby.len(), 2);
}

#[test]
fn test_nearby_selector_excludes_origin() {
    let director = create_test_director();

    let recorder = MimicRecorder::new("origin");
    let origin_child = FromSolutionEntitySelector::new(0);
    let origin_selector = MimicRecordingEntitySelector::new(origin_child, recorder.clone());

    let dest_child = FromSolutionEntitySelector::new(0);
    let distance_meter = EuclideanDistanceMeter::new(&director.working_solution().locations);
    let nearby_config = NearbySelectionConfig::default();
    let nearby_selector =
        NearbyEntitySelector::new(dest_child, recorder.clone(), distance_meter, nearby_config);

    // Select origin (entity 0)
    let mut origin_iter = origin_selector.iter(&director);
    let origin = origin_iter.next().unwrap();

    // Nearby should not include the origin
    let nearby: Vec<_> = nearby_selector.iter(&director).collect();
    assert!(!nearby.contains(&origin));
}
