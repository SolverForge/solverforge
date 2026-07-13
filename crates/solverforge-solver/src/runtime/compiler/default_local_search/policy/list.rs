//! The explicit list capability-policy table for omitted local search.

use solverforge_config::{
    KOptMoveSelectorConfig, ListChangeMoveConfig, ListPermuteMoveConfig, ListPrecedenceMoveConfig,
    ListReverseMoveConfig, ListRuinMoveSelectorConfig, ListSwapMoveConfig, MoveSelectorConfig,
    NearbyListChangeMoveConfig, NearbyListSwapMoveConfig, SelectionOrder, SublistChangeMoveConfig,
    SublistSwapMoveConfig, VariableTargetConfig,
};

use crate::builder::context::list_access::ListAccessCapability;
use crate::builder::RuntimeScalarSlotId;

use super::super::super::types::CompiledListSlot;
use super::super::{
    target_for, DefaultLocalSearchSelectorDeclaration, DefaultLocalSearchSelectorFamily,
    DefaultSelectorCapabilityPolicy,
};

const DEFAULT_LIST_NEARBY_LIMIT: usize = 20;

/// Typed and dynamic slots consume these rows in exactly this order. A named
/// reduced-capability row is legal only when it is a distinct selector with
/// its own immutable provenance.
const LIST_POLICY_TABLE: [DefaultListPolicyRule; 8] = [
    DefaultListPolicyRule::PrecedencePair,
    DefaultListPolicyRule::NearbyChange,
    DefaultListPolicyRule::NearbySwap,
    DefaultListPolicyRule::SublistChange,
    DefaultListPolicyRule::SublistSwap,
    DefaultListPolicyRule::Reverse,
    DefaultListPolicyRule::KOpt,
    DefaultListPolicyRule::Ruin,
];

#[derive(Clone, Copy, Debug)]
enum DefaultListPolicyRule {
    PrecedencePair,
    NearbyChange,
    NearbySwap,
    SublistChange,
    SublistSwap,
    Reverse,
    KOpt,
    Ruin,
}

pub(super) fn append_list_policy<S, V, DM, IDM>(
    list_slots: &[CompiledListSlot<S, V, DM, IDM>],
    declarations: &mut Vec<DefaultLocalSearchSelectorDeclaration>,
) {
    for rule in LIST_POLICY_TABLE {
        append_rule(rule, list_slots, declarations);
    }
}

fn append_rule<S, V, DM, IDM>(
    rule: DefaultListPolicyRule,
    list_slots: &[CompiledListSlot<S, V, DM, IDM>],
    declarations: &mut Vec<DefaultLocalSearchSelectorDeclaration>,
) {
    match rule {
        DefaultListPolicyRule::PrecedencePair => {
            for slot in list_slots
                .iter()
                .filter(|slot| supports_precedence_moves(slot))
            {
                let target = target_for(&slot.identity());
                push_list(
                    declarations,
                    DefaultLocalSearchSelectorFamily::ListPrecedence,
                    DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
                    MoveSelectorConfig::ListPrecedenceMoveSelector(ListPrecedenceMoveConfig {
                        selection_order: Some(SelectionOrder::Random),
                        selection_metric: None,
                        target: target.clone(),
                    }),
                    vec![slot.identity()],
                );
                push_list(
                    declarations,
                    DefaultLocalSearchSelectorFamily::ListPermute,
                    DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
                    MoveSelectorConfig::ListPermuteMoveSelector(ListPermuteMoveConfig {
                        target,
                        ..ListPermuteMoveConfig::default()
                    }),
                    vec![slot.identity()],
                );
            }
        }
        DefaultListPolicyRule::NearbyChange => {
            push_list_if_any(
                declarations,
                DefaultLocalSearchSelectorFamily::NearbyListChange,
                DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
                MoveSelectorConfig::NearbyListChangeMoveSelector(NearbyListChangeMoveConfig {
                    selection_order: Some(SelectionOrder::Random),
                    selection_metric: None,
                    max_nearby: DEFAULT_LIST_NEARBY_LIMIT,
                    target: VariableTargetConfig::default(),
                }),
                slot_ids_with(list_slots, &[ListAccessCapability::CrossPositionDistance]),
            );
            push_list_if_any(
                declarations,
                DefaultLocalSearchSelectorFamily::PlainListChange,
                DefaultSelectorCapabilityPolicy::PlainListChangeWithoutCrossPositionDistance,
                MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig::default()),
                slot_ids_where(list_slots, |slot| {
                    !slot.supports(ListAccessCapability::CrossPositionDistance)
                }),
            );
        }
        DefaultListPolicyRule::NearbySwap => {
            push_list_if_any(
                declarations,
                DefaultLocalSearchSelectorFamily::NearbyListSwap,
                DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
                MoveSelectorConfig::NearbyListSwapMoveSelector(NearbyListSwapMoveConfig {
                    selection_order: Some(SelectionOrder::Random),
                    selection_metric: None,
                    max_nearby: DEFAULT_LIST_NEARBY_LIMIT,
                    target: VariableTargetConfig::default(),
                }),
                slot_ids_with(
                    list_slots,
                    &[
                        ListAccessCapability::CrossPositionDistance,
                        ListAccessCapability::Set,
                    ],
                ),
            );
            push_list_if_any(
                declarations,
                DefaultLocalSearchSelectorFamily::PlainListSwap,
                DefaultSelectorCapabilityPolicy::PlainListSwapWithoutCrossPositionDistance,
                MoveSelectorConfig::ListSwapMoveSelector(ListSwapMoveConfig::default()),
                slot_ids_where(list_slots, |slot| {
                    !slot.supports(ListAccessCapability::CrossPositionDistance)
                        && slot.supports(ListAccessCapability::Set)
                }),
            );
        }
        DefaultListPolicyRule::SublistChange => push_list_if_any(
            declarations,
            DefaultLocalSearchSelectorFamily::SublistChange,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
            MoveSelectorConfig::SublistChangeMoveSelector(SublistChangeMoveConfig::default()),
            slot_ids_with(list_slots, &[ListAccessCapability::Sublist]),
        ),
        DefaultListPolicyRule::SublistSwap => push_list_if_any(
            declarations,
            DefaultLocalSearchSelectorFamily::SublistSwap,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
            MoveSelectorConfig::SublistSwapMoveSelector(SublistSwapMoveConfig::default()),
            slot_ids_with(list_slots, &[ListAccessCapability::Sublist]),
        ),
        DefaultListPolicyRule::Reverse => push_list_if_any(
            declarations,
            DefaultLocalSearchSelectorFamily::ListReverse,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
            MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig::default()),
            slot_ids_with(list_slots, &[ListAccessCapability::Reverse]),
        ),
        DefaultListPolicyRule::KOpt => {
            push_list_if_any(
                declarations,
                DefaultLocalSearchSelectorFamily::NearbyKOpt,
                DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
                MoveSelectorConfig::KOptMoveSelector(KOptMoveSelectorConfig {
                    max_nearby: DEFAULT_LIST_NEARBY_LIMIT,
                    ..KOptMoveSelectorConfig::default()
                }),
                slot_ids_with(
                    list_slots,
                    &[
                        ListAccessCapability::Sublist,
                        ListAccessCapability::IntraPositionDistance,
                    ],
                ),
            );
            push_list_if_any(
                declarations,
                DefaultLocalSearchSelectorFamily::UnboundedKOpt,
                DefaultSelectorCapabilityPolicy::UnboundedKOptWithoutIntraPositionDistance,
                MoveSelectorConfig::KOptMoveSelector(KOptMoveSelectorConfig::default()),
                slot_ids_where(list_slots, |slot| {
                    slot.supports(ListAccessCapability::Sublist)
                        && !slot.supports(ListAccessCapability::IntraPositionDistance)
                }),
            );
        }
        DefaultListPolicyRule::Ruin => push_list_if_any(
            declarations,
            DefaultLocalSearchSelectorFamily::ListRuin,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
            MoveSelectorConfig::ListRuinMoveSelector(ListRuinMoveSelectorConfig::default()),
            list_slots.iter().map(|slot| slot.identity()).collect(),
        ),
    }
}

fn push_list(
    declarations: &mut Vec<DefaultLocalSearchSelectorDeclaration>,
    family: DefaultLocalSearchSelectorFamily,
    capability_policy: DefaultSelectorCapabilityPolicy,
    config: MoveSelectorConfig,
    slots: Vec<RuntimeScalarSlotId>,
) {
    declarations.push(DefaultLocalSearchSelectorDeclaration {
        family,
        capability_policy,
        config,
        slots,
    });
}

fn push_list_if_any(
    declarations: &mut Vec<DefaultLocalSearchSelectorDeclaration>,
    family: DefaultLocalSearchSelectorFamily,
    capability_policy: DefaultSelectorCapabilityPolicy,
    config: MoveSelectorConfig,
    slots: Vec<RuntimeScalarSlotId>,
) {
    for slot in slots {
        let targeted = with_target(config.clone(), target_for(&slot));
        push_list(
            declarations,
            family,
            capability_policy,
            targeted,
            vec![slot],
        );
    }
}

fn with_target(mut config: MoveSelectorConfig, target: VariableTargetConfig) -> MoveSelectorConfig {
    match &mut config {
        MoveSelectorConfig::ListChangeMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::NearbyListChangeMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::ListSwapMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::ListPermuteMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::ListPrecedenceMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::NearbyListSwapMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::SublistChangeMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::SublistSwapMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::ListReverseMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::KOptMoveSelector(selector) => selector.target = target,
        MoveSelectorConfig::ListRuinMoveSelector(selector) => selector.target = target,
        _ => unreachable!("default list policy produced a non-list selector"),
    }
    config
}

fn slot_ids_with<S, V, DM, IDM>(
    list_slots: &[CompiledListSlot<S, V, DM, IDM>],
    required: &[ListAccessCapability],
) -> Vec<RuntimeScalarSlotId> {
    slot_ids_where(list_slots, |slot| {
        required.iter().all(|capability| slot.supports(*capability))
    })
}

fn slot_ids_where<S, V, DM, IDM>(
    list_slots: &[CompiledListSlot<S, V, DM, IDM>],
    predicate: impl Fn(&CompiledListSlot<S, V, DM, IDM>) -> bool,
) -> Vec<RuntimeScalarSlotId> {
    list_slots
        .iter()
        .filter(|slot| predicate(slot))
        .map(|slot| slot.identity())
        .collect()
}

pub(super) fn supports_precedence_moves<S, V, DM, IDM>(
    slot: &CompiledListSlot<S, V, DM, IDM>,
) -> bool {
    [
        ListAccessCapability::Precedence,
        ListAccessCapability::Set,
        ListAccessCapability::Reverse,
        ListAccessCapability::Sublist,
    ]
    .into_iter()
    .all(|capability| slot.supports(capability))
}
