/* The gate's planning model, declared the way generated model code declares
one: a solution descriptor plus a `RuntimeModel` of list or scalar slots,
bound into a `SearchContext` and compiled from a `SolverConfig`. */

const SEED: u64 = 0x5EED;

/* Every neighborhood pins canonical (`original`) candidate order so its
candidate count and order are a fixed, seed-independent contract; the
compiled cursors are the same ones a randomly ordered solve pulls from. */
const CANONICAL: Option<SelectionOrder> = Some(SelectionOrder::Original);

#[derive(Clone, Debug)]
struct Vehicle {
    visits: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Plan {
    vehicles: Vec<Vehicle>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct PositionDistanceMeter;

impl CrossEntityDistanceMeter<Plan> for PositionDistanceMeter {
    fn distance(
        &self,
        _solution: &Plan,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64 {
        source_entity.abs_diff(destination_entity) as f64 * 100.0
            + source_position.abs_diff(destination_position) as f64
    }
}

type Context = SearchContext<Plan, usize, PositionDistanceMeter, PositionDistanceMeter>;
type Runner = CompiledRuntimeRunner<Plan, usize, PositionDistanceMeter, PositionDistanceMeter>;
type Neighborhood = CompiledNeighborhood<Plan, usize, PositionDistanceMeter, PositionDistanceMeter>;
type Slot = VariableSlot<Plan, usize, PositionDistanceMeter, PositionDistanceMeter>;

fn get_vehicles(plan: &Plan) -> &Vec<Vehicle> {
    &plan.vehicles
}

fn get_vehicles_mut(plan: &mut Plan) -> &mut Vec<Vehicle> {
    &mut plan.vehicles
}

fn descriptor(variables: &[VariableDescriptor]) -> SolutionDescriptor {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Vehicle",
        "vehicles",
        get_vehicles,
        get_vehicles_mut,
    ));
    let entity = variables.iter().cloned().fold(
        EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
            .with_extractor(extractor),
        EntityDescriptor::with_variable,
    );
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(entity)
}

fn plan_director(plan: Plan, variables: &[VariableDescriptor]) -> ScoreDirector<Plan, ()> {
    ScoreDirector::simple(plan, descriptor(variables), |plan, _| plan.vehicles.len())
}

/* Elements are numbered densely, vehicle by vehicle, so element index and
element value coincide. */
fn routed_plan(vehicle_count: usize, visits_per_vehicle: usize) -> Plan {
    Plan {
        vehicles: (0..vehicle_count)
            .map(|vehicle| Vehicle {
                visits: (0..visits_per_vehicle)
                    .map(|visit| vehicle * visits_per_vehicle + visit)
                    .collect(),
            })
            .collect(),
        score: None,
    }
}

fn entity_count(plan: &Plan) -> usize {
    plan.vehicles.len()
}

fn element_count(plan: &Plan) -> usize {
    plan.vehicles
        .iter()
        .map(|vehicle| vehicle.visits.len())
        .sum()
}

fn assigned_elements(plan: &Plan) -> Vec<usize> {
    plan.vehicles
        .iter()
        .flat_map(|vehicle| vehicle.visits.iter().copied())
        .collect()
}

fn list_len(plan: &Plan, entity: usize) -> usize {
    plan.vehicles[entity].visits.len()
}

fn list_get(plan: &Plan, entity: usize, position: usize) -> Option<usize> {
    plan.vehicles.get(entity)?.visits.get(position).copied()
}

fn list_set(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.vehicles[entity].visits[position] = value;
}

fn list_remove(plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
    let visits = &mut plan.vehicles[entity].visits;
    (position < visits.len()).then(|| visits.remove(position))
}

fn list_take(plan: &mut Plan, entity: usize, position: usize) -> usize {
    plan.vehicles[entity].visits.remove(position)
}

fn list_insert(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.vehicles[entity].visits.insert(position, value);
}

fn list_reverse(plan: &mut Plan, entity: usize, start: usize, end: usize) {
    plan.vehicles[entity].visits[start..end].reverse();
}

fn sublist_remove(plan: &mut Plan, entity: usize, start: usize, end: usize) -> Vec<usize> {
    plan.vehicles[entity].visits.drain(start..end).collect()
}

fn sublist_insert(plan: &mut Plan, entity: usize, position: usize, values: Vec<usize>) {
    plan.vehicles[entity]
        .visits
        .splice(position..position, values);
}

fn index_to_element(_plan: &Plan, index: usize) -> usize {
    index
}

fn element_source_key(_plan: &Plan, element: &usize) -> usize {
    *element
}

fn visits_slot() -> Slot {
    VariableSlot::List(ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned_elements,
        list_len,
        list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        list_take,
        list_insert,
        index_to_element,
        element_source_key,
        entity_count,
        PositionDistanceMeter,
        PositionDistanceMeter,
        "visits",
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ))
}

fn list_variables() -> [VariableDescriptor; 1] {
    [VariableDescriptor::list("visits")]
}

fn list_context() -> Context {
    SearchContext::new(
        descriptor(&list_variables()),
        RuntimeModel::new(vec![visits_slot()]),
        Some(SEED),
    )
    .expect("the list gate model must bind to its descriptor")
}

fn list_director(vehicle_count: usize, visits_per_vehicle: usize) -> ScoreDirector<Plan, ()> {
    plan_director(
        routed_plan(vehicle_count, visits_per_vehicle),
        &list_variables(),
    )
}

/* Scalar slots read and write one visit position as the variable value; an
empty visit list is an unassigned first variable. */
fn first_get(plan: &Plan, entity: usize, _variable_index: usize) -> Option<usize> {
    plan.vehicles[entity].visits.first().copied()
}

fn first_set(plan: &mut Plan, entity: usize, _variable_index: usize, value: Option<usize>) {
    let visits = &mut plan.vehicles[entity].visits;
    match (visits.is_empty(), value) {
        (false, Some(value)) => visits[0] = value,
        (true, Some(value)) => visits.push(value),
        (false, None) => {
            visits.remove(0);
        }
        (true, None) => {}
    }
}

fn second_get(plan: &Plan, entity: usize, _variable_index: usize) -> Option<usize> {
    plan.vehicles[entity].visits.get(1).copied()
}

fn second_set(plan: &mut Plan, entity: usize, _variable_index: usize, value: Option<usize>) {
    if let (Some(slot), Some(value)) = (plan.vehicles[entity].visits.get_mut(1), value) {
        *slot = value;
    }
}

fn first_slot(values: std::ops::Range<usize>) -> Slot {
    VariableSlot::Scalar(ScalarVariableSlot::new(
        0,
        0,
        "Vehicle",
        entity_count,
        "first",
        first_get,
        first_set,
        ValueSource::CountableRange {
            from: values.start,
            to: values.end,
        },
        false,
    ))
}

fn second_slot(values: std::ops::Range<usize>) -> Slot {
    VariableSlot::Scalar(ScalarVariableSlot::new(
        0,
        1,
        "Vehicle",
        entity_count,
        "second",
        second_get,
        second_set,
        ValueSource::CountableRange {
            from: values.start,
            to: values.end,
        },
        false,
    ))
}

fn scalar_variables(count: usize) -> Vec<VariableDescriptor> {
    [
        VariableDescriptor::genuine("first"),
        VariableDescriptor::genuine("second"),
    ][..count]
        .to_vec()
}

fn scalar_context(slots: Vec<Slot>) -> Context {
    let variables = scalar_variables(slots.len());
    SearchContext::new(descriptor(&variables), RuntimeModel::new(slots), Some(SEED))
        .expect("the scalar gate model must bind to its descriptor")
}

fn scalar_director(plan: Plan, variable_count: usize) -> ScoreDirector<Plan, ()> {
    plan_director(plan, &scalar_variables(variable_count))
}

fn unassigned_plan(vehicle_count: usize) -> Plan {
    Plan {
        vehicles: (0..vehicle_count)
            .map(|_| Vehicle { visits: Vec::new() })
            .collect(),
        score: None,
    }
}

fn target(variable_name: &str) -> VariableTargetConfig {
    VariableTargetConfig {
        entity_class: None,
        variable_name: Some(variable_name.to_string()),
    }
}

fn solver_config(phase: PhaseConfig) -> SolverConfig {
    SolverConfig {
        random_seed: Some(SEED),
        phases: vec![phase],
        ..SolverConfig::default()
    }
}

fn local_search_config(move_selector: MoveSelectorConfig) -> SolverConfig {
    solver_config(PhaseConfig::LocalSearch(LocalSearchConfig {
        move_selector: Some(move_selector),
        ..LocalSearchConfig::default()
    }))
}

fn compile_neighborhood(context: Context, move_selector: MoveSelectorConfig) -> Neighborhood {
    Runner::compile(&local_search_config(move_selector), context)
        .expect("the gate local-search configuration must compile")
        .into_local_search_neighborhood()
        .expect("the gate configuration's first phase is an acceptor/forager local search")
}
