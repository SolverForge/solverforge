use std::any::TypeId;

use solverforge_config::{
    AcceptorConfig, CartesianProductConfig, ChangeMoveConfig, ForagerConfig, LateAcceptanceConfig,
    LimitedNeighborhoodConfig, ListChangeMoveConfig, ListReverseMoveConfig,
    ListRuinMoveSelectorConfig, LocalSearchConfig, MoveSelectorConfig,
    RuinRecreateMoveSelectorConfig, SwapMoveConfig, UnionMoveSelectorConfig, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
    VariableDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::*;
use crate::builder::list_selector::ListLeafSelector;
use crate::builder::scalar_selector::ScalarLeafSelector;
use crate::builder::{ListVariableContext, ScalarVariableContext, ValueSource, VariableContext};
use crate::heuristic::selector::decorator::FilteringMoveSelector;
use crate::heuristic::selector::move_selector::{
    collect_cursor_indices, MoveCandidateRef, MoveCursor,
};
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

#[test]
fn default_scalar_selector_uses_change_and_swap() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor.clone(),
    );
    let selector = build_move_selector(None, &scalar_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 2);
    match &neighborhoods[0] {
        Neighborhood::Flat(leafs) => {
            assert_eq!(leafs.selectors().len(), 1);
            assert!(matches!(
                &leafs.selectors()[0],
                NeighborhoodLeaf::Scalar(ScalarLeafSelector::Change(_))
            ));
        }
        Neighborhood::Limited { .. } => panic!("default scalar selector must not wrap a limit"),
        Neighborhood::Cartesian(_) => {
            panic!("default scalar selector must not wrap a cartesian neighborhood")
        }
    }
    assert!(matches!(
        &neighborhoods[1],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Swap(_)))
    ));
    assert_eq!(selector.size(&director), 7);
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
fn mixed_default_selector_puts_list_neighborhoods_before_scalar_defaults() {
    let selector = build_move_selector(None, &mixed_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 5);
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
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Change(_)))
    ));
    assert!(matches!(
        &neighborhoods[4],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Swap(_)))
    ));
}

#[test]
fn explicit_limited_neighborhood_remains_supported() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor.clone(),
    );
    let config = MoveSelectorConfig::LimitedNeighborhood(LimitedNeighborhoodConfig {
        selected_count_limit: 2,
        selector: Box::new(MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
            target: VariableTargetConfig::default(),
        })),
    });

    let selector = build_move_selector(Some(&config), &scalar_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 1);
    match &neighborhoods[0] {
        Neighborhood::Limited {
            selected_count_limit,
            ..
        } => {
            assert_eq!(*selected_count_limit, 2);
            assert_eq!(selector.size(&director), 2);
        }
        Neighborhood::Flat(_) => panic!("limited_neighborhood must remain a neighborhood wrapper"),
        Neighborhood::Cartesian(_) => {
            panic!("limited_neighborhood must not become a cartesian neighborhood")
        }
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
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Change(_)))
    ));
    assert!(matches!(
        &neighborhoods[1],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Swap(_)))
    ));
}

#[test]
fn cartesian_scalar_selector_builds_composite_moves() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![
                Shift { worker: Some(0) },
                Shift { worker: Some(1) },
                Shift { worker: Some(2) },
            ],
            vehicles: vec![],
            score: None,
        },
        descriptor,
    );
    let change = MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let swap = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        selectors: vec![change.clone(), swap.clone()],
    });

    let selector = build_move_selector(Some(&config), &scalar_only_model(), None);
    let neighborhoods = selector.selectors();
    let left = build_move_selector(Some(&change), &scalar_only_model(), None);
    let right = build_move_selector(Some(&swap), &scalar_only_model(), None);

    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert_eq!(neighborhoods.len(), 1);
    assert!(selector.size(&director) <= left.size(&director) * right.size(&director));
    assert!(matches!(&neighborhoods[0], Neighborhood::Cartesian(_)));
    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|mov| mov.is_doable(&director))));
    assert!(matches!(
        cursor.take_candidate(indices[0]),
        NeighborhoodMove::Composite(_)
    ));
}

#[test]
fn cartesian_list_selector_builds_composite_moves() {
    let descriptor = descriptor(false);
    let director = create_director(
        MixedPlan {
            shifts: vec![],
            vehicles: vec![
                Vehicle {
                    visits: vec![1, 2, 3],
                },
                Vehicle { visits: vec![4, 5] },
            ],
            score: None,
        },
        descriptor,
    );
    let list_change = MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let list_reverse = MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        selectors: vec![list_change.clone(), list_reverse.clone()],
    });

    let selector = build_move_selector(Some(&config), &list_only_model(), None);
    let neighborhoods = selector.selectors();
    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert_eq!(neighborhoods.len(), 1);
    assert!(!indices.is_empty());
    assert!(matches!(&neighborhoods[0], Neighborhood::Cartesian(_)));
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|mov| mov.is_doable(&director))));
    assert!(matches!(
        cursor.take_candidate(indices[0]),
        NeighborhoodMove::Composite(_)
    ));
}

#[test]
fn cartesian_mixed_selector_supports_limited_children() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![Vehicle {
                visits: vec![1, 2, 3],
            }],
            score: None,
        },
        descriptor,
    );
    let limited_change = MoveSelectorConfig::LimitedNeighborhood(LimitedNeighborhoodConfig {
        selected_count_limit: 2,
        selector: Box::new(MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
            target: VariableTargetConfig::default(),
        })),
    });
    let list_reverse = MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        selectors: vec![limited_change.clone(), list_reverse.clone()],
    });

    let selector = build_move_selector(Some(&config), &mixed_model(), None);
    let left = build_move_selector(Some(&limited_change), &mixed_model(), None);
    let right = build_move_selector(Some(&list_reverse), &mixed_model(), None);
    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert!(selector.size(&director) <= left.size(&director) * right.size(&director));
    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|mov| mov.is_doable(&director))));
    assert!(indices.iter().all(|&index| {
        cursor
            .candidate(index)
            .is_some_and(|mov| mov.variable_name() == "cartesian_product")
    }));
    assert!(matches!(
        cursor.take_candidate(indices[0]),
        NeighborhoodMove::Composite(_)
    ));
}

fn keep_all_mixed_cartesian_candidates(
    candidate: MoveCandidateRef<'_, MixedPlan, NeighborhoodMove<MixedPlan, usize>>,
) -> bool {
    matches!(candidate, MoveCandidateRef::Sequential(_))
}

#[test]
fn mixed_builder_cartesian_selector_survives_filtering_wrapper() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![Vehicle {
                visits: vec![1, 2, 3],
            }],
            score: None,
        },
        descriptor,
    );
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let selector = build_move_selector(Some(&config), &mixed_model(), None);
    let filtered = FilteringMoveSelector::new(selector, keep_all_mixed_cartesian_candidates);
    let mut cursor = filtered.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(cursor.take_candidate(indices[0]).is_doable(&director));
}

#[test]
#[should_panic(
    expected = "cartesian_product left child cannot contain ruin_recreate_move_selector or list_ruin_move_selector"
)]
fn cartesian_selector_rejects_score_seeking_scalar_left_child() {
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        selectors: vec![
            MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig::default()),
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let _ = build_move_selector(Some(&config), &scalar_only_model(), None);
}

#[test]
#[should_panic(
    expected = "cartesian_product left child cannot contain ruin_recreate_move_selector or list_ruin_move_selector"
)]
fn cartesian_selector_rejects_score_seeking_list_left_child() {
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        selectors: vec![
            MoveSelectorConfig::ListRuinMoveSelector(ListRuinMoveSelectorConfig::default()),
            MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig {
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let _ = build_move_selector(Some(&config), &list_only_model(), Some(7));
}

#[test]
#[should_panic(expected = "move selector configuration produced no neighborhoods")]
fn empty_model_does_not_synthesize_scalar_neighborhoods() {
    let _ =
        build_move_selector::<MixedPlan, usize, NoopMeter, NoopMeter>(None, &empty_model(), None);
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
        forager: Some(ForagerConfig::FirstBestScoreImproving),
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
