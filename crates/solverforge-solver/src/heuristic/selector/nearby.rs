//! Nearby selection for distance-based filtering of candidates.
//!
//! Nearby selection improves move quality by preferring destinations that are
//! geographically or otherwise "close" to an origin. This is critical for
//! vehicle routing problems (VRP) where swapping with nearby customers
//! is more likely to improve the solution than swapping with distant ones.
//!
//! # Architecture
//!
//! - [`NearbyDistanceMeter`]: User-defined function to measure distance between elements
//! - [`NearbySelectionConfig`]: Configuration for nearby selection behavior
//! - [`NearbyEntitySelector`]: Selects entities nearby to a reference entity

use std::cmp::Ordering;
use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::entity::{EntityReference, EntitySelector};
use super::mimic::MimicRecorder;

/// Trait for measuring distance between an origin and a destination.
///
/// Implementations should be stateless. The solver may reuse instances.
///
/// # Type Parameters
///
/// - `Origin`: The type of the origin element (usually an entity or value)
/// - `Destination`: The type of the destination element (usually an entity or value)
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::NearbyDistanceMeter;
///
/// #[derive(Debug)]
/// struct Location { x: f64, y: f64 }
///
/// #[derive(Debug)]
/// struct EuclideanMeter;
///
/// impl NearbyDistanceMeter<Location, Location> for EuclideanMeter {
///     fn distance(&self, origin: &Location, dest: &Location) -> f64 {
///         let dx = origin.x - dest.x;
///         let dy = origin.y - dest.y;
///         (dx * dx + dy * dy).sqrt()
///     }
/// }
/// ```
pub trait NearbyDistanceMeter<Origin, Destination>: Send + Sync + Debug {
    /// Measures the distance from the origin to the destination.
    ///
    /// The distance can be in any unit (meters, seconds, cost, etc.).
    /// Distances can be asymmetrical: the distance from A to B may differ
    /// from the distance from B to A.
    ///
    /// Returns a value >= 0.0. If origin == destination, typically returns 0.0.
    fn distance(&self, origin: &Origin, destination: &Destination) -> f64;
}

/// Distribution type for nearby selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NearbyDistributionType {
    /// Select all candidates sorted by distance (up to a maximum).
    #[default]
    Linear,
    /// Select candidates with probability proportional to distance (closer = more likely).
    Parabolic,
    /// Use a block distribution for k-opt style moves.
    Block,
}

/// Configuration for nearby selection.
#[derive(Debug, Clone)]
pub struct NearbySelectionConfig {
    /// The distribution type to use.
    pub distribution_type: NearbyDistributionType,
    /// Maximum number of nearby candidates to consider.
    /// If None, considers all candidates (sorted by distance).
    pub max_nearby_size: Option<usize>,
    /// Minimum distance to include a candidate (exclusive of origin).
    pub min_distance: f64,
}

impl Default for NearbySelectionConfig {
    fn default() -> Self {
        Self {
            distribution_type: NearbyDistributionType::Linear,
            max_nearby_size: None,
            min_distance: 0.0,
        }
    }
}

impl NearbySelectionConfig {
    /// Creates a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the distribution type.
    pub fn with_distribution_type(mut self, distribution_type: NearbyDistributionType) -> Self {
        self.distribution_type = distribution_type;
        self
    }

    /// Sets the maximum number of nearby candidates.
    pub fn with_max_nearby_size(mut self, max_size: usize) -> Self {
        self.max_nearby_size = Some(max_size);
        self
    }

    /// Sets the minimum distance threshold.
    pub fn with_min_distance(mut self, min_distance: f64) -> Self {
        self.min_distance = min_distance;
        self
    }
}

/// Type-erased distance meter for dynamic dispatch.
pub trait DynDistanceMeter: Send + Sync + Debug {
    /// Measures distance between two entity references.
    fn distance_between<S: PlanningSolution>(
        &self,
        score_director: &dyn ScoreDirector<S>,
        origin: EntityReference,
        destination: EntityReference,
    ) -> f64;
}

/// An entity selector that returns entities nearby to an origin entity.
///
/// The origin entity is obtained from a mimic recorder, allowing this selector
/// to be synchronized with another selector that picks the "current" entity.
///
/// # Zero-Erasure Design
///
/// The child entity selector `ES` is stored as a concrete generic type parameter,
/// eliminating virtual dispatch overhead when iterating over candidate entities.
pub struct NearbyEntitySelector<S, M, ES> {
    /// The child selector providing all candidate entities (zero-erasure).
    child: ES,
    /// The recorder providing the origin entity.
    origin_recorder: MimicRecorder,
    /// The distance meter for measuring nearness.
    distance_meter: M,
    /// Configuration for nearby selection.
    config: NearbySelectionConfig,
    /// Marker for solution type.
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S, M, ES> NearbyEntitySelector<S, M, ES> {
    /// Creates a new nearby entity selector.
    pub fn new(
        child: ES,
        origin_recorder: MimicRecorder,
        distance_meter: M,
        config: NearbySelectionConfig,
    ) -> Self {
        Self {
            child,
            origin_recorder,
            distance_meter,
            config,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: PlanningSolution, M: DynDistanceMeter, ES: Debug> Debug for NearbyEntitySelector<S, M, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NearbyEntitySelector")
            .field("child", &self.child)
            .field("origin_recorder_id", &self.origin_recorder.id())
            .field("distance_meter", &self.distance_meter)
            .field("config", &self.config)
            .finish()
    }
}

impl<S, M, ES> EntitySelector<S> for NearbyEntitySelector<S, M, ES>
where
    S: PlanningSolution,
    M: DynDistanceMeter + 'static,
    ES: EntitySelector<S>,
{
    fn iter<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a> {
        // Get the origin entity from the recorder
        let origin = match self.origin_recorder.get_recorded_entity() {
            Some(e) => e,
            None => {
                // No origin recorded yet, return empty
                return Box::new(std::iter::empty());
            }
        };

        // Collect all candidate entities with their distances
        let mut candidates: Vec<(EntityReference, f64)> = self
            .child
            .iter(score_director)
            .filter(|&dest| dest != origin) // Exclude the origin itself
            .map(|dest| {
                let dist = self
                    .distance_meter
                    .distance_between(score_director, origin, dest);
                (dest, dist)
            })
            .filter(|(_, dist)| *dist >= self.config.min_distance)
            .collect();

        // Sort by distance (closest first)
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

        // Apply max size limit
        if let Some(max_size) = self.config.max_nearby_size {
            candidates.truncate(max_size);
        }

        Box::new(candidates.into_iter().map(|(entity, _)| entity))
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        // This is an estimate; the actual size depends on the origin
        let child_size = self.child.size(score_director);
        match self.config.max_nearby_size {
            Some(max) => child_size.min(max),
            None => child_size,
        }
    }
}
