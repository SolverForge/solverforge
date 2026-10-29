use super::*;

/// The one position-metric adapter for compiled list slots.
///
/// Native list declarations retain their established intra metric as a
/// cross-entity meter and are invoked with the same source and destination
/// entity. Dynamic slots never use that generic meter: they delegate to their
/// explicitly bound dynamic metadata. Compilation validates the corresponding
/// capability before a K-opt consumer can call this adapter.
impl<S, V, DM, IDM> ListPositionDistanceMeter<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn distance(
        &self,
        solution: &S,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> f64 {
        ListAccess::intra_position_distance(self, solution, entity, from_position, to_position)
            .expect("compiled list position distance requires a validated slot capability")
    }
}
