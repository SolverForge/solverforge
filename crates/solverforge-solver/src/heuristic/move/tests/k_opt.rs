//! Tests for KOptMove and k-opt reconnection enumeration.

use super::*;
use crate::heuristic::r#move::k_opt_reconnection::{
    enumerate_reconnections, THREE_OPT_RECONNECTIONS,
};

// =============================================================================
// KOptMove tests
// =============================================================================

mod k_opt_tests {
    use super::*;

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
        let tours = vec![Tour {
            cities: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }];
        let mut director = create_director(tours);

        let cuts = [
            CutPoint::new(0, 2),
            CutPoint::new(0, 4),
            CutPoint::new(0, 6),
        ];
        let reconnection = &THREE_OPT_RECONNECTIONS[3];

        let m = KOptMove::<TspSolution, i32>::new(
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
        let tours = vec![Tour {
            cities: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }];
        let mut director = create_director(tours);

        let cuts = [
            CutPoint::new(0, 2),
            CutPoint::new(0, 4),
            CutPoint::new(0, 6),
        ];
        let reconnection = &THREE_OPT_RECONNECTIONS[0];

        let m = KOptMove::<TspSolution, i32>::new(
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

        let cuts = [
            CutPoint::new(0, 2),
            CutPoint::new(0, 4),
            CutPoint::new(0, 10),
        ];
        let reconnection = &THREE_OPT_RECONNECTIONS[0];

        let m = KOptMove::<TspSolution, i32>::new(
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

        let cuts = [
            CutPoint::new(0, 4),
            CutPoint::new(0, 2),
            CutPoint::new(0, 6),
        ];
        let reconnection = &THREE_OPT_RECONNECTIONS[0];

        let m = KOptMove::<TspSolution, i32>::new(
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

// =============================================================================
// KOptReconnection tests
// =============================================================================

mod k_opt_reconnection_tests {
    use super::*;

    #[test]
    fn enumerate_2opt_count() {
        let patterns = enumerate_reconnections(2);
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].should_reverse(1));
    }

    #[test]
    fn enumerate_3opt_count() {
        let patterns = enumerate_reconnections(3);
        assert_eq!(patterns.len(), 7);
    }

    #[test]
    fn enumerate_matches_static_3opt() {
        let dynamic = enumerate_reconnections(3);
        assert_eq!(dynamic.len(), THREE_OPT_RECONNECTIONS.len());
    }

    #[test]
    fn enumerate_4opt_count() {
        let patterns = enumerate_reconnections(4);
        assert_eq!(patterns.len(), 47);
    }

    #[test]
    fn enumerate_5opt_count() {
        let patterns = enumerate_reconnections(5);
        assert_eq!(patterns.len(), 383);
    }
}
