use solverforge_config::{MoveSelectorConfig, SolverConfig};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCursor;

use super::super::super::super::graph::ListLeafKind;
use super::super::{
    CompiledListNeighborhoodLeafAdapter, RuntimeListNeighborhoodLeaf,
    RuntimeListNeighborhoodLeafError, RuntimeListNeighborhoodSelector,
};
use super::support::{
    config, director, dynamic_slot, initial_plan, static_slot, static_slot_for, ListPlan,
    PositionMetric, ALL_KINDS,
};

fn adapter(seed: Option<u64>) -> CompiledListNeighborhoodLeafAdapter {
    CompiledListNeighborhoodLeafAdapter::from_solver_config(&SolverConfig {
        random_seed: seed,
        ..SolverConfig::default()
    })
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn frozen_list_leaf_and_selector_hold_no_interior_mutable_stream_state() {
    assert_send_sync::<
        RuntimeListNeighborhoodSelector<ListPlan, usize, PositionMetric, PositionMetric>,
    >();
    assert_send_sync::<RuntimeListNeighborhoodLeaf<ListPlan, usize, PositionMetric, PositionMetric>>(
    );
}

#[test]
fn every_compiled_list_leaf_lowers_once_with_its_frozen_slots_config_and_seed() {
    for kind in ALL_KINDS {
        let leaf = adapter(Some(41))
            .lower_compiled_list(kind, config(kind), vec![static_slot_for(kind)])
            .unwrap_or_else(|_| panic!("{kind:?} lowers through the list leaf adapter"));
        assert_eq!(leaf.plan().kind(), kind);
        assert_eq!(leaf.plan().random_seed(), Some(41));
        assert_eq!(leaf.plan().slots().len(), 1);
        let score_director = director(initial_plan());
        let mut stream_state = leaf.new_stream_state();
        let mut cursor = leaf.open_cursor_with_stream_state(
            &mut stream_state,
            &score_director,
            crate::heuristic::selector::move_selector::MoveStreamContext::default(),
        );
        let runtime_move = cursor
            .next_owned_candidate(score_director.working_solution())
            .unwrap_or_else(|| panic!("{kind:?} leaf emits a runtime move"));
        assert_eq!(runtime_move.descriptor_index(), 0);
    }
}

#[test]
fn adapter_preserves_compiled_declaration_order_without_schema_rediscovery() {
    let leaf = adapter(Some(17))
        .lower_compiled_list(
            ListLeafKind::Change,
            config(ListLeafKind::Change),
            vec![static_slot(), dynamic_slot()],
        )
        .expect("frozen list slots lower directly");
    let identities = leaf
        .plan()
        .slots()
        .iter()
        .map(|slot| slot.identity())
        .collect::<Vec<_>>();
    assert_eq!(identities.len(), 2);
    assert!(identities[0].dynamic_identity.is_none());
    assert!(identities[1].dynamic_identity.is_some());
}

#[test]
fn adapter_moves_a_frozen_list_declaration_into_the_runtime_plan() {
    // The compiler hands the owned declaration parts directly; no node borrow
    // and no slot clone occurs at the runtime boundary.
    let leaf = CompiledListNeighborhoodLeafAdapter::from_solver_config(&SolverConfig::default())
        .lower_compiled_list(
            ListLeafKind::Change,
            config(ListLeafKind::Change),
            vec![static_slot()],
        )
        .expect("a compiled list leaf lowers on its owned declaration");
    assert_eq!(leaf.plan().kind(), ListLeafKind::Change);
}

#[test]
fn adapter_rejects_a_mismatched_family_instead_of_substituting_a_selector() {
    let error = adapter(None)
        .lower_compiled_list(
            ListLeafKind::Change,
            MoveSelectorConfig::ListSwapMoveSelector(
                solverforge_config::ListSwapMoveConfig::default(),
            ),
            vec![static_slot()],
        )
        .expect_err("family mismatch never falls back");
    assert!(matches!(error, RuntimeListNeighborhoodLeafError::Plan(_)));
}
