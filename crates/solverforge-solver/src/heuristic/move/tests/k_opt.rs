// Tests for KOptMove and k-opt reconnection enumeration.

use super::*;
use crate::heuristic::r#move::k_opt_reconnection::THREE_OPT_RECONNECTIONS;

/* =============================================================================
KOptMove tests
=============================================================================
*/

mod k_opt_tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Tour {
        cities: Vec<i32>,
    }

    #[derive(Clone, Debug)]
    struct TspSolution {
        tours: Vec<Tour>,
        score: Option<SoftScore>,
    }

    impl PlanningSolution for TspSolution {
        type Score = SoftScore;
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
    fn list_get(s: &TspSolution, entity_idx: usize, pos: usize) -> Option<i32> {
        s.tours
            .get(entity_idx)
            .and_then(|t| t.cities.get(pos))
            .copied()
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

    fn create_director(tours: Vec<Tour>) -> ScoreDirector<TspSolution, ()> {
        let solution = TspSolution { tours, score: None };
        let extractor = Box::new(EntityCollectionExtractor::new(
            "Tour",
            "tours",
            get_tours,
            get_tours_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Tour", TypeId::of::<Tour>(), "tours").with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TspSolution", TypeId::of::<TspSolution>())
            .with_entity(entity_desc);
        ScoreDirector::simple(solution, descriptor, |s, _| s.tours.len())
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
            list_get,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        assert!(m.is_doable(&director));
        assert_eq!(m.k(), 3);

        {
            let mut recording = SnapshotDirector::new(&mut director);
            m.do_move(&mut recording);

            let cities = &director.working_solution().tours[0].cities;
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
            list_get,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        assert!(m.is_doable(&director));

        {
            let mut recording = SnapshotDirector::new(&mut director);
            m.do_move(&mut recording);

            let cities = &director.working_solution().tours[0].cities;
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
            list_get,
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
            list_get,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        assert!(!m.is_doable(&director));
    }

    /// The scratch trial pair must (a) restore arbitrary-length routes
    /// exactly and (b) reuse the retired journal so inlined capacity
    /// survives across trials.
    #[test]
    fn scratch_trial_pair_restores_and_recycles_the_journal() {
        let tours = vec![Tour {
            cities: (1..=30).collect::<Vec<i32>>(),
        }];
        let mut director = create_director(tours);
        let baseline = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30,
        ];

        let cuts = [
            CutPoint::new(0, 4),
            CutPoint::new(0, 12),
            CutPoint::new(0, 22),
        ];
        let reconnection = &THREE_OPT_RECONNECTIONS[3];
        let m = KOptMove::<TspSolution, i32>::new(
            &cuts,
            reconnection,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        let mut journal_pool = Vec::new();
        for _ in 0..3 {
            assert!(m.is_doable(&director));
            {
                let score_state = director.snapshot_score_state();
                let undo = m.do_move_with_scratch(&mut director, &mut journal_pool);
                let moved = &director.working_solution().tours[0].cities;
                assert_ne!(moved, &baseline, "the k-opt move must reorder segments");
                moved
                    .iter()
                    .try_for_each(|city| {
                        if baseline.contains(city) {
                            Ok(())
                        } else {
                            Err(())
                        }
                    })
                    .expect("the moved route keeps the same city set");
                m.undo_move_with_scratch(&mut director, undo, &mut journal_pool);
                director.restore_score_state(score_state);
            }
            assert_eq!(
                &director.working_solution().tours[0].cities,
                &baseline,
                "each trial must restore the route exactly"
            );
            assert!(
                !journal_pool.is_empty(),
                "the trial pair must retire a journal for the next trial"
            );
        }
    }
}
