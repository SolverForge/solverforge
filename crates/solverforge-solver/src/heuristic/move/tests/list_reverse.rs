//! Tests for ListReverseMove operations.

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
fn list_reverse(s: &mut TspSolution, entity_idx: usize, start: usize, end: usize) {
    if let Some(t) = s.tours.get_mut(entity_idx) {
        t.cities[start..end].reverse();
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
fn reverse_segment() {
    let tours = vec![Tour {
        cities: vec![1, 2, 3, 4, 5],
    }];
    let mut director = create_director(tours);

    let m = ListReverseMove::<TspSolution, i32>::new(0, 1, 4, list_len, list_reverse, "cities", 0);

    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        let cities = &recording.working_solution().tours[0].cities;
        assert_eq!(cities, &[1, 4, 3, 2, 5]);

        recording.undo_changes();
    }

    let cities = &director.working_solution().tours[0].cities;
    assert_eq!(cities, &[1, 2, 3, 4, 5]);
}

#[test]
fn reverse_entire_list() {
    let tours = vec![Tour {
        cities: vec![1, 2, 3, 4],
    }];
    let mut director = create_director(tours);

    let m = ListReverseMove::<TspSolution, i32>::new(0, 0, 4, list_len, list_reverse, "cities", 0);

    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        let cities = &recording.working_solution().tours[0].cities;
        assert_eq!(cities, &[4, 3, 2, 1]);

        recording.undo_changes();
    }

    let cities = &director.working_solution().tours[0].cities;
    assert_eq!(cities, &[1, 2, 3, 4]);
}

#[test]
fn single_element_not_doable() {
    let tours = vec![Tour {
        cities: vec![1, 2, 3],
    }];
    let director = create_director(tours);

    let m = ListReverseMove::<TspSolution, i32>::new(0, 1, 2, list_len, list_reverse, "cities", 0);

    assert!(!m.is_doable(&director));
}

#[test]
fn out_of_bounds_not_doable() {
    let tours = vec![Tour {
        cities: vec![1, 2, 3],
    }];
    let director = create_director(tours);

    let m = ListReverseMove::<TspSolution, i32>::new(0, 1, 10, list_len, list_reverse, "cities", 0);

    assert!(!m.is_doable(&director));
}
