#[derive(Clone, Debug)]
struct Shift {
    worker: Option<usize>,
}

#[derive(Clone, Debug)]
struct Vehicle {
    visits: Vec<usize>,
}

#[derive(Clone, Debug)]
struct MixedPlan {
    shifts: Vec<Shift>,
    vehicles: Vec<Vehicle>,
    score: Option<SoftScore>,
}

impl PlanningSolution for MixedPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct NoopMeter;

impl CrossEntityDistanceMeter<MixedPlan> for NoopMeter {
    fn distance(
        &self,
        _solution: &MixedPlan,
        _src_entity: usize,
        _src_pos: usize,
        _dst_entity: usize,
        _dst_pos: usize,
    ) -> f64 {
        1.0
    }
}

fn get_shifts(solution: &MixedPlan) -> &Vec<Shift> {
    &solution.shifts
}

fn get_shifts_mut(solution: &mut MixedPlan) -> &mut Vec<Shift> {
    &mut solution.shifts
}

fn get_vehicles(solution: &MixedPlan) -> &Vec<Vehicle> {
    &solution.vehicles
}

fn get_vehicles_mut(solution: &mut MixedPlan) -> &mut Vec<Vehicle> {
    &mut solution.vehicles
}

fn get_worker_dyn(entity: &dyn std::any::Any) -> Option<usize> {
    entity
        .downcast_ref::<Shift>()
        .and_then(|shift| shift.worker)
}

fn set_worker_dyn(entity: &mut dyn std::any::Any, value: Option<usize>) {
    if let Some(shift) = entity.downcast_mut::<Shift>() {
        shift.worker = value;
    }
}

fn descriptor(include_scalar_binding: bool) -> SolutionDescriptor {
    let shift_descriptor =
        EntityDescriptor::new("Shift", TypeId::of::<Shift>(), "shifts").with_extractor(Box::new(
            EntityCollectionExtractor::new("Shift", "shifts", get_shifts, get_shifts_mut),
        ));
    let shift_descriptor = if include_scalar_binding {
        shift_descriptor.with_variable(
            VariableDescriptor::genuine("worker")
                .with_allows_unassigned(true)
                .with_value_range("shifts")
                .with_usize_accessors(get_worker_dyn, set_worker_dyn),
        )
    } else {
        shift_descriptor
    };

    SolutionDescriptor::new("MixedPlan", TypeId::of::<MixedPlan>())
        .with_entity(shift_descriptor)
        .with_entity(
            EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Vehicle",
                    "vehicles",
                    get_vehicles,
                    get_vehicles_mut,
                )),
            ),
        )
}

fn create_director(
    solution: MixedPlan,
    descriptor: SolutionDescriptor,
) -> ScoreDirector<MixedPlan, ()> {
    ScoreDirector::simple(
        solution,
        descriptor,
        |solution, descriptor_index| match descriptor_index {
            0 => solution.shifts.len(),
            1 => solution.vehicles.len(),
            _ => 0,
        },
    )
}

fn shift_count(solution: &MixedPlan) -> usize {
    solution.shifts.len()
}

fn get_worker(solution: &MixedPlan, entity_index: usize, _variable_index: usize) -> Option<usize> {
    solution.shifts[entity_index].worker
}

fn set_worker(
    solution: &mut MixedPlan,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.shifts[entity_index].worker = value;
}

fn worker_candidate_values(
    _solution: &MixedPlan,
    _entity_index: usize,
    _variable_index: usize,
) -> &'static [usize] {
    &[0, 1]
}

fn worker_count(solution: &MixedPlan, _provider_index: usize) -> usize {
    solution.shifts.len().max(1)
}

fn vehicle_count(solution: &MixedPlan) -> usize {
    solution.vehicles.len()
}

fn list_len(solution: &MixedPlan, entity_index: usize) -> usize {
    solution.vehicles[entity_index].visits.len()
}

fn list_remove(solution: &mut MixedPlan, entity_index: usize, pos: usize) -> Option<usize> {
    let visits = &mut solution.vehicles.get_mut(entity_index)?.visits;
    if pos < visits.len() {
        Some(visits.remove(pos))
    } else {
        None
    }
}

fn list_insert(solution: &mut MixedPlan, entity_index: usize, pos: usize, value: usize) {
    solution.vehicles[entity_index].visits.insert(pos, value);
}

fn list_get(solution: &MixedPlan, entity_index: usize, pos: usize) -> Option<usize> {
    solution.vehicles[entity_index].visits.get(pos).copied()
}

fn list_set(solution: &mut MixedPlan, entity_index: usize, pos: usize, value: usize) {
    solution.vehicles[entity_index].visits[pos] = value;
}

fn list_reverse(solution: &mut MixedPlan, entity_index: usize, start: usize, end: usize) {
    solution.vehicles[entity_index].visits[start..end].reverse();
}

fn sublist_remove(
    solution: &mut MixedPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) -> Vec<usize> {
    solution.vehicles[entity_index]
        .visits
        .drain(start..end)
        .collect()
}

fn sublist_insert(solution: &mut MixedPlan, entity_index: usize, pos: usize, values: Vec<usize>) {
    solution.vehicles[entity_index]
        .visits
        .splice(pos..pos, values);
}

fn ruin_remove(solution: &mut MixedPlan, entity_index: usize, pos: usize) -> usize {
    solution.vehicles[entity_index].visits.remove(pos)
}

fn ruin_insert(solution: &mut MixedPlan, entity_index: usize, pos: usize, value: usize) {
    solution.vehicles[entity_index].visits.insert(pos, value);
}

fn assigned_visits(solution: &MixedPlan) -> Vec<usize> {
    solution
        .vehicles
        .iter()
        .flat_map(|vehicle| vehicle.visits.iter().copied())
        .collect()
}

fn visit_count(solution: &MixedPlan) -> usize {
    assigned_visits(solution).len()
}

fn construction_list_remove(solution: &mut MixedPlan, entity_index: usize, pos: usize) -> usize {
    solution.vehicles[entity_index].visits.remove(pos)
}

fn index_to_visit(solution: &MixedPlan, idx: usize) -> usize {
    assigned_visits(solution).get(idx).copied().unwrap_or(idx)
}

fn scalar_context() -> ScalarVariableContext<MixedPlan> {
    ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        true,
    )
    .with_candidate_values(worker_candidate_values)
}

fn list_context() -> ListVariableContext<MixedPlan, usize, NoopMeter, NoopMeter> {
    ListVariableContext::new(
        "Vehicle",
        visit_count,
        assigned_visits,
        list_len,
        list_remove,
        construction_list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        index_to_visit,
        vehicle_count,
        NoopMeter,
        NoopMeter,
        "visits",
        1,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

fn scalar_only_model() -> ModelContext<MixedPlan, usize, NoopMeter, NoopMeter> {
    ModelContext::new(vec![VariableContext::Scalar(scalar_context())])
}

fn list_only_model() -> ModelContext<MixedPlan, usize, NoopMeter, NoopMeter> {
    ModelContext::new(vec![VariableContext::List(list_context())])
}

fn mixed_model() -> ModelContext<MixedPlan, usize, NoopMeter, NoopMeter> {
    ModelContext::new(vec![
        VariableContext::Scalar(scalar_context()),
        VariableContext::List(list_context()),
    ])
}

fn empty_model() -> ModelContext<MixedPlan, usize, NoopMeter, NoopMeter> {
    ModelContext::new(vec![])
}
