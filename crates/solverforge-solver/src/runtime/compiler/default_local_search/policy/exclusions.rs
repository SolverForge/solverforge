//! Default move families a slot does not receive, and why.
//!
//! These records describe the selector table in `list.rs` and `scalar.rs`
//! from the same capability checks; they never feed selector construction.

use crate::builder::context::list_access::ListAccessCapability;
use crate::builder::ScalarAccessCapability;

use super::super::super::defaults::DefaultScalarBinding;
use super::super::super::types::CompiledListSlot;
use super::super::DefaultExcludedFamily;

const PRECEDENCE_BUNDLE: [ListAccessCapability; 4] = [
    ListAccessCapability::Precedence,
    ListAccessCapability::Set,
    ListAccessCapability::Reverse,
    ListAccessCapability::Sublist,
];

pub(super) fn list_exclusions<S, V, DM, IDM>(
    list_slots: &[CompiledListSlot<S, V, DM, IDM>],
    excluded: &mut Vec<DefaultExcludedFamily>,
) {
    for slot in list_slots {
        let mut exclude = |family: &'static str, required: &[ListAccessCapability]| {
            let missing = required
                .iter()
                .copied()
                .filter(|capability| !slot.supports(*capability))
                .map(list_capability_label)
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                excluded.push(DefaultExcludedFamily {
                    family,
                    slot: slot.identity(),
                    reason: format!("missing={}", missing.join("+")),
                });
            }
        };
        exclude("list_precedence_and_permute", &PRECEDENCE_BUNDLE);
        exclude(
            "nearby_list_change",
            &[ListAccessCapability::CrossPositionDistance],
        );
        if slot.supports(ListAccessCapability::Set) {
            exclude(
                "nearby_list_swap",
                &[ListAccessCapability::CrossPositionDistance],
            );
        } else {
            exclude("list_swap", &[ListAccessCapability::Set]);
        }
        exclude("sublist_change_and_swap", &[ListAccessCapability::Sublist]);
        exclude("list_reverse", &[ListAccessCapability::Reverse]);
        if slot.supports(ListAccessCapability::Sublist) {
            exclude(
                "nearby_k_opt",
                &[ListAccessCapability::IntraPositionDistance],
            );
        } else {
            exclude("k_opt", &[ListAccessCapability::Sublist]);
        }
    }
}

pub(super) fn scalar_exclusions<S>(
    scalar_slots: &[DefaultScalarBinding<S>],
    excluded: &mut Vec<DefaultExcludedFamily>,
) {
    for binding in scalar_slots {
        let slot = binding.slot.id();
        if binding.assignment_owned {
            excluded.push(DefaultExcludedFamily {
                family: "scalar_change_and_swap",
                slot,
                reason: "owned_by_assignment_group".to_string(),
            });
            continue;
        }
        for (family, capability, label) in [
            (
                "nearby_scalar_change",
                ScalarAccessCapability::NearbyValue,
                "nearby_value",
            ),
            (
                "nearby_scalar_swap",
                ScalarAccessCapability::NearbyEntity,
                "nearby_entity",
            ),
        ] {
            if !binding.slot.has_capability(capability) {
                excluded.push(DefaultExcludedFamily {
                    family,
                    slot: slot.clone(),
                    reason: format!("missing={label}"),
                });
            }
        }
    }
}

fn list_capability_label(capability: ListAccessCapability) -> &'static str {
    match capability {
        ListAccessCapability::Set => "set",
        ListAccessCapability::Replace => "replace",
        ListAccessCapability::Reverse => "reverse",
        ListAccessCapability::Sublist => "sublist",
        ListAccessCapability::ElementOwner => "element_owner",
        ListAccessCapability::ConstructionOrderKey => "construction_order_key",
        ListAccessCapability::Precedence => "precedence",
        ListAccessCapability::CrossPositionDistance => "cross_position_distance",
        ListAccessCapability::IntraPositionDistance => "intra_position_distance",
        ListAccessCapability::Route => "route",
        ListAccessCapability::Savings => "savings",
        ListAccessCapability::SavingsNeighbors => "savings_neighbors",
    }
}
