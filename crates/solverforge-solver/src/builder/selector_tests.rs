use std::any::TypeId;

use solverforge_config::{
    AcceptorConfig, ChangeMoveConfig, ForagerConfig, LateAcceptanceConfig, LocalSearchConfig,
    MoveSelectorConfig, PickEarlyType, SelectedCountLimitMoveSelectorConfig, SwapMoveConfig,
    UnionMoveSelectorConfig, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::*;
use crate::builder::list_selector::ListLeafSelector;
use crate::builder::standard_selector::StandardLeafSelector;
use crate::builder::{ListVariableContext, ScalarVariableContext, ValueSource, VariableContext};
use crate::CrossEntityDistanceMeter;

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

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("MixedPlan", TypeId::of::<MixedPlan>())
        .with_entity(
            EntityDescriptor::new("Shift", TypeId::of::<Shift>(), "shifts").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Shift",
                    "shifts",
                    get_shifts,
                    get_shifts_mut,
                )),
            ),
        )
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

fn create_director(solution: MixedPlan) -> ScoreDirector<MixedPlan, ()> {
    ScoreDirector::simple(solution, descriptor(), |solution, descriptor_index| {
        match descriptor_index {
            0 => solution.shifts.len(),
            1 => solution.vehicles.len(),
            _ => 0,
        }
    })
}

fn shift_count(solution: &MixedPlan) -> usize {
    solution.shifts.len()
}

fn get_worker(solution: &MixedPlan, entity_index: usize) -> Option<usize> {
    solution.shifts[entity_index].worker
}

fn set_worker(solution: &mut MixedPlan, entity_index: usize, value: Option<usize>) {
    solution.shifts[entity_index].worker = value;
}

fn worker_count(solution: &MixedPlan) -> usize {
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

fn scalar_context() -> ScalarVariableContext<MixedPlan> {
    ScalarVariableContext::new(
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
        },
        true,
    )
}

fn list_context() -> ListVariableContext<MixedPlan, usize, NoopMeter, NoopMeter> {
    ListVariableContext::new(
        "Vehicle",
        list_len,
        list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        vehicle_count,
        NoopMeter,
        NoopMeter,
        "visits",
        1,
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

#[test]
fn default_scalar_selector_uses_change_only() {
    let director = create_director(MixedPlan {
        shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
        vehicles: vec![],
        score: None,
    });
    let selector = build_move_selector(None, &scalar_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 1);
    match &neighborhoods[0] {
        Neighborhood::Flat(leafs) => {
            assert_eq!(leafs.selectors().len(), 1);
            assert!(matches!(
                &leafs.selectors()[0],
                NeighborhoodLeaf::Scalar(StandardLeafSelector::Change(_))
            ));
            assert_eq!(selector.size(&director), 4);
        }
        Neighborhood::Limited(_) => panic!("default scalar selector must not wrap a limit"),
    }
}

#[test]
fn default_list_selector_uses_three_explicit_neighborhoods() {
    let selector = build_move_selector(None, &list_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 3);
    assert!(matches!(
        &neighborhoods[0],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::NearbyListChange(_)))
    ));
    assert!(matches!(
        &neighborhoods[1],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::NearbyListSwap(_)))
    ));
    assert!(matches!(
        &neighborhoods[2],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::ListReverse(_)))
    ));
}

#[test]
fn mixed_default_selector_puts_list_neighborhoods_before_scalar_change() {
    let selector = build_move_selector(None, &mixed_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 4);
    assert!(matches!(
        &neighborhoods[0],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::NearbyListChange(_)))
    ));
    assert!(matches!(
        &neighborhoods[1],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::NearbyListSwap(_)))
    ));
    assert!(matches!(
        &neighborhoods[2],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::ListReverse(_)))
    ));
    assert!(matches!(
        &neighborhoods[3],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(StandardLeafSelector::Change(_)))
    ));
}

#[test]
fn explicit_selected_count_limit_selector_remains_supported() {
    let director = create_director(MixedPlan {
        shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
        vehicles: vec![],
        score: None,
    });
    let config =
        MoveSelectorConfig::SelectedCountLimitMoveSelector(SelectedCountLimitMoveSelectorConfig {
            selected_count_limit: 2,
            selector: Box::new(MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                target: VariableTargetConfig::default(),
            })),
        });

    let selector = build_move_selector(Some(&config), &scalar_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 1);
    match &neighborhoods[0] {
        Neighborhood::Limited(limit) => {
            assert_eq!(limit.limit(), 2);
            assert_eq!(selector.size(&director), 2);
        }
        Neighborhood::Flat(_) => panic!("selected_count_limit must remain a neighborhood wrapper"),
    }
}

#[test]
fn explicit_scalar_union_selector_remains_supported() {
    let config = MoveSelectorConfig::UnionMoveSelector(UnionMoveSelectorConfig {
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let selector = build_move_selector(Some(&config), &scalar_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 2);
    assert!(matches!(
        &neighborhoods[0],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(StandardLeafSelector::Change(_)))
    ));
    assert!(matches!(
        &neighborhoods[1],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(StandardLeafSelector::Swap(_)))
    ));
}

#[test]
fn default_scalar_local_search_uses_scalar_streaming_defaults() {
    let phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        None,
        &scalar_only_model(),
        Some(7),
    );
    let debug = format!("{phase:?}");

    assert!(debug.contains("SimulatedAnnealing"));
    assert!(debug.contains("accepted_count_limit: 1"));
}

#[test]
fn default_list_and_mixed_local_search_use_list_streaming_defaults() {
    let list_phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        None,
        &list_only_model(),
        None,
    );
    let list_debug = format!("{list_phase:?}");
    assert!(list_debug.contains("LateAcceptance"));
    assert!(list_debug.contains("accepted_count_limit: 4"));

    let mixed_phase =
        build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(None, &mixed_model(), None);
    let mixed_debug = format!("{mixed_phase:?}");
    assert!(mixed_debug.contains("LateAcceptance"));
    assert!(mixed_debug.contains("accepted_count_limit: 4"));
}

#[test]
fn explicit_acceptor_and_forager_configs_override_defaults() {
    let config = LocalSearchConfig {
        acceptor: Some(AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(17),
        })),
        forager: Some(ForagerConfig {
            accepted_count_limit: Some(9),
            pick_early_type: Some(PickEarlyType::FirstBestScoreImproving),
        }),
        move_selector: None,
        termination: None,
    };

    let phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        Some(&config),
        &scalar_only_model(),
        None,
    );
    let debug = format!("{phase:?}");

    assert!(debug.contains("LateAcceptance"));
    assert!(debug.contains("size: 17"));
    assert!(debug.contains("BestScoreImproving"));
}
