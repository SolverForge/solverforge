use crate::ProblemData;

/// Trait implemented by a planning solution that holds a fleet of vehicles,
/// each carrying a `*const ProblemData` pointer and a list of visited stops.
///
/// # Safety
/// Implementors must ensure every `vehicle_data_ptr` points to a valid
/// `ProblemData` for the entire duration of a solve call. Returning a null
/// pointer for a non-empty fleet is an initialization bug and will panic in
/// the helper functions below.
pub trait VrpSolution {
    fn vehicle_data_ptr(&self, entity_idx: usize) -> *const ProblemData;
    fn vehicle_visits(&self, entity_idx: usize) -> &[usize];
    fn vehicle_visits_mut(&mut self, entity_idx: usize) -> &mut Vec<usize>;
    fn vehicle_count(&self) -> usize;
}
