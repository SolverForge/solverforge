use std::cell::Cell;
use std::fmt;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::support::{distance, ListPlan};

thread_local! {
    static SOLUTION_CLONES: Cell<usize> = const { Cell::new(0) };
    static METER_CLONES: Cell<usize> = const { Cell::new(0) };
}

pub(super) struct PositionMetric;

impl Clone for PositionMetric {
    fn clone(&self) -> Self {
        METER_CLONES.set(METER_CLONES.get() + 1);
        Self
    }
}

impl fmt::Debug for PositionMetric {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PositionMetric")
    }
}

impl CrossEntityDistanceMeter<ListPlan> for PositionMetric {
    fn distance(
        &self,
        _: &ListPlan,
        from_entity: usize,
        from_position: usize,
        to_entity: usize,
        to_position: usize,
    ) -> f64 {
        distance(from_entity, from_position, to_entity, to_position) as f64
    }
}

pub(super) fn record_solution_clone() {
    SOLUTION_CLONES.set(SOLUTION_CLONES.get() + 1);
}

pub(super) fn reset_solution_clones() {
    SOLUTION_CLONES.set(0);
}

pub(super) fn solution_clones() -> usize {
    SOLUTION_CLONES.get()
}

pub(super) fn reset_meter_clones() {
    METER_CLONES.set(0);
}

pub(super) fn meter_clones() -> usize {
    METER_CLONES.get()
}
