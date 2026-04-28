#[derive(Clone, Debug)]
struct Shift {
    worker: Option<usize>,
    allowed_workers: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Schedule {
    workers: Vec<usize>,
    shifts: Vec<Shift>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Schedule {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_shifts(solution: &Schedule) -> &Vec<Shift> {
    &solution.shifts
}

fn get_shifts_mut(solution: &mut Schedule) -> &mut Vec<Shift> {
    &mut solution.shifts
}

fn shift_count(solution: &Schedule) -> usize {
    solution.shifts.len()
}

fn get_worker(solution: &Schedule, entity_index: usize, _variable_index: usize) -> Option<usize> {
    solution.shifts[entity_index].worker
}

fn set_worker(
    solution: &mut Schedule,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.shifts[entity_index].worker = value;
}

fn worker_count(solution: &Schedule, _provider_index: usize) -> usize {
    solution.workers.len()
}

fn allowed_workers(solution: &Schedule, entity_index: usize, _variable_index: usize) -> &[usize] {
    &solution.shifts[entity_index].allowed_workers
}

fn nearby_worker_candidates(
    solution: &Schedule,
    entity_index: usize,
    variable_index: usize,
) -> &[usize] {
    allowed_workers(solution, entity_index, variable_index)
}

fn nearby_shift_candidates(
    solution: &Schedule,
    entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    &solution.shifts[entity_index].allowed_workers
}

fn nearby_worker_value_distance(
    _solution: &Schedule,
    entity_index: usize,
    _variable_index: usize,
    value: usize,
) -> Option<f64> {
    Some(entity_index.abs_diff(value) as f64)
}

fn nearby_worker_entity_distance(
    _solution: &Schedule,
    left: usize,
    right: usize,
    _variable_index: usize,
) -> Option<f64> {
    Some(match (left, right) {
        (0, 1) => 0.0,
        (0, 2) => 1.0,
        (1, 2) => 0.5,
        _ => left.abs_diff(right) as f64,
    })
}

fn create_director(solution: Schedule) -> ScoreDirector<Schedule, ()> {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Shift",
        "shifts",
        get_shifts,
        get_shifts_mut,
    ));
    let descriptor = SolutionDescriptor::new("Schedule", TypeId::of::<Schedule>()).with_entity(
        EntityDescriptor::new("Shift", TypeId::of::<Shift>(), "shifts").with_extractor(extractor),
    );

    ScoreDirector::simple(solution, descriptor, |solution, _| solution.shifts.len())
}
