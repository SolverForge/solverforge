/* The route gate model: stops in pick-up and drop-off pairs (element
groups) on vehicle lists, with list shadows and a cascading arrival time,
and constraints over every stop row. Declared by hand, the way generated
model code declares one. */

use solverforge_config::ListRuinMoveSelectorConfig;
use solverforge_core::domain::CascadePropagation;
use solverforge_scoring::Director;

const ROUTE_VEHICLES: usize = 12;
const ROUTE_PAIRS_PER_VEHICLE: usize = 10;
const ROUTE_CHANGES: usize = 2_048;
const GROUP_RUIN_MOVES: usize = 4;

#[derive(Clone, Debug)]
struct RouteVehicle {
    visits: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RouteStop {
    pair: usize,
    duration: i64,
    ready: i64,
    due: i64,
    vehicle: Option<usize>,
    previous: Option<usize>,
    next: Option<usize>,
    arrival: Option<i64>,
}

#[derive(Clone, Debug)]
struct RoutePlan {
    vehicles: Vec<RouteVehicle>,
    stops: Vec<RouteStop>,
    score: Option<HardSoftScore>,
}

// Stop rows are a fact collection, reported past the one entity descriptor.
const ROUTE_STOP_SOURCE: usize = 1;

fn route_travel(from: usize, to: usize) -> i64 {
    ((from * 31 + to * 17) % 23) as i64 + 1
}

impl RoutePlan {
    fn end(&self, stop: usize) -> Option<i64> {
        let stop = &self.stops[stop];
        stop.arrival.map(|arrival| arrival + stop.duration)
    }

    fn compute_arrival(&self, stop: usize) -> Option<i64> {
        let ready = self.stops[stop].ready;
        match self.stops[stop].previous {
            Some(previous) => Some(ready.max(self.end(previous)? + route_travel(previous, stop))),
            None => Some(ready.max(route_travel(stop, stop))),
        }
    }

    fn write_structure(&mut self, vehicle: usize) {
        let visits = std::mem::take(&mut self.vehicles[vehicle].visits);
        for (position, &stop) in visits.iter().enumerate() {
            let stop = &mut self.stops[stop];
            stop.vehicle = Some(vehicle);
            stop.previous = position.checked_sub(1).map(|previous| visits[previous]);
            stop.next = visits.get(position + 1).copied();
        }
        self.vehicles[vehicle].visits = visits;
    }
}

impl PlanningSolution for RoutePlan {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }

    fn update_entity_shadows(&mut self, descriptor_index: usize, entity_index: usize) {
        self.update_list_structure_shadows(descriptor_index, entity_index);
        let changed = self.vehicles[entity_index].visits.clone();
        CascadePropagation::new(&*self).propagate(self, changed, |_, _| {});
    }

    fn update_all_shadows(&mut self) {
        for stop in &mut self.stops {
            stop.vehicle = None;
            stop.previous = None;
            stop.next = None;
        }
        for vehicle in 0..self.vehicles.len() {
            self.write_structure(vehicle);
        }
        CascadePropagation::refresh_all(self);
    }

    fn list_shadow_elements(
        &self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Option<(usize, &[usize])> {
        (descriptor_index == 0)
            .then(|| self.vehicles.get(entity_index))
            .flatten()
            .map(|vehicle| (ROUTE_STOP_SOURCE, vehicle.visits.as_slice()))
    }

    fn clear_list_shadows(&mut self, element: usize) {
        let stop = &mut self.stops[element];
        stop.vehicle = None;
        stop.previous = None;
        stop.next = None;
    }

    fn list_shadow_source(collection: &str) -> Option<usize> {
        (collection == "stops").then_some(ROUTE_STOP_SOURCE)
    }

    const HAS_CASCADING_SHADOWS: bool = true;

    fn cascading_element_count(&self) -> usize {
        self.stops.len()
    }

    fn cascading_assigned(&self, element: usize) -> bool {
        self.stops[element].vehicle.is_some()
    }

    fn cascading_next(&self, element: usize) -> Option<usize> {
        self.stops[element].next
    }

    // The structure hook exists from the candidate tree on; a baseline
    // without it rescores every element of a changed owner.
    #[cfg(feature = "candidate")]
    fn list_structure_unchanged(
        &self,
        element: usize,
        owner_descriptor: usize,
        owner_index: usize,
        _index: usize,
        previous: Option<usize>,
        next: Option<usize>,
    ) -> bool {
        let stop = &self.stops[element];
        owner_descriptor == 0
            && stop.vehicle == Some(owner_index)
            && stop.previous == previous
            && stop.next == next
    }

    fn cascading_unset(&self, element: usize) -> bool {
        self.stops[element].arrival.is_none()
    }

    fn update_cascading_shadow<W: FnOnce(&Self)>(
        &mut self,
        element: usize,
        computable: bool,
        before_write: W,
    ) -> bool {
        let arrival = if computable {
            self.compute_arrival(element)
        } else {
            None
        };
        if self.stops[element].arrival == arrival {
            return false;
        }
        before_write(self);
        self.stops[element].arrival = arrival;
        true
    }

    fn update_list_structure_shadows(&mut self, descriptor_index: usize, entity_index: usize) {
        if descriptor_index == 0 && entity_index < self.vehicles.len() {
            self.write_structure(entity_index);
        }
    }
}

impl CrossEntityDistanceMeter<RoutePlan> for PositionDistanceMeter {
    fn distance(
        &self,
        _solution: &RoutePlan,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64 {
        source_entity.abs_diff(destination_entity) as f64 * 100.0
            + source_position.abs_diff(destination_position) as f64
    }
}

type RouteContext = SearchContext<RoutePlan, usize, PositionDistanceMeter, PositionDistanceMeter>;
type RouteRunner =
    CompiledRuntimeRunner<RoutePlan, usize, PositionDistanceMeter, PositionDistanceMeter>;
type RouteNeighborhood =
    CompiledNeighborhood<RoutePlan, usize, PositionDistanceMeter, PositionDistanceMeter>;
type RouteSlot = VariableSlot<RoutePlan, usize, PositionDistanceMeter, PositionDistanceMeter>;

fn route_stops(plan: &RoutePlan) -> &[RouteStop] {
    plan.stops.as_slice()
}

/* Rows of every stop: late arrivals (hard), split pairs (hard, a join on
the pair), each route's service load (grouped), and arrival times (soft). */
fn route_constraints() -> impl ConstraintSet<RoutePlan, HardSoftScore> {
    let stops = || {
        source(
            route_stops as fn(&RoutePlan) -> &[RouteStop],
            ChangeSource::Descriptor(ROUTE_STOP_SOURCE),
        )
    };
    let factory = ConstraintFactory::<RoutePlan, HardSoftScore>::new;
    (
        factory()
            .for_each(stops())
            .filter(|stop: &RouteStop| stop.arrival.is_some_and(|arrival| arrival > stop.due))
            .penalize(|stop: &RouteStop| {
                HardSoftScore::of_hard(stop.arrival.unwrap_or(0) - stop.due)
            })
            .named("Late"),
        factory()
            .for_each(stops())
            .join((
                stops(),
                equal_bi(|stop: &RouteStop| stop.pair, |other: &RouteStop| other.pair),
            ))
            .filter(|stop: &RouteStop, other: &RouteStop| {
                stop.duration < other.duration && stop.vehicle != other.vehicle
            })
            .penalize(HardSoftScore::of_hard(1))
            .named("Split pair"),
        factory()
            .for_each(stops())
            .group_by(
                |stop: &RouteStop| stop.vehicle.unwrap_or(usize::MAX),
                sum(|stop: &RouteStop| stop.duration),
            )
            .penalize(|_vehicle: &usize, load: &i64| HardSoftScore::of_soft(load * load / 64))
            .named("Load"),
        factory()
            .for_each(stops())
            .penalize(|stop: &RouteStop| HardSoftScore::of_soft(stop.arrival.unwrap_or(1_000)))
            .named("Arrival"),
    )
}

/* Pair `p` is stops `2p` (pick-up) and `2p + 1` (drop-off), one vehicle's
pairs in order; the drop-off is one unit longer, which orients the pair
join. */
fn route_plan() -> RoutePlan {
    let pairs = ROUTE_VEHICLES * ROUTE_PAIRS_PER_VEHICLE;
    let stops = (0..2 * pairs)
        .map(|stop| {
            let pair = stop / 2;
            let ready = ((pair * 37) % 90) as i64;
            RouteStop {
                pair,
                duration: (pair % 4) as i64 + 2 + (stop % 2) as i64,
                ready,
                due: ready + 160 + (pair % 7) as i64 * 20,
                vehicle: None,
                previous: None,
                next: None,
                arrival: None,
            }
        })
        .collect();
    let vehicles = (0..ROUTE_VEHICLES)
        .map(|vehicle| RouteVehicle {
            visits: (0..2 * ROUTE_PAIRS_PER_VEHICLE)
                .map(|offset| vehicle * 2 * ROUTE_PAIRS_PER_VEHICLE + offset)
                .collect(),
        })
        .collect();
    let mut plan = RoutePlan {
        vehicles,
        stops,
        score: None,
    };
    plan.update_all_shadows();
    plan
}

fn route_vehicles(plan: &RoutePlan) -> &Vec<RouteVehicle> {
    &plan.vehicles
}

fn route_vehicles_mut(plan: &mut RoutePlan) -> &mut Vec<RouteVehicle> {
    &mut plan.vehicles
}

fn route_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Vehicle",
        "vehicles",
        route_vehicles,
        route_vehicles_mut,
    ));
    let entity = EntityDescriptor::new("Vehicle", TypeId::of::<RouteVehicle>(), "vehicles")
        .with_extractor(extractor)
        .with_variable(VariableDescriptor::list("visits"));
    SolutionDescriptor::new("RoutePlan", TypeId::of::<RoutePlan>()).with_entity(entity)
}

fn route_director(
) -> ScoreDirector<RoutePlan, impl ConstraintSet<RoutePlan, HardSoftScore>, SolutionDescriptor> {
    ScoreDirector::with_descriptor(
        route_plan(),
        route_constraints(),
        route_descriptor(),
        |plan, _| plan.vehicles.len(),
    )
}

fn route_entity_count(plan: &RoutePlan) -> usize {
    plan.vehicles.len()
}

fn route_element_count(plan: &RoutePlan) -> usize {
    plan.stops.len()
}

fn route_assigned(plan: &RoutePlan) -> Vec<usize> {
    plan.vehicles
        .iter()
        .flat_map(|vehicle| vehicle.visits.iter().copied())
        .collect()
}

fn route_list_len(plan: &RoutePlan, entity: usize) -> usize {
    plan.vehicles[entity].visits.len()
}

fn route_list_get(plan: &RoutePlan, entity: usize, position: usize) -> Option<usize> {
    plan.vehicles.get(entity)?.visits.get(position).copied()
}

fn route_list_set(plan: &mut RoutePlan, entity: usize, position: usize, value: usize) {
    plan.vehicles[entity].visits[position] = value;
}

fn route_list_remove(plan: &mut RoutePlan, entity: usize, position: usize) -> Option<usize> {
    let visits = &mut plan.vehicles[entity].visits;
    (position < visits.len()).then(|| visits.remove(position))
}

fn route_list_take(plan: &mut RoutePlan, entity: usize, position: usize) -> usize {
    plan.vehicles[entity].visits.remove(position)
}

fn route_list_insert(plan: &mut RoutePlan, entity: usize, position: usize, value: usize) {
    plan.vehicles[entity].visits.insert(position, value);
}

fn route_list_reverse(plan: &mut RoutePlan, entity: usize, start: usize, end: usize) {
    plan.vehicles[entity].visits[start..end].reverse();
}

fn route_sublist_remove(
    plan: &mut RoutePlan,
    entity: usize,
    start: usize,
    end: usize,
) -> Vec<usize> {
    plan.vehicles[entity].visits.drain(start..end).collect()
}

fn route_sublist_insert(plan: &mut RoutePlan, entity: usize, position: usize, values: Vec<usize>) {
    plan.vehicles[entity]
        .visits
        .splice(position..position, values);
}

fn route_index_to_element(_plan: &RoutePlan, index: usize) -> usize {
    index
}

fn route_source_key(_plan: &RoutePlan, element: &usize) -> usize {
    *element
}

fn route_pair(plan: &RoutePlan, stop: usize, members: &mut Vec<usize>) {
    let pair = plan.stops[stop].pair;
    members.extend([2 * pair, 2 * pair + 1]);
}

fn route_slot() -> RouteSlot {
    VariableSlot::List(
        ListVariableSlot::new(
            "Vehicle",
            route_element_count,
            route_assigned,
            route_list_len,
            route_list_remove,
            route_list_insert,
            route_list_get,
            route_list_set,
            route_list_reverse,
            route_sublist_remove,
            route_sublist_insert,
            route_list_take,
            route_list_insert,
            route_index_to_element,
            route_source_key,
            route_entity_count,
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
        )
        .with_element_group_fn(Some(route_pair)),
    )
}

fn route_context() -> RouteContext {
    SearchContext::new(
        route_descriptor(),
        RuntimeModel::new(vec![route_slot()]),
        Some(SEED),
    )
    .expect("the route gate model must bind to its descriptor")
}
