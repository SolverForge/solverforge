use std::fmt;

/// Source of leg distances and travel times between locations.
///
/// The stock CVRP helpers, meters and route checks read legs only through
/// this trait, so a [`ProblemData`](crate::ProblemData) can hold a dense
/// matrix or compute legs on demand. Values follow the matrix conventions:
/// [`UNREACHABLE`](crate::UNREACHABLE) or a negative value marks a leg that
/// cannot be traversed, and `None` marks a location outside the metric.
pub trait RouteMetric: Clone + fmt::Debug + Send + Sync + 'static {
    /// Distance from `from` to `to`; `None` when either is outside the metric.
    fn distance(&self, from: usize, to: usize) -> Option<i64>;

    /// Travel time from `from` to `to`; `None` when either is outside the
    /// metric.
    fn travel_time(&self, from: usize, to: usize) -> Option<i64>;

    /// Whether every leg among `route`'s visits and `depot` is defined, given
    /// that `max_node` is the largest of them.
    fn covers(&self, route: &[usize], depot: usize, max_node: usize) -> bool;
}

/// Dense distance and travel-time tables, indexed `[from][to]`.
///
/// Holds exactly the values a caller supplies: rows may differ in length,
/// and a leg outside a row is undefined, so a route touching it is not
/// structurally valid.
#[derive(Clone, Debug)]
pub struct DenseMatrix {
    /// Distance from location `[from][to]`.
    pub distances: Vec<Vec<i64>>,
    /// Travel time from location `[from][to]`.
    pub travel_times: Vec<Vec<i64>>,
}

impl DenseMatrix {
    /// Builds the metric from a distance table and a travel-time table.
    pub fn new(distances: Vec<Vec<i64>>, travel_times: Vec<Vec<i64>>) -> Self {
        Self {
            distances,
            travel_times,
        }
    }
}

impl RouteMetric for DenseMatrix {
    #[inline]
    fn distance(&self, from: usize, to: usize) -> Option<i64> {
        self.distances.get(from)?.get(to).copied()
    }

    #[inline]
    fn travel_time(&self, from: usize, to: usize) -> Option<i64> {
        self.travel_times.get(from)?.get(to).copied()
    }

    fn covers(&self, route: &[usize], depot: usize, max_node: usize) -> bool {
        if max_node >= self.distances.len() || max_node >= self.travel_times.len() {
            return false;
        }
        route.iter().chain(std::iter::once(&depot)).all(|&node| {
            self.distances[node].len() > max_node && self.travel_times[node].len() > max_node
        })
    }
}

/// Straight-line legs between planar coordinates, computed on demand.
///
/// Distance is the Euclidean length of the leg, `sqrt(dx² + dy²)`, rounded to
/// the nearest integer with halves away from zero (TSPLIB `EUC_2D`); travel
/// time is that length divided by `speed`, rounded the same way. Memory is
/// linear in the location count.
#[derive(Clone, Debug)]
pub struct EuclideanMetric {
    coordinates: Vec<(f64, f64)>,
    speed: f64,
}

impl EuclideanMetric {
    /// Builds the metric from one `(x, y)` per location and a travel speed in
    /// distance units per time unit.
    ///
    /// # Panics
    ///
    /// Panics when `speed` is not finite and positive, or a coordinate is not
    /// finite.
    pub fn new(coordinates: Vec<(f64, f64)>, speed: f64) -> Self {
        assert!(
            speed.is_finite() && speed > 0.0,
            "EuclideanMetric speed must be finite and positive, got {speed}"
        );
        assert!(
            coordinates
                .iter()
                .all(|(x, y)| x.is_finite() && y.is_finite()),
            "EuclideanMetric coordinates must be finite"
        );
        Self { coordinates, speed }
    }

    /// The `(x, y)` coordinates, one per location.
    pub fn coordinates(&self) -> &[(f64, f64)] {
        &self.coordinates
    }

    /// The travel speed in distance units per time unit.
    pub fn speed(&self) -> f64 {
        self.speed
    }

    #[inline]
    fn length(&self, from: usize, to: usize) -> Option<f64> {
        let (from_x, from_y) = *self.coordinates.get(from)?;
        let (to_x, to_y) = *self.coordinates.get(to)?;
        let (dx, dy) = (to_x - from_x, to_y - from_y);
        Some((dx * dx + dy * dy).sqrt())
    }
}

impl RouteMetric for EuclideanMetric {
    #[inline]
    fn distance(&self, from: usize, to: usize) -> Option<i64> {
        self.length(from, to).map(|length| length.round() as i64)
    }

    #[inline]
    fn travel_time(&self, from: usize, to: usize) -> Option<i64> {
        self.length(from, to)
            .map(|length| (length / self.speed).round() as i64)
    }

    fn covers(&self, _route: &[usize], _depot: usize, max_node: usize) -> bool {
        max_node < self.coordinates.len()
    }
}
