use std::collections::BTreeMap;
use std::marker::PhantomData;

use super::{Accumulator, UniCollector};

/* A consecutive run of unique integer points.

`item_count` includes duplicate input items mapped to points in the run.
`point_count` counts only unique points.
*/
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    start: i64,
    end: i64,
    point_count: usize,
    item_count: usize,
}

impl Run {
    fn new(start: i64, end: i64, point_count: usize, item_count: usize) -> Self {
        Self {
            start,
            end,
            point_count,
            item_count,
        }
    }

    #[inline]
    pub fn start(&self) -> i64 {
        self.start
    }

    #[inline]
    pub fn end(&self) -> i64 {
        self.end
    }

    #[inline]
    pub fn point_count(&self) -> usize {
        self.point_count
    }

    #[inline]
    pub fn item_count(&self) -> usize {
        self.item_count
    }
}

// Aggregated consecutive-run result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Runs {
    runs: Vec<Run>,
    point_count: usize,
    item_count: usize,
}

impl Runs {
    #[inline]
    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    #[inline]
    pub fn point_count(&self) -> usize {
        self.point_count
    }

    #[inline]
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.runs.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }
}

/* Creates a collector that groups extracted `i64` points into consecutive runs.

Duplicate points increase the `item_count` of their run but count as one unique
point for `point_count`.
*/
pub fn consecutive_runs<A, F>(index_fn: F) -> RunsCollector<A, F>
where
    A: Send + Sync,
    F: Fn(&A) -> i64 + Send + Sync,
{
    RunsCollector {
        index_fn,
        _phantom: PhantomData,
    }
}

// Collector for consecutive runs over concrete i64 indexes.
pub struct RunsCollector<A, F> {
    index_fn: F,
    _phantom: PhantomData<fn(&A)>,
}

impl<A, F> UniCollector<A> for RunsCollector<A, F>
where
    A: Send + Sync,
    F: Fn(&A) -> i64 + Send + Sync,
{
    type Value = i64;
    type Result = Runs;
    type Accumulator = RunsAccumulator;

    #[inline]
    fn extract(&self, entity: &A) -> Self::Value {
        (self.index_fn)(entity)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        RunsAccumulator::new()
    }
}

// Incremental accumulator for consecutive runs.
pub struct RunsAccumulator {
    points: BTreeMap<i64, usize>,
}

impl RunsAccumulator {
    fn new() -> Self {
        Self {
            points: BTreeMap::new(),
        }
    }
}

impl Accumulator<i64, Runs> for RunsAccumulator {
    #[inline]
    fn accumulate(&mut self, value: &i64) {
        *self.points.entry(*value).or_insert(0) += 1;
    }

    #[inline]
    fn retract(&mut self, value: &i64) {
        let Some(count) = self.points.get_mut(value) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count == 0 {
            self.points.remove(value);
        }
    }

    fn finish(&self) -> Runs {
        runs_from_counts(&self.points)
    }

    #[inline]
    fn reset(&mut self) {
        self.points.clear();
    }
}

pub(crate) fn runs_from_counts(points: &BTreeMap<i64, usize>) -> Runs {
    let point_count = points.len();
    let item_count = points.values().sum();
    let mut runs = Vec::new();

    let mut current_start = None;
    let mut previous = 0;
    let mut current_point_count = 0;
    let mut current_item_count = 0;

    for (&point, &count) in points {
        match current_start {
            None => {
                current_start = Some(point);
                previous = point;
                current_point_count = 1;
                current_item_count = count;
            }
            Some(_) if previous.checked_add(1) == Some(point) => {
                previous = point;
                current_point_count += 1;
                current_item_count += count;
            }
            Some(start) => {
                runs.push(Run::new(
                    start,
                    previous,
                    current_point_count,
                    current_item_count,
                ));
                current_start = Some(point);
                previous = point;
                current_point_count = 1;
                current_item_count = count;
            }
        }
    }

    if let Some(start) = current_start {
        runs.push(Run::new(
            start,
            previous,
            current_point_count,
            current_item_count,
        ));
    }

    Runs {
        runs,
        point_count,
        item_count,
    }
}
