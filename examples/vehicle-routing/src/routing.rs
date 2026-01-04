//! Local OSM road routing using Overpass API and petgraph.
//!
//! Downloads OpenStreetMap road network data via Overpass API,
//! builds a graph locally, and computes shortest paths with Dijkstra.
//! Results are cached in memory (per-process) and `.osm_cache/` (persistent).

use ordered_float::OrderedFloat;
use petgraph::algo::{astar, dijkstra};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// In-memory cache of road networks, keyed by bbox cache key.
/// First request downloads, subsequent requests reuse the cached network.
static NETWORK_CACHE: OnceLock<RwLock<HashMap<String, Arc<RoadNetwork>>>> = OnceLock::new();

fn network_cache() -> &'static RwLock<HashMap<String, Arc<RoadNetwork>>> {
    NETWORK_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Overpass API URL.
const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";

/// Cache directory for downloaded OSM data.
const CACHE_DIR: &str = ".osm_cache";

/// Default driving speed in m/s (50 km/h = 13.89 m/s).
const DEFAULT_SPEED_MPS: f64 = 50.0 * 1000.0 / 3600.0;

/// Error type for routing operations.
#[derive(Debug)]
pub enum RoutingError {
    /// Network request failed.
    Network(String),
    /// Failed to parse OSM data.
    Parse(String),
    /// I/O error.
    Io(std::io::Error),
    /// No route found.
    NoRoute,
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingError::Network(msg) => write!(f, "Network error: {}", msg),
            RoutingError::Parse(msg) => write!(f, "Parse error: {}", msg),
            RoutingError::Io(e) => write!(f, "I/O error: {}", e),
            RoutingError::NoRoute => write!(f, "No route found"),
        }
    }
}

impl std::error::Error for RoutingError {}

impl From<std::io::Error> for RoutingError {
    fn from(e: std::io::Error) -> Self {
        RoutingError::Io(e)
    }
}

/// Bounding box for OSM queries.
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub min_lng: f64,
    pub max_lat: f64,
    pub max_lng: f64,
}

impl BoundingBox {
    /// Creates a new bounding box.
    pub fn new(min_lat: f64, min_lng: f64, max_lat: f64, max_lng: f64) -> Self {
        Self {
            min_lat,
            min_lng,
            max_lat,
            max_lng,
        }
    }

    /// Expands the bounding box by a factor (e.g., 0.1 = 10% on each side).
    pub fn expand(&self, factor: f64) -> Self {
        let lat_range = self.max_lat - self.min_lat;
        let lng_range = self.max_lng - self.min_lng;
        let lat_pad = lat_range * factor;
        let lng_pad = lng_range * factor;

        Self {
            min_lat: self.min_lat - lat_pad,
            min_lng: self.min_lng - lng_pad,
            max_lat: self.max_lat + lat_pad,
            max_lng: self.max_lng + lng_pad,
        }
    }

    /// Returns a cache key for this bounding box.
    fn cache_key(&self) -> String {
        format!(
            "{:.4}_{:.4}_{:.4}_{:.4}",
            self.min_lat, self.min_lng, self.max_lat, self.max_lng
        )
    }
}

/// Node data in the road graph.
#[derive(Debug, Clone)]
struct NodeData {
    lat: f64,
    lng: f64,
}

/// Edge data in the road graph.
#[derive(Debug, Clone)]
struct EdgeData {
    /// Travel time in seconds.
    travel_time_s: f64,
    /// Distance in meters.
    distance_m: f64,
    /// Intermediate geometry points (for future full path reconstruction).
    #[allow(dead_code)]
    geometry: Vec<(f64, f64)>,
}

/// Result of a route computation.
#[derive(Debug, Clone)]
pub struct RouteResult {
    /// Travel time in seconds.
    pub duration_seconds: i64,
    /// Distance in meters.
    pub distance_meters: f64,
    /// Full route geometry (lat, lng pairs).
    pub geometry: Vec<(f64, f64)>,
}

/// Road network graph built from OSM data.
pub struct RoadNetwork {
    /// Directed graph with travel times as edge weights.
    graph: DiGraph<NodeData, EdgeData>,
    /// Map from (lat_e7, lng_e7) to node index.
    coord_to_node: HashMap<(i64, i64), NodeIndex>,
}

impl RoadNetwork {
    /// Creates an empty road network.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            coord_to_node: HashMap::new(),
        }
    }

    /// Loads or fetches road network for a bounding box.
    ///
    /// Uses three-tier caching:
    /// 1. In-memory cache (instant, per-process)
    /// 2. File cache (fast, persists across restarts)
    /// 3. Overpass API download (slow, ~5-30s)
    ///
    /// Thread-safe: concurrent requests for the same bbox will wait for
    /// the first download to complete rather than downloading multiple times.
    pub async fn load_or_fetch(bbox: &BoundingBox) -> Result<Arc<Self>, RoutingError> {
        let cache_key = bbox.cache_key();

        // 1. Check in-memory cache (fast path, read lock)
        {
            let cache = network_cache().read().await;
            if let Some(network) = cache.get(&cache_key) {
                info!("Using in-memory cached road network for {}", cache_key);
                return Ok(Arc::clone(network));
            }
        }

        // 2. Acquire write lock and double-check (another request may have loaded it)
        let mut cache = network_cache().write().await;
        if let Some(network) = cache.get(&cache_key) {
            info!("Using in-memory cached road network for {}", cache_key);
            return Ok(Arc::clone(network));
        }

        // 3. Try loading from file cache
        tokio::fs::create_dir_all(CACHE_DIR).await?;
        let cache_path = Path::new(CACHE_DIR).join(format!("{}.json", cache_key));

        let network = if tokio::fs::try_exists(&cache_path).await.unwrap_or(false) {
            info!("Loading road network from file cache: {:?}", cache_path);
            match Self::load_from_cache(&cache_path).await {
                Ok(n) => n,
                Err(e) => {
                    // File cache failed (corrupted/old version), download fresh
                    info!("File cache invalid ({}), downloading fresh", e);
                    let n = Self::from_bbox(bbox).await?;
                    n.save_to_cache(&cache_path).await?;
                    info!("Saved road network to file cache: {:?}", cache_path);
                    n
                }
            }
        } else {
            // 4. Download from Overpass API
            info!("Downloading road network from Overpass API");
            let n = Self::from_bbox(bbox).await?;
            n.save_to_cache(&cache_path).await?;
            info!("Saved road network to file cache: {:?}", cache_path);
            n
        };

        // Store in memory cache
        let network = Arc::new(network);
        cache.insert(cache_key, Arc::clone(&network));

        Ok(network)
    }

    /// Downloads and builds road network from Overpass API.
    pub async fn from_bbox(bbox: &BoundingBox) -> Result<Self, RoutingError> {
        let query = format!(
            r#"[out:json][timeout:120];
(
  way["highway"~"^(motorway|trunk|primary|secondary|tertiary|residential|unclassified|service|living_street)$"]
    ({},{},{},{});
);
(._;>;);
out body;"#,
            bbox.min_lat, bbox.min_lng, bbox.max_lat, bbox.max_lng
        );

        debug!("Overpass query:\n{}", query);

        info!("Preparing Overpass query for bbox: {:.4},{:.4} to {:.4},{:.4}",
            bbox.min_lat, bbox.min_lng, bbox.max_lat, bbox.max_lng);

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .read_timeout(std::time::Duration::from_secs(180))
            .timeout(std::time::Duration::from_secs(180))
            .user_agent("SolverForge/0.4.0")
            .build()
            .map_err(|e| RoutingError::Network(e.to_string()))?;

        info!("Sending request to Overpass API...");

        let response = client
            .post(OVERPASS_URL)
            .body(query)
            .header("Content-Type", "text/plain")
            .send()
            .await
            .map_err(|e| {
                error!("Overpass request failed: {}", e);
                RoutingError::Network(e.to_string())
            })?;

        info!("Received response: status={}", response.status());

        if !response.status().is_success() {
            return Err(RoutingError::Network(format!(
                "Overpass API returned status {}",
                response.status()
            )));
        }

        let osm_data: OverpassResponse = response
            .json()
            .await
            .map_err(|e| RoutingError::Parse(e.to_string()))?;

        info!(
            "Downloaded {} OSM elements",
            osm_data.elements.len()
        );

        Self::build_from_osm(&osm_data)
    }

    /// Builds the road network from parsed OSM data.
    fn build_from_osm(osm: &OverpassResponse) -> Result<Self, RoutingError> {
        let mut network = Self::new();

        // First pass: collect all nodes
        let mut nodes: HashMap<i64, (f64, f64)> = HashMap::new();
        for elem in &osm.elements {
            if elem.elem_type == "node" {
                if let (Some(lat), Some(lon)) = (elem.lat, elem.lon) {
                    nodes.insert(elem.id, (lat, lon));
                }
            }
        }

        info!("Parsed {} nodes", nodes.len());

        // Second pass: process ways and build graph
        let mut way_count = 0;
        for elem in &osm.elements {
            if elem.elem_type == "way" {
                if let Some(ref node_ids) = elem.nodes {
                    let highway = elem.tags.as_ref().and_then(|t| t.highway.as_deref());
                    let oneway = elem.tags.as_ref().and_then(|t| t.oneway.as_deref());
                    let speed = get_speed_for_highway(highway.unwrap_or("residential"));
                    let is_oneway = matches!(oneway, Some("yes") | Some("1"));

                    // Process consecutive node pairs
                    for window in node_ids.windows(2) {
                        let n1_id = window[0];
                        let n2_id = window[1];

                        let Some(&(lat1, lng1)) = nodes.get(&n1_id) else {
                            continue;
                        };
                        let Some(&(lat2, lng2)) = nodes.get(&n2_id) else {
                            continue;
                        };

                        // Get or create node indices
                        let idx1 = network.get_or_create_node(lat1, lng1);
                        let idx2 = network.get_or_create_node(lat2, lng2);

                        // Calculate edge properties
                        let distance = haversine_distance(lat1, lng1, lat2, lng2);
                        let travel_time = distance / speed;

                        let edge_data = EdgeData {
                            travel_time_s: travel_time,
                            distance_m: distance,
                            geometry: vec![(lat1, lng1), (lat2, lng2)],
                        };

                        // Add forward edge
                        network.graph.add_edge(idx1, idx2, edge_data.clone());

                        // Add reverse edge if not oneway
                        if !is_oneway {
                            network.graph.add_edge(idx2, idx1, edge_data);
                        }
                    }

                    way_count += 1;
                }
            }
        }

        info!(
            "Built graph with {} nodes and {} edges from {} ways",
            network.graph.node_count(),
            network.graph.edge_count(),
            way_count
        );

        Ok(network)
    }

    /// Gets or creates a node for the given coordinates.
    fn get_or_create_node(&mut self, lat: f64, lng: f64) -> NodeIndex {
        let key = coord_key(lat, lng);
        if let Some(&idx) = self.coord_to_node.get(&key) {
            idx
        } else {
            let idx = self.graph.add_node(NodeData { lat, lng });
            self.coord_to_node.insert(key, idx);
            idx
        }
    }

    /// Finds the nearest road node to the given coordinates.
    pub fn snap_to_road(&self, lat: f64, lng: f64) -> Option<NodeIndex> {
        self.coord_to_node
            .iter()
            .min_by_key(|((lat_e7, lng_e7), _)| {
                let node_lat = *lat_e7 as f64 / 1e7;
                let node_lng = *lng_e7 as f64 / 1e7;
                OrderedFloat(haversine_distance(lat, lng, node_lat, node_lng))
            })
            .map(|(_, &idx)| idx)
    }

    /// Computes shortest path between two coordinates.
    ///
    /// Returns the route with full geometry following roads.
    pub fn route(&self, from: (f64, f64), to: (f64, f64)) -> Option<RouteResult> {
        let start = self.snap_to_road(from.0, from.1)?;
        let end = self.snap_to_road(to.0, to.1)?;

        if start == end {
            return Some(RouteResult {
                duration_seconds: 0,
                distance_meters: 0.0,
                geometry: vec![from, to],
            });
        }

        // Use A* with zero heuristic (equivalent to Dijkstra, but returns full path)
        let (cost, path) = astar(
            &self.graph,
            start,
            |n| n == end,
            |e| OrderedFloat(e.weight().travel_time_s),
            |_| OrderedFloat(0.0),
        )?;

        let total_time = cost.0;

        // Build geometry from path nodes
        let geometry: Vec<(f64, f64)> = path
            .iter()
            .filter_map(|&idx| self.graph.node_weight(idx).map(|n| (n.lat, n.lng)))
            .collect();

        // Sum actual edge distances along the path
        let mut distance = 0.0;
        for window in path.windows(2) {
            if let Some(edge) = self.graph.find_edge(window[0], window[1]) {
                if let Some(weight) = self.graph.edge_weight(edge) {
                    distance += weight.distance_m;
                }
            }
        }

        Some(RouteResult {
            duration_seconds: total_time.round() as i64,
            distance_meters: distance,
            geometry,
        })
    }

    /// Computes route geometries for all location pairs.
    ///
    /// Returns a map from `(from_idx, to_idx)` to the route geometry.
    pub fn compute_all_geometries(
        &self,
        locations: &[(f64, f64)],
    ) -> HashMap<(usize, usize), Vec<(f64, f64)>> {
        let n = locations.len();
        let mut geometries = HashMap::new();

        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                if let Some(result) = self.route(locations[i], locations[j]) {
                    geometries.insert((i, j), result.geometry);
                }
            }
        }

        geometries
    }

    /// Computes all-pairs travel time matrix for given locations.
    ///
    /// Returns a matrix where `result[i][j]` is the travel time from location i to j.
    pub fn compute_matrix(&self, locations: &[(f64, f64)]) -> Vec<Vec<i64>> {
        let n = locations.len();
        let mut matrix = vec![vec![0i64; n]; n];

        // Snap all locations to nodes
        let nodes: Vec<Option<NodeIndex>> = locations
            .iter()
            .map(|&(lat, lng)| self.snap_to_road(lat, lng))
            .collect();

        // Compute travel times
        for i in 0..n {
            if let Some(from_node) = nodes[i] {
                // Run Dijkstra from this node
                let costs = dijkstra(&self.graph, from_node, None, |e| {
                    OrderedFloat(e.weight().travel_time_s)
                });

                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    if let Some(to_node) = nodes[j] {
                        if let Some(cost) = costs.get(&to_node) {
                            matrix[i][j] = cost.0.round() as i64;
                        } else {
                            // No route found, use haversine estimate
                            let dist = haversine_distance(
                                locations[i].0,
                                locations[i].1,
                                locations[j].0,
                                locations[j].1,
                            );
                            matrix[i][j] = (dist / DEFAULT_SPEED_MPS).round() as i64;
                        }
                    }
                }
            } else {
                // Location couldn't be snapped, use haversine for all
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let dist = haversine_distance(
                        locations[i].0,
                        locations[i].1,
                        locations[j].0,
                        locations[j].1,
                    );
                    matrix[i][j] = (dist / DEFAULT_SPEED_MPS).round() as i64;
                }
            }
        }

        matrix
    }

    /// Returns the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Returns the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Loads road network from cache file.
    async fn load_from_cache(path: &Path) -> Result<Self, RoutingError> {
        let data = tokio::fs::read_to_string(path).await?;

        // Parse cached data, handling corrupted files
        let cached: CachedNetwork = match serde_json::from_str(&data) {
            Ok(c) => c,
            Err(e) => {
                info!("Cache file corrupted, will re-download: {}", e);
                let _ = tokio::fs::remove_file(path).await;
                return Err(RoutingError::Parse(e.to_string()));
            }
        };

        // Check version - delete old format and re-download
        if cached.version != CACHE_VERSION {
            info!(
                "Cache version mismatch (got {}, need {}), will re-download",
                cached.version, CACHE_VERSION
            );
            let _ = tokio::fs::remove_file(path).await;
            return Err(RoutingError::Parse("cache version mismatch".into()));
        }

        let mut network = Self::new();

        // Rebuild graph from cached data
        for node in &cached.nodes {
            let idx = network.graph.add_node(NodeData {
                lat: node.lat,
                lng: node.lng,
            });
            let key = coord_key(node.lat, node.lng);
            network.coord_to_node.insert(key, idx);
        }

        for edge in &cached.edges {
            let from = NodeIndex::new(edge.from);
            let to = NodeIndex::new(edge.to);
            network.graph.add_edge(
                from,
                to,
                EdgeData {
                    travel_time_s: edge.travel_time_s,
                    distance_m: edge.distance_m,
                    geometry: vec![],
                },
            );
        }

        Ok(network)
    }

    /// Saves road network to cache file.
    async fn save_to_cache(&self, path: &Path) -> Result<(), RoutingError> {
        let nodes: Vec<CachedNode> = self
            .graph
            .node_indices()
            .filter_map(|idx| {
                self.graph.node_weight(idx).map(|n| CachedNode {
                    lat: n.lat,
                    lng: n.lng,
                })
            })
            .collect();

        let edges: Vec<CachedEdge> = self
            .graph
            .edge_indices()
            .filter_map(|idx| {
                let (from, to) = self.graph.edge_endpoints(idx)?;
                let weight = self.graph.edge_weight(idx)?;
                Some(CachedEdge {
                    from: from.index(),
                    to: to.index(),
                    travel_time_s: weight.travel_time_s,
                    distance_m: weight.distance_m,
                })
            })
            .collect();

        let cached = CachedNetwork {
            version: CACHE_VERSION,
            nodes,
            edges,
        };
        let data = serde_json::to_string(&cached).map_err(|e| RoutingError::Parse(e.to_string()))?;
        tokio::fs::write(path, data).await?;

        Ok(())
    }
}

impl Default for RoadNetwork {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// OSM Data Structures (Overpass API)
// ============================================================================

#[derive(Debug, Deserialize)]
struct OverpassResponse {
    elements: Vec<OsmElement>,
}

#[derive(Debug, Deserialize)]
struct OsmElement {
    #[serde(rename = "type")]
    elem_type: String,
    id: i64,
    lat: Option<f64>,
    lon: Option<f64>,
    nodes: Option<Vec<i64>>,
    tags: Option<OsmTags>,
}

#[derive(Debug, Deserialize)]
struct OsmTags {
    highway: Option<String>,
    oneway: Option<String>,
    /// Maxspeed tag (for future use with dynamic speed calculation).
    #[allow(dead_code)]
    maxspeed: Option<String>,
}

// ============================================================================
// Cache Data Structures
// ============================================================================

/// Cache format version. Bump this when changing the cache structure.
const CACHE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct CachedNetwork {
    /// Cache format version for automatic invalidation.
    version: u32,
    nodes: Vec<CachedNode>,
    edges: Vec<CachedEdge>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedNode {
    lat: f64,
    lng: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedEdge {
    from: usize,
    to: usize,
    travel_time_s: f64,
    distance_m: f64,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Converts coordinates to a hash key (7 decimal places precision).
fn coord_key(lat: f64, lng: f64) -> (i64, i64) {
    ((lat * 1e7).round() as i64, (lng * 1e7).round() as i64)
}

/// Returns speed in m/s for a highway type.
fn get_speed_for_highway(highway: &str) -> f64 {
    let kmh = match highway {
        "motorway" | "motorway_link" => 100.0,
        "trunk" | "trunk_link" => 80.0,
        "primary" | "primary_link" => 60.0,
        "secondary" | "secondary_link" => 50.0,
        "tertiary" | "tertiary_link" => 40.0,
        "residential" => 30.0,
        "unclassified" => 30.0,
        "service" => 20.0,
        "living_street" => 10.0,
        _ => 30.0,
    };
    kmh * 1000.0 / 3600.0
}

/// Haversine distance between two points in meters.
fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius in meters

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let dlat = (lat2 - lat1).to_radians();
    let dlng = (lng2 - lng1).to_radians();

    let a = (dlat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (dlng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    R * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance() {
        // Philadelphia City Hall to Liberty Bell (~500m)
        let dist = haversine_distance(39.9526, -75.1635, 39.9496, -75.1503);
        assert!((dist - 1200.0).abs() < 100.0); // Approximately 1.2 km
    }

    #[test]
    fn test_coord_key() {
        let key = coord_key(39.9526, -75.1635);
        assert_eq!(key, (399526000, -751635000));
    }

    #[test]
    fn test_bbox_expand() {
        let bbox = BoundingBox::new(39.9, -75.2, 40.0, -75.1);
        let expanded = bbox.expand(0.1);
        assert!(expanded.min_lat < bbox.min_lat);
        assert!(expanded.max_lat > bbox.max_lat);
    }

    #[test]
    fn test_empty_network() {
        let network = RoadNetwork::new();
        assert_eq!(network.node_count(), 0);
        assert_eq!(network.edge_count(), 0);
    }

    #[test]
    fn test_snap_to_road_empty() {
        let network = RoadNetwork::new();
        assert!(network.snap_to_road(39.95, -75.16).is_none());
    }
}
