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

struct NamedConstraint {
    constraint_ref: ConstraintRef,
    is_hard: bool,
}

impl Default for NamedConstraint {
    fn default() -> Self {
        Self {
            constraint_ref: ConstraintRef::new("", ""),
            is_hard: false,
        }
    }
}

impl IncrementalConstraint<MixedPlan, SoftScore> for NamedConstraint {
    fn evaluate(&self, _solution: &MixedPlan) -> SoftScore {
        SoftScore::of(0)
    }

    fn match_count(&self, _solution: &MixedPlan) -> usize {
        0
    }

    fn initialize(&mut self, _solution: &MixedPlan) -> SoftScore {
        SoftScore::of(0)
    }

    fn on_insert(
        &mut self,
        _solution: &MixedPlan,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> SoftScore {
        SoftScore::of(0)
    }

    fn on_retract(
        &mut self,
        _solution: &MixedPlan,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> SoftScore {
        SoftScore::of(0)
    }

    fn reset(&mut self) {}

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    fn get_matches<'a>(
        &'a self,
        _solution: &MixedPlan,
    ) -> Vec<DetailedConstraintMatch<'a, SoftScore>> {
        Vec::new()
    }
}

fn create_director_with_constraint(
    solution: MixedPlan,
    descriptor: SolutionDescriptor,
    constraint_name: &'static str,
    is_hard: bool,
) -> ScoreDirector<MixedPlan, (NamedConstraint,)> {
    create_director_with_constraint_ref(solution, descriptor, "", constraint_name, is_hard)
}

fn create_director_with_constraint_ref(
    solution: MixedPlan,
    descriptor: SolutionDescriptor,
    constraint_package: &'static str,
    constraint_name: &'static str,
    is_hard: bool,
) -> ScoreDirector<MixedPlan, (NamedConstraint,)> {
    ScoreDirector::with_descriptor(
        solution,
        (NamedConstraint {
            constraint_ref: ConstraintRef::new(constraint_package, constraint_name),
            is_hard,
        },),
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

fn nearby_shift_candidates(
    _solution: &MixedPlan,
    _entity_index: usize,
    _variable_index: usize,
) -> &'static [usize] {
    &[0, 1]
}

fn nearby_worker_value_distance(
    _solution: &MixedPlan,
    entity_index: usize,
    _variable_index: usize,
    value: usize,
) -> Option<f64> {
    Some(entity_index.abs_diff(value) as f64)
}

fn nearby_worker_entity_distance(
    _solution: &MixedPlan,
    left_entity_index: usize,
    right_entity_index: usize,
    _variable_index: usize,
) -> Option<f64> {
    Some(left_entity_index.abs_diff(right_entity_index) as f64)
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

fn scalar_slot() -> ScalarVariableSlot<MixedPlan> {
    ScalarVariableSlot::new(
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

fn nearby_scalar_slot() -> ScalarVariableSlot<MixedPlan> {
    scalar_slot()
        .with_nearby_value_candidates(worker_candidate_values)
        .with_nearby_value_distance_meter(nearby_worker_value_distance)
        .with_nearby_entity_candidates(nearby_shift_candidates)
        .with_nearby_entity_distance_meter(nearby_worker_entity_distance)
}

fn required_unassigned_shift(solution: &MixedPlan, entity_index: usize) -> bool {
    solution.shifts[entity_index].worker.is_none()
}

fn worker_capacity_key(
    _solution: &MixedPlan,
    _entity_index: usize,
    worker: usize,
) -> Option<usize> {
    Some(worker)
}

fn list_slot() -> ListVariableSlot<MixedPlan, usize, NoopMeter, NoopMeter> {
    ListVariableSlot::new(
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

fn scalar_only_model() -> RuntimeModel<MixedPlan, usize, NoopMeter, NoopMeter> {
    RuntimeModel::new(vec![VariableSlot::Scalar(scalar_slot())])
}

fn nearby_scalar_only_model() -> RuntimeModel<MixedPlan, usize, NoopMeter, NoopMeter> {
    RuntimeModel::new(vec![VariableSlot::Scalar(nearby_scalar_slot())])
}

fn assignment_scalar_model() -> RuntimeModel<MixedPlan, usize, NoopMeter, NoopMeter> {
    let scalar_slot = scalar_slot();
    let groups = bind_scalar_groups(
        vec![
            ScalarGroup::assignment(
                "worker_assignment",
                ScalarTarget::from_descriptor_index(0, "worker"),
            )
            .with_required_entity(required_unassigned_shift)
            .with_capacity_key(worker_capacity_key)
            .with_limits(ScalarGroupLimits {
                max_moves_per_step: Some(7),
                ..ScalarGroupLimits::new()
            }),
        ],
        &[scalar_slot],
    );
    RuntimeModel::new(vec![VariableSlot::Scalar(scalar_slot)]).with_scalar_groups(groups)
}

fn list_only_model() -> RuntimeModel<MixedPlan, usize, NoopMeter, NoopMeter> {
    RuntimeModel::new(vec![VariableSlot::List(list_slot())])
}

fn mixed_model() -> RuntimeModel<MixedPlan, usize, NoopMeter, NoopMeter> {
    RuntimeModel::new(vec![
        VariableSlot::Scalar(scalar_slot()),
        VariableSlot::List(list_slot()),
    ])
}

fn empty_model() -> RuntimeModel<MixedPlan, usize, NoopMeter, NoopMeter> {
    RuntimeModel::new(vec![])
}
