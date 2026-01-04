//! Domain model for Vehicle Routing Problem.
//!
//! # Overview
//!
//! Models a vehicle routing problem with:
//! - Geographic [`Location`]s with haversine distance calculation
//! - Customer [`Visit`]s with time windows, demand, and service duration
//! - [`Vehicle`]s with capacity constraints and routes
//! - [`VehicleRoutePlan`] as the complete planning solution
//!
//! # Design
//!
//! Uses index-based references for O(1) lookups:
//! - `Visit.location_idx` references `VehicleRoutePlan.locations`
//! - `Vehicle.home_location_idx` references `VehicleRoutePlan.locations`
//! - `Vehicle.visits` contains visit indices into `VehicleRoutePlan.visits`

use serde::{Deserialize, Serialize};
use solverforge::prelude::*;
use std::collections::HashMap;

/// Average driving speed in km/h for travel time estimation.
pub const AVERAGE_SPEED_KMPH: f64 = 50.0;

/// Earth radius in meters for haversine calculation.
const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// A geographic location with latitude and longitude.
///
/// Supports haversine distance calculation for travel time estimation.
///
/// # Examples
///
/// ```
/// use vehicle_routing::domain::Location;
///
/// let philadelphia = Location::new(0, 39.9526, -75.1652);
/// let new_york = Location::new(1, 40.7128, -74.0060);
///
/// // Distance is approximately 130 km
/// let distance = philadelphia.distance_meters(&new_york);
/// assert!(distance > 120_000.0 && distance < 140_000.0);
///
/// // Travel time at 50 km/h is approximately 2.6 hours
/// let travel_secs = philadelphia.travel_time_seconds(&new_york);
/// assert!(travel_secs > 8000 && travel_secs < 10000);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    /// Index in `VehicleRoutePlan.locations`.
    pub index: usize,
    /// Latitude in degrees (-90 to 90).
    pub latitude: f64,
    /// Longitude in degrees (-180 to 180).
    pub longitude: f64,
}

impl Location {
    /// Creates a new location.
    pub fn new(index: usize, latitude: f64, longitude: f64) -> Self {
        Self {
            index,
            latitude,
            longitude,
        }
    }

    /// Calculates the great-circle distance in meters using the haversine formula.
    ///
    /// # Examples
    ///
    /// ```
    /// use vehicle_routing::domain::Location;
    ///
    /// let a = Location::new(0, 0.0, 0.0);
    /// let b = Location::new(1, 0.0, 1.0);
    ///
    /// // 1 degree of longitude at equator is about 111 km
    /// let dist = a.distance_meters(&b);
    /// assert!(dist > 110_000.0 && dist < 112_000.0);
    /// ```
    pub fn distance_meters(&self, other: &Location) -> f64 {
        if self.latitude == other.latitude && self.longitude == other.longitude {
            return 0.0;
        }

        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let lon1 = self.longitude.to_radians();
        let lon2 = other.longitude.to_radians();

        // Haversine formula
        let dlat = lat2 - lat1;
        let dlon = lon2 - lon1;
        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_M * c
    }

    /// Calculates travel time in seconds assuming average driving speed.
    ///
    /// Uses [`AVERAGE_SPEED_KMPH`] (50 km/h) for conversion.
    pub fn travel_time_seconds(&self, other: &Location) -> i64 {
        let meters = self.distance_meters(other);
        // seconds = meters / (km/h * 1000 / 3600) = meters * 3.6 / km/h
        (meters * 3.6 / AVERAGE_SPEED_KMPH).round() as i64
    }
}

/// A customer visit with time window and demand constraints.
///
/// # Time Window
///
/// - `min_start_time`: Earliest time service can begin (vehicle may wait)
/// - `max_end_time`: Latest time service must finish (hard constraint)
/// - `service_duration`: Time required to complete the visit
///
/// All times are in seconds from midnight.
///
/// # Examples
///
/// ```
/// use vehicle_routing::domain::Visit;
///
/// // A restaurant delivery: 6am-10am window, 5-minute service
/// let visit = Visit::new(0, "Restaurant A", 0)
///     .with_demand(8)
///     .with_time_window(6 * 3600, 10 * 3600)
///     .with_service_duration(300);
///
/// assert_eq!(visit.demand, 8);
/// assert_eq!(visit.min_start_time, 21600); // 6 * 3600
/// ```
#[problem_fact]
#[derive(Serialize, Deserialize)]
pub struct Visit {
    /// Index in `VehicleRoutePlan.visits`.
    pub index: usize,
    /// Customer name.
    pub name: String,
    /// Index into `VehicleRoutePlan.locations`.
    pub location_idx: usize,
    /// Quantity demanded (must fit in vehicle capacity).
    pub demand: i32,
    /// Earliest service start time (seconds from midnight).
    #[serde(rename = "minStartTime")]
    pub min_start_time: i64,
    /// Latest service end time (seconds from midnight).
    #[serde(rename = "maxEndTime")]
    pub max_end_time: i64,
    /// Service duration in seconds.
    #[serde(rename = "serviceDuration")]
    pub service_duration: i64,
}

impl Visit {
    /// Creates a new visit with default time window (all day).
    pub fn new(index: usize, name: impl Into<String>, location_idx: usize) -> Self {
        Self {
            index,
            name: name.into(),
            location_idx,
            demand: 1,
            min_start_time: 0,
            max_end_time: 24 * 3600,
            service_duration: 0,
        }
    }

    /// Sets the demand.
    pub fn with_demand(mut self, demand: i32) -> Self {
        self.demand = demand;
        self
    }

    /// Sets the time window (min_start_time, max_end_time) in seconds from midnight.
    pub fn with_time_window(mut self, min_start: i64, max_end: i64) -> Self {
        self.min_start_time = min_start;
        self.max_end_time = max_end;
        self
    }

    /// Sets the service duration in seconds.
    pub fn with_service_duration(mut self, duration: i64) -> Self {
        self.service_duration = duration;
        self
    }
}

/// A delivery vehicle with capacity and assigned route.
///
/// The route is stored as a list of visit indices in order.
/// Arrival times are computed on-demand, not stored.
///
/// # Examples
///
/// ```
/// use vehicle_routing::domain::Vehicle;
///
/// let vehicle = Vehicle::new(0, "Truck 1", 100, 0)
///     .with_departure_time(8 * 3600);  // Departs at 8am
///
/// assert_eq!(vehicle.capacity, 100);
/// assert!(vehicle.visits.is_empty());
/// ```
#[planning_entity]
#[derive(Serialize, Deserialize)]
pub struct Vehicle {
    /// Unique vehicle ID.
    #[planning_id]
    pub id: usize,
    /// Vehicle name for display.
    pub name: String,
    /// Maximum capacity (sum of visit demands must not exceed).
    pub capacity: i32,
    /// Index into `VehicleRoutePlan.locations` for home depot.
    #[serde(rename = "homeLocation")]
    pub home_location_idx: usize,
    /// Departure time from depot (seconds from midnight).
    #[serde(rename = "departureTime")]
    pub departure_time: i64,
    /// Ordered list of visit indices (the route).
    #[serde(default)]
    pub visits: Vec<usize>,
}

impl Vehicle {
    /// Creates a new vehicle with empty route.
    pub fn new(id: usize, name: impl Into<String>, capacity: i32, home_location_idx: usize) -> Self {
        Self {
            id,
            name: name.into(),
            capacity,
            home_location_idx,
            departure_time: 8 * 3600, // Default 8am
            visits: Vec::new(),
        }
    }

    /// Sets the departure time in seconds from midnight.
    pub fn with_departure_time(mut self, time: i64) -> Self {
        self.departure_time = time;
        self
    }

    /// Calculates total demand of all visits in the route.
    ///
    /// # Examples
    ///
    /// ```
    /// use vehicle_routing::domain::{Vehicle, Visit, VehicleRoutePlan, Location};
    ///
    /// let locations = vec![Location::new(0, 0.0, 0.0)];
    /// let visits = vec![
    ///     Visit::new(0, "A", 0).with_demand(5),
    ///     Visit::new(1, "B", 0).with_demand(3),
    /// ];
    /// let mut vehicle = Vehicle::new(0, "V1", 100, 0);
    /// vehicle.visits = vec![0, 1];
    ///
    /// let plan = VehicleRoutePlan::new("test", locations, visits, vec![vehicle]);
    /// assert_eq!(plan.vehicles[0].total_demand(&plan), 8);
    /// ```
    pub fn total_demand(&self, plan: &VehicleRoutePlan) -> i32 {
        self.visits
            .iter()
            .filter_map(|&idx| plan.visits.get(idx))
            .map(|v| v.demand)
            .sum()
    }
}

/// Arrival and departure times for a visit in a route.
#[derive(Debug, Clone, Copy)]
pub struct VisitTiming {
    /// Visit index.
    pub visit_idx: usize,
    /// Arrival time at the visit (seconds from midnight).
    pub arrival: i64,
    /// Departure time from the visit (seconds from midnight).
    pub departure: i64,
}

/// The complete vehicle routing solution.
///
/// Contains all problem facts (locations, visits) and planning entities (vehicles).
/// Precomputes a travel time matrix on `finalize()` for O(1) lookups.
///
/// # Examples
///
/// ```
/// use vehicle_routing::domain::{Location, Visit, Vehicle, VehicleRoutePlan};
///
/// let locations = vec![
///     Location::new(0, 39.95, -75.17),  // Philadelphia (depot)
///     Location::new(1, 40.00, -75.10),  // Customer 1
/// ];
/// let visits = vec![
///     Visit::new(0, "Customer 1", 1).with_demand(5),
/// ];
/// let vehicles = vec![
///     Vehicle::new(0, "Truck 1", 100, 0),
/// ];
///
/// let mut plan = VehicleRoutePlan::new("Philadelphia", locations, visits, vehicles);
/// plan.finalize();
///
/// // Travel time matrix is now populated
/// assert!(plan.travel_time(0, 1) > 0);
/// ```
#[planning_solution]
#[derive(Serialize, Deserialize)]
pub struct VehicleRoutePlan {
    /// Problem name.
    pub name: String,
    /// South-west corner of bounding box (for map display).
    #[serde(rename = "southWestCorner")]
    pub south_west_corner: [f64; 2],
    /// North-east corner of bounding box (for map display).
    #[serde(rename = "northEastCorner")]
    pub north_east_corner: [f64; 2],
    /// All locations (depot and customer locations).
    #[problem_fact_collection]
    pub locations: Vec<Location>,
    /// All customer visits.
    #[problem_fact_collection]
    pub visits: Vec<Visit>,
    /// All vehicles.
    #[planning_entity_collection]
    pub vehicles: Vec<Vehicle>,
    /// Current score.
    #[planning_score]
    pub score: Option<HardSoftScore>,
    /// Solver status for REST API.
    #[serde(rename = "solverStatus", skip_serializing_if = "Option::is_none")]
    pub solver_status: Option<String>,
    /// Precomputed travel times: `travel_time_matrix[from][to]` in seconds.
    #[serde(skip)]
    pub travel_time_matrix: Vec<Vec<i64>>,
    /// Route geometries: `(from_loc, to_loc)` -> list of (lat, lng) waypoints.
    #[serde(skip)]
    pub route_geometries: HashMap<(usize, usize), Vec<(f64, f64)>>,
}

impl VehicleRoutePlan {
    /// Creates a new vehicle route plan.
    pub fn new(
        name: impl Into<String>,
        locations: Vec<Location>,
        visits: Vec<Visit>,
        vehicles: Vec<Vehicle>,
    ) -> Self {
        // Compute bounding box from locations
        let (sw, ne) = Self::compute_bounds(&locations);

        Self {
            name: name.into(),
            south_west_corner: sw,
            north_east_corner: ne,
            locations,
            visits,
            vehicles,
            score: None,
            solver_status: None,
            travel_time_matrix: Vec::new(),
            route_geometries: HashMap::new(),
        }
    }

    /// Computes bounding box from locations.
    fn compute_bounds(locations: &[Location]) -> ([f64; 2], [f64; 2]) {
        if locations.is_empty() {
            return ([0.0, 0.0], [0.0, 0.0]);
        }

        let mut min_lat = f64::MAX;
        let mut max_lat = f64::MIN;
        let mut min_lon = f64::MAX;
        let mut max_lon = f64::MIN;

        for loc in locations {
            min_lat = min_lat.min(loc.latitude);
            max_lat = max_lat.max(loc.latitude);
            min_lon = min_lon.min(loc.longitude);
            max_lon = max_lon.max(loc.longitude);
        }

        // No padding here - init_routing() adds expansion
        ([min_lat, min_lon], [max_lat, max_lon])
    }

    /// Populates the travel time matrix using haversine distances.
    ///
    /// Must be called after construction and before solving.
    /// For real road routing, use `init_routing()` instead.
    pub fn finalize(&mut self) {
        let n = self.locations.len();
        self.travel_time_matrix = vec![vec![0; n]; n];

        for i in 0..n {
            for j in 0..n {
                if i != j {
                    self.travel_time_matrix[i][j] =
                        self.locations[i].travel_time_seconds(&self.locations[j]);
                }
            }
        }
    }

    /// Initializes with real road routing from OSM data.
    ///
    /// Downloads road network via Overpass API (cached), builds graph,
    /// and computes travel times using Dijkstra shortest paths.
    /// Also stores route geometries for visualization.
    pub async fn init_routing(&mut self) -> Result<(), crate::routing::RoutingError> {
        use crate::routing::{BoundingBox, RoadNetwork};

        // Build bounding box from plan bounds (with expansion)
        let bbox = BoundingBox::new(
            self.south_west_corner[0],
            self.south_west_corner[1],
            self.north_east_corner[0],
            self.north_east_corner[1],
        )
        .expand(0.05); // 5% expansion to catch nearby roads

        // Load or fetch road network
        let network = RoadNetwork::load_or_fetch(&bbox).await?;

        // Extract coordinates
        let coords: Vec<(f64, f64)> = self
            .locations
            .iter()
            .map(|l| (l.latitude, l.longitude))
            .collect();

        // Compute travel time matrix
        self.travel_time_matrix = network.compute_matrix(&coords);

        // Compute route geometries for visualization
        self.route_geometries = network.compute_all_geometries(&coords);

        Ok(())
    }

    /// Returns the bounding box for this plan.
    pub fn bounding_box(&self) -> crate::routing::BoundingBox {
        crate::routing::BoundingBox::new(
            self.south_west_corner[0],
            self.south_west_corner[1],
            self.north_east_corner[0],
            self.north_east_corner[1],
        )
    }

    /// Gets travel time between two locations in seconds.
    ///
    /// Returns 0 if indices are out of bounds or matrix not initialized.
    #[inline]
    pub fn travel_time(&self, from_idx: usize, to_idx: usize) -> i64 {
        self.travel_time_matrix
            .get(from_idx)
            .and_then(|row| row.get(to_idx))
            .copied()
            .unwrap_or(0)
    }

    /// Gets route geometry between two locations.
    ///
    /// Returns the waypoints if real road routing was initialized,
    /// or `None` if using haversine fallback.
    #[inline]
    pub fn route_geometry(&self, from_idx: usize, to_idx: usize) -> Option<&[(f64, f64)]> {
        self.route_geometries.get(&(from_idx, to_idx)).map(|v| v.as_slice())
    }

    /// Gets a location by index.
    #[inline]
    pub fn get_location(&self, idx: usize) -> Option<&Location> {
        self.locations.get(idx)
    }

    /// Gets a visit by index.
    #[inline]
    pub fn get_visit(&self, idx: usize) -> Option<&Visit> {
        self.visits.get(idx)
    }

    /// Calculates arrival and departure times for each visit in a vehicle's route.
    ///
    /// Returns a vector of [`VisitTiming`] in route order.
    ///
    /// # Examples
    ///
    /// ```
    /// use vehicle_routing::domain::{Location, Visit, Vehicle, VehicleRoutePlan};
    ///
    /// let locations = vec![
    ///     Location::new(0, 0.0, 0.0),  // Depot
    ///     Location::new(1, 0.0, 0.01), // ~1.1 km away
    /// ];
    /// let visits = vec![
    ///     Visit::new(0, "A", 1)
    ///         .with_service_duration(300)
    ///         .with_time_window(0, 86400),
    /// ];
    /// let mut vehicle = Vehicle::new(0, "V1", 100, 0);
    /// vehicle.departure_time = 8 * 3600; // 8am
    /// vehicle.visits = vec![0];
    ///
    /// let mut plan = VehicleRoutePlan::new("test", locations, visits, vec![vehicle]);
    /// plan.finalize();
    ///
    /// let timings = plan.calculate_route_times(&plan.vehicles[0]);
    /// assert_eq!(timings.len(), 1);
    /// assert!(timings[0].arrival > 8 * 3600); // Arrives after departure
    /// assert_eq!(timings[0].departure, timings[0].arrival + 300); // Service takes 5 min
    /// ```
    pub fn calculate_route_times(&self, vehicle: &Vehicle) -> Vec<VisitTiming> {
        let mut timings = Vec::with_capacity(vehicle.visits.len());
        let mut current_time = vehicle.departure_time;
        let mut current_loc = vehicle.home_location_idx;

        for &visit_idx in &vehicle.visits {
            let Some(visit) = self.visits.get(visit_idx) else {
                continue;
            };

            // Travel to this visit
            let travel = self.travel_time(current_loc, visit.location_idx);
            let arrival = current_time + travel;

            // Service starts at max(arrival, min_start_time)
            let service_start = arrival.max(visit.min_start_time);
            let departure = service_start + visit.service_duration;

            timings.push(VisitTiming {
                visit_idx,
                arrival,
                departure,
            });

            current_time = departure;
            current_loc = visit.location_idx;
        }

        timings
    }

    /// Calculates total driving time for a vehicle's route in seconds.
    ///
    /// Includes travel from depot, between visits, and back to depot.
    pub fn total_driving_time(&self, vehicle: &Vehicle) -> i64 {
        if vehicle.visits.is_empty() {
            return 0;
        }

        let mut total = 0i64;
        let mut current_loc = vehicle.home_location_idx;

        for &visit_idx in &vehicle.visits {
            if let Some(visit) = self.visits.get(visit_idx) {
                total += self.travel_time(current_loc, visit.location_idx);
                current_loc = visit.location_idx;
            }
        }

        // Return to depot
        total += self.travel_time(current_loc, vehicle.home_location_idx);
        total
    }

    /// Calculates total driving time across all vehicles.
    pub fn total_driving_time_all(&self) -> i64 {
        self.vehicles.iter().map(|v| self.total_driving_time(v)).sum()
    }
}
