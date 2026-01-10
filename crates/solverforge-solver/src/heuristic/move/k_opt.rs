//! K-opt move for tour optimization.
//!
//! K-opt removes k edges from a tour and reconnects the resulting segments
//! in a different order, potentially reversing some segments. This is a
//! fundamental move for TSP and VRP optimization.
//!
//! # Zero-Erasure Design
//!
//! - Fixed arrays for cut points (no SmallVec for static data)
//! - Reference to static reconnection pattern
//! - Typed function pointers for all list operations
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::r#move::{KOptMove, CutPoint};
//! use solverforge_solver::heuristic::r#move::k_opt_reconnection::THREE_OPT_RECONNECTIONS;
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug)]
//! struct Tour { cities: Vec<i32>, score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Tour {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! fn list_len(s: &Tour, _: usize) -> usize { s.cities.len() }
//! fn sublist_remove(s: &mut Tour, _: usize, start: usize, end: usize) -> Vec<i32> {
//!     s.cities.drain(start..end).collect()
//! }
//! fn sublist_insert(s: &mut Tour, _: usize, pos: usize, items: Vec<i32>) {
//!     for (i, item) in items.into_iter().enumerate() {
//!         s.cities.insert(pos + i, item);
//!     }
//! }
//!
//! // Create a 3-opt move with cuts at positions 2, 4, 6
//! // This creates 4 segments: [0..2), [2..4), [4..6), [6..)
//! let cuts = [
//!     CutPoint::new(0, 2),
//!     CutPoint::new(0, 4),
//!     CutPoint::new(0, 6),
//! ];
//! let reconnection = &THREE_OPT_RECONNECTIONS[3]; // Swap middle segments
//!
//! let m = KOptMove::<Tour, _, i32>::new(
//!     &cuts,
//!     reconnection,
//!     list_len,
//!     sublist_remove,
//!     sublist_insert,
//!     "cities",
//!     0,
//! );
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::k_opt_reconnection::KOptReconnection;
use super::Move;

/// A cut point in a route, defining where an edge is removed.
///
/// For k-opt, we have k cut points which divide the route into k+1 segments.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::CutPoint;
///
/// // Cut at position 5 in entity 0
/// let cut = CutPoint::new(0, 5);
/// assert_eq!(cut.entity_index(), 0);
/// assert_eq!(cut.position(), 5);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CutPoint {
    /// Entity (route/vehicle) index.
    entity_index: usize,
    /// Position in the route where the cut occurs.
    /// The edge between position-1 and position is removed.
    position: usize,
}

impl CutPoint {
    /// Creates a new cut point.
    #[inline]
    pub const fn new(entity_index: usize, position: usize) -> Self {
        Self {
            entity_index,
            position,
        }
    }

    /// Returns the entity index.
    #[inline]
    pub const fn entity_index(&self) -> usize {
        self.entity_index
    }

    /// Returns the position.
    #[inline]
    pub const fn position(&self) -> usize {
        self.position
    }
}

/// A k-opt move that removes k edges and reconnects segments.
///
/// This is the generalized k-opt move supporting k=2,3,4,5.
/// For k=2, this is equivalent to a 2-opt (segment reversal).
///
/// # Zero-Erasure Design
///
/// - Fixed array `[CutPoint; 5]` for up to 5 cuts (5-opt)
/// - Reference to static `&'static KOptReconnection`
/// - Typed function pointers for list operations
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `V` - The list element value type
#[derive(Clone)]
pub struct KOptMove<S, D, V> {
    /// Cut points (up to 5 for 5-opt).
    cuts: [CutPoint; 5],
    /// Number of actual cuts (k value).
    cut_count: u8,
    /// Reconnection pattern to apply.
    reconnection: &'static KOptReconnection,
    /// Get list length for an entity.
    list_len: fn(&S, usize) -> usize,
    /// Remove sublist [start, end), returns removed elements.
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    /// Insert elements at position.
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    /// Variable name.
    variable_name: &'static str,
    /// Descriptor index.
    descriptor_index: usize,
    /// Entity index (for intra-route moves).
    entity_index: usize,
    _phantom: PhantomData<(D, V)>,
}

impl<S, D, V: Debug> Debug for KOptMove<S, D, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cuts: Vec<_> = self.cuts[..self.cut_count as usize]
            .iter()
            .map(|c| c.position)
            .collect();
        f.debug_struct("KOptMove")
            .field("k", &self.cut_count)
            .field("entity", &self.entity_index)
            .field("cuts", &cuts)
            .field("reconnection", &self.reconnection)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, D, V> KOptMove<S, D, V> {
    /// Creates a new k-opt move.
    ///
    /// # Arguments
    ///
    /// * `cuts` - Slice of cut points (must be sorted by position for intra-route)
    /// * `reconnection` - How to reconnect the segments
    /// * `list_len` - Function to get list length
    /// * `sublist_remove` - Function to remove a range
    /// * `sublist_insert` - Function to insert elements
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    ///
    /// # Panics
    ///
    /// Panics if cuts is empty or has more than 5 elements.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cuts: &[CutPoint],
        reconnection: &'static KOptReconnection,
        list_len: fn(&S, usize) -> usize,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        assert!(!cuts.is_empty() && cuts.len() <= 5, "k must be 1-5");

        let mut cut_array = [CutPoint::default(); 5];
        for (i, cut) in cuts.iter().enumerate() {
            cut_array[i] = *cut;
        }

        // For now, assume intra-route (all cuts on same entity)
        let entity_index = cuts[0].entity_index;

        Self {
            cuts: cut_array,
            cut_count: cuts.len() as u8,
            reconnection,
            list_len,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            entity_index,
            _phantom: PhantomData,
        }
    }

    /// Returns the k value (number of edges removed).
    #[inline]
    pub fn k(&self) -> usize {
        self.cut_count as usize
    }

    /// Returns the cut points.
    #[inline]
    pub fn cuts(&self) -> &[CutPoint] {
        &self.cuts[..self.cut_count as usize]
    }

    /// Returns true if this is an intra-route move (all cuts on same entity).
    pub fn is_intra_route(&self) -> bool {
        let first = self.cuts[0].entity_index;
        self.cuts[..self.cut_count as usize]
            .iter()
            .all(|c| c.entity_index == first)
    }
}

impl<S, D, V> Move<S, D> for KOptMove<S, D, V>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        let k = self.cut_count as usize;

        // Must have at least 2 cuts for meaningful k-opt
        if k < 2 {
            return false;
        }

        // Verify reconnection pattern matches k
        if self.reconnection.k() != k {
            return false;
        }

        // For intra-route, verify cuts are sorted and within bounds
        let len = (self.list_len)(solution, self.entity_index);

        // Check cuts are valid positions
        for cut in &self.cuts[..k] {
            if cut.position > len {
                return false;
            }
        }

        // For intra-route, cuts must be strictly increasing
        if self.is_intra_route() {
            for i in 1..k {
                if self.cuts[i].position <= self.cuts[i - 1].position {
                    return false;
                }
            }
            // Need at least 1 element between cuts for meaningful segments
            // Actually, empty segments are allowed in some cases
        }

        true
    }

    fn do_move(&self, score_director: &mut D) {
        let k = self.cut_count as usize;
        let entity = self.entity_index;

        // Notify before change
        score_director.before_variable_changed(self.descriptor_index, entity, self.variable_name);

        // For intra-route k-opt, we need to:
        // 1. Extract all segments
        // 2. Reorder according to reconnection pattern
        // 3. Reverse segments as needed
        // 4. Rebuild the list

        // Calculate segment boundaries (segments are between consecutive cuts)
        // For k cuts at positions [p0, p1, ..., pk-1], we have k+1 segments:
        // Segment 0: [0, p0)
        // Segment 1: [p0, p1)
        // ...
        // Segment k: [pk-1, len)

        let solution = score_director.working_solution_mut();
        let len = (self.list_len)(solution, entity);

        // Extract all elements
        let all_elements = (self.sublist_remove)(solution, entity, 0, len);

        // Build segment boundaries
        let mut boundaries = Vec::with_capacity(k + 2);
        boundaries.push(0);
        for i in 0..k {
            boundaries.push(self.cuts[i].position);
        }
        boundaries.push(len);

        // Extract segments
        let mut segments: Vec<Vec<V>> = Vec::with_capacity(k + 1);
        for i in 0..=k {
            let start = boundaries[i];
            let end = boundaries[i + 1];
            segments.push(all_elements[start..end].to_vec());
        }

        // Reorder and reverse according to reconnection pattern
        let mut new_elements = Vec::with_capacity(len);
        for pos in 0..self.reconnection.segment_count() {
            let seg_idx = self.reconnection.segment_at(pos);
            let mut seg = segments[seg_idx].clone();
            if self.reconnection.should_reverse(seg_idx) {
                seg.reverse();
            }
            new_elements.extend(seg);
        }

        // Insert reordered elements back
        (self.sublist_insert)(
            score_director.working_solution_mut(),
            entity,
            0,
            new_elements.clone(),
        );

        // Notify after change
        score_director.after_variable_changed(self.descriptor_index, entity, self.variable_name);

        // Register undo - need to restore original order
        let sublist_remove = self.sublist_remove;
        let sublist_insert = self.sublist_insert;

        score_director.register_undo(Box::new(move |s: &mut S| {
            // Remove current elements
            let current_len = new_elements.len();
            let _ = sublist_remove(s, entity, 0, current_len);
            // Insert original elements
            sublist_insert(s, entity, 0, all_elements);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::k_opt_reconnection::THREE_OPT_RECONNECTIONS;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Tour {
        cities: Vec<i32>,
    }

    #[derive(Clone, Debug)]
    struct TspSolution {
        tours: Vec<Tour>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TspSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_tours(s: &TspSolution) -> &Vec<Tour> {
        &s.tours
    }
    fn get_tours_mut(s: &mut TspSolution) -> &mut Vec<Tour> {
        &mut s.tours
    }

    fn list_len(s: &TspSolution, entity_idx: usize) -> usize {
        s.tours.get(entity_idx).map_or(0, |t| t.cities.len())
    }
    fn sublist_remove(
        s: &mut TspSolution,
        entity_idx: usize,
        start: usize,
        end: usize,
    ) -> Vec<i32> {
        s.tours
            .get_mut(entity_idx)
            .map(|t| t.cities.drain(start..end).collect())
            .unwrap_or_default()
    }
    fn sublist_insert(s: &mut TspSolution, entity_idx: usize, pos: usize, items: Vec<i32>) {
        if let Some(t) = s.tours.get_mut(entity_idx) {
            for (i, item) in items.into_iter().enumerate() {
                t.cities.insert(pos + i, item);
            }
        }
    }

    fn create_director(
        tours: Vec<Tour>,
    ) -> SimpleScoreDirector<TspSolution, impl Fn(&TspSolution) -> SimpleScore> {
        let solution = TspSolution { tours, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Tour",
            "tours",
            get_tours,
            get_tours_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Tour", TypeId::of::<Tour>(), "tours").with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TspSolution", TypeId::of::<TspSolution>())
            .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn three_opt_swap_segments() {
        // Tour: [1, 2, 3, 4, 5, 6, 7, 8]
        // Cuts at positions 2, 4, 6 creates segments:
        //   Segment 0: [1, 2]
        //   Segment 1: [3, 4]
        //   Segment 2: [5, 6]
        //   Segment 3: [7, 8]
        // Pattern 3 (swap B and C, no reversal): [A, C, B, D]
        // Result: [1, 2, 5, 6, 3, 4, 7, 8]

        let tours = vec![Tour {
            cities: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }];
        let mut director = create_director(tours);

        let cuts = [
            CutPoint::new(0, 2),
            CutPoint::new(0, 4),
            CutPoint::new(0, 6),
        ];
        let reconnection = &THREE_OPT_RECONNECTIONS[3]; // [0,2,1,3] no reversal

        let m = KOptMove::<TspSolution, _, i32>::new(
            &cuts,
            reconnection,
            list_len,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        assert!(m.is_doable(&director));
        assert_eq!(m.k(), 3);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let cities = &recording.working_solution().tours[0].cities;
            assert_eq!(cities, &[1, 2, 5, 6, 3, 4, 7, 8]);

            recording.undo_changes();
        }

        let cities = &director.working_solution().tours[0].cities;
        assert_eq!(cities, &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn three_opt_reverse_segment() {
        // Tour: [1, 2, 3, 4, 5, 6]
        // Cuts at 2, 4 (only using 2 cuts for simpler test, but with 3-opt pattern)
        // Wait, 3-opt needs 3 cuts. Let me use proper 3 cuts.

        // Tour: [1, 2, 3, 4, 5, 6, 7, 8]
        // Cuts at 2, 4, 6
        // Pattern 0 (reverse B only): segments [A, B', C, D]
        // B = [3, 4], reversed = [4, 3]
        // Result: [1, 2, 4, 3, 5, 6, 7, 8]

        let tours = vec![Tour {
            cities: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }];
        let mut director = create_director(tours);

        let cuts = [
            CutPoint::new(0, 2),
            CutPoint::new(0, 4),
            CutPoint::new(0, 6),
        ];
        let reconnection = &THREE_OPT_RECONNECTIONS[0]; // Reverse B only

        let m = KOptMove::<TspSolution, _, i32>::new(
            &cuts,
            reconnection,
            list_len,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let cities = &recording.working_solution().tours[0].cities;
            assert_eq!(cities, &[1, 2, 4, 3, 5, 6, 7, 8]);

            recording.undo_changes();
        }

        let cities = &director.working_solution().tours[0].cities;
        assert_eq!(cities, &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn invalid_cuts_not_doable() {
        let tours = vec![Tour {
            cities: vec![1, 2, 3],
        }];
        let director = create_director(tours);

        // Cuts out of bounds
        let cuts = [
            CutPoint::new(0, 2),
            CutPoint::new(0, 4),
            CutPoint::new(0, 10), // Out of bounds
        ];
        let reconnection = &THREE_OPT_RECONNECTIONS[0];

        let m = KOptMove::<TspSolution, _, i32>::new(
            &cuts,
            reconnection,
            list_len,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn cuts_not_sorted_not_doable() {
        let tours = vec![Tour {
            cities: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }];
        let director = create_director(tours);

        // Cuts not in order
        let cuts = [
            CutPoint::new(0, 4),
            CutPoint::new(0, 2), // Out of order
            CutPoint::new(0, 6),
        ];
        let reconnection = &THREE_OPT_RECONNECTIONS[0];

        let m = KOptMove::<TspSolution, _, i32>::new(
            &cuts,
            reconnection,
            list_len,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        assert!(!m.is_doable(&director));
    }
}
