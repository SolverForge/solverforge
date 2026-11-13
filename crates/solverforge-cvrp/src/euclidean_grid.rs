//! Exact k-nearest-location queries for `EuclideanMetric` over a uniform grid.

use crate::metric::{leg_rank, nearest_by_rank};
use crate::{EuclideanMetric, RouteMetric};

/// Writes what `RouteMetric::nearest` specifies, ranking by the same rounded
/// leg distance and index the provided method uses, so both agree exactly.
pub(crate) fn nearest(
    metric: &EuclideanMetric,
    locations: &[usize],
    k: usize,
    out: &mut Vec<usize>,
) {
    let stride = k.min(locations.len().saturating_sub(1));
    if stride == 0 {
        return;
    }
    let coordinates = metric.coordinates();
    let Some(points) = locations
        .iter()
        .map(|&location| coordinates.get(location).copied())
        .collect::<Option<Vec<_>>>()
    else {
        // A location outside the coordinates has no position to index; the
        // provided ranking handles its undefined legs.
        return nearest_by_rank(metric, locations, k, out);
    };
    let grid = Grid::new(&points);
    let mut best: Vec<(i64, usize)> = Vec::with_capacity(stride + 1);
    for (index, &point) in points.iter().enumerate() {
        best.clear();
        let (cell_x, cell_y) = grid.cell_of(point);
        let mut ring = 0usize;
        loop {
            grid.visit_ring(cell_x, cell_y, ring, |other| {
                if other == index {
                    return;
                }
                let key = (
                    leg_rank(metric.distance(locations[index], locations[other])),
                    other,
                );
                if best.len() < stride || key < best[stride - 1] {
                    let at = best.partition_point(|&held| held < key);
                    best.insert(at, key);
                    best.truncate(stride);
                }
            });
            // Every location outside rings 0..=ring lies at least
            // `ring * cell` away, so its rounded distance exceeds the k-th
            // best once that bound passes the k-th best by half a unit.
            let settled = best.len() == stride && {
                let bound = ring as f64 * grid.cell;
                let kth = best[stride - 1].0 as f64;
                bound > kth + 0.5 + 1e-9 * (1.0 + kth.abs())
            };
            if settled || ring > grid.width.max(grid.height) {
                break;
            }
            ring += 1;
        }
        out.extend(best.iter().map(|&(_, other)| other));
    }
}

/// Location indexes bucketed by cell, row-major, ascending within a cell.
struct Grid {
    min_x: f64,
    min_y: f64,
    cell: f64,
    width: usize,
    height: usize,
    cell_start: Vec<usize>,
    members: Vec<usize>,
}

impl Grid {
    fn new(points: &[(f64, f64)]) -> Self {
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &(x, y) in points {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        let span_x = max_x - min_x;
        let span_y = max_y - min_y;
        // About two locations per cell over the bounding box.
        let area = (span_x.max(1.0)) * (span_y.max(1.0));
        let cell = (2.0 * area / points.len() as f64).sqrt().max(1e-9);
        let width = (span_x / cell) as usize + 1;
        let height = (span_y / cell) as usize + 1;
        let mut grid = Self {
            min_x,
            min_y,
            cell,
            width,
            height,
            cell_start: vec![0; width * height + 1],
            members: vec![0; points.len()],
        };
        let cells: Vec<usize> = points
            .iter()
            .map(|&point| {
                let (x, y) = grid.cell_of(point);
                y * width + x
            })
            .collect();
        for &cell_index in &cells {
            grid.cell_start[cell_index + 1] += 1;
        }
        for cell_index in 0..width * height {
            grid.cell_start[cell_index + 1] += grid.cell_start[cell_index];
        }
        let mut next = grid.cell_start.clone();
        for (index, &cell_index) in cells.iter().enumerate() {
            grid.members[next[cell_index]] = index;
            next[cell_index] += 1;
        }
        grid
    }

    fn cell_of(&self, (x, y): (f64, f64)) -> (usize, usize) {
        let cell_x = (((x - self.min_x) / self.cell) as usize).min(self.width - 1);
        let cell_y = (((y - self.min_y) / self.cell) as usize).min(self.height - 1);
        (cell_x, cell_y)
    }

    /// Visits the locations of the cells at Chebyshev distance `ring` from
    /// `(cell_x, cell_y)` that lie inside the grid.
    fn visit_ring(&self, cell_x: usize, cell_y: usize, ring: usize, mut visit: impl FnMut(usize)) {
        let (cx, cy, r) = (cell_x as isize, cell_y as isize, ring as isize);
        let (width, height) = (self.width as isize, self.height as isize);
        for y in (cy - r).max(0)..=(cy + r).min(height - 1) {
            let edge_row = y == cy - r || y == cy + r;
            let step = if edge_row || r == 0 { 1 } else { 2 * r };
            let mut x = cx - r;
            while x <= cx + r {
                if (0..width).contains(&x) {
                    let cell_index = y as usize * self.width + x as usize;
                    for &member in
                        &self.members[self.cell_start[cell_index]..self.cell_start[cell_index + 1]]
                    {
                        visit(member);
                    }
                }
                x += step;
            }
        }
    }
}
