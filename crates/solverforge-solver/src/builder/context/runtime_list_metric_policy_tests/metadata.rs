use super::*;

#[test]
fn absent_typed_metadata_is_bound_to_named_public_policies() {
    let slot = typed_slot(None, None, None);
    let plan = Plan {
        score: None,
        elements: vec![2],
        routes: vec![vec![]],
    };

    assert_eq!(
        slot.ownership_policy(),
        OwnershipPolicy::DeclaredUnrestricted
    );
    assert_eq!(
        slot.construction_order_policy(),
        ConstructionOrderPolicy::DeclaredNaturalElementOrder
    );
    assert_eq!(slot.precedence_policy(), PrecedencePolicy::Absent);
    assert_eq!(
        ListAccess::element_owner(&slot, &plan, &RuntimeListElement::Static(2)),
        Ok(None)
    );
    assert_eq!(
        ListAccess::construction_order_key(&slot, &plan, RuntimeListElement::Static(2)),
        Ok(0)
    );
    assert_eq!(
        ListAccess::extend_precedence_successors(
            &slot,
            &plan,
            RuntimeListElement::Static(2),
            &mut Vec::new(),
        )
        .expect_err("absent precedence must not enter precedence-only kernels")
        .capability,
        ListAccessCapability::Precedence
    );
}

#[test]
fn typed_successors_only_precedence_preserves_list_ruin_metadata() {
    let slot = RuntimeListSlot::from_static(
        typed_list_slot(None, None, None).with_precedence_hooks(None, Some(precedence_successors)),
        0,
    );
    let plan = Plan {
        score: None,
        elements: vec![2, 3],
        routes: vec![vec![]],
    };
    let mut successors = Vec::new();

    assert_eq!(slot.precedence_policy(), PrecedencePolicy::SuccessorsOnly);
    assert!(!slot.supports(ListAccessCapability::Precedence));
    ListAccess::extend_precedence_successors(
        &slot,
        &plan,
        RuntimeListElement::Static(2),
        &mut successors,
    )
    .expect("successor-only policy remains available to ListRuin");
    assert_eq!(successors, vec![RuntimeListElement::Static(3)]);
    assert_eq!(
        ListAccess::precedence_duration(&slot, &plan, RuntimeListElement::Static(2))
            .expect_err("scored precedence requires an explicit duration")
            .capability,
        ListAccessCapability::Precedence
    );
}

#[test]
fn typed_explicit_metadata_remains_available_to_required_families() {
    let slot = RuntimeListSlot::from_static(
        typed_list_slot(None, None, None)
            .with_element_owner_fn(Some(explicit_owner))
            .with_construction_element_order_key(Some(explicit_order))
            .with_precedence_hooks(Some(precedence_duration), Some(precedence_successors)),
        0,
    );
    let plan = Plan {
        score: None,
        elements: vec![2, 3],
        routes: vec![vec![]],
    };

    assert_eq!(
        slot.ownership_policy(),
        OwnershipPolicy::ExplicitStaticProvider
    );
    assert_eq!(
        slot.construction_order_policy(),
        ConstructionOrderPolicy::ExplicitStaticProvider
    );
    assert_eq!(slot.precedence_policy(), PrecedencePolicy::Explicit);
    assert!(slot.supports(ListAccessCapability::ElementOwner));
    assert!(slot.supports(ListAccessCapability::ConstructionOrderKey));
    assert!(slot.supports(ListAccessCapability::Precedence));
    assert_eq!(
        ListAccess::element_owner(&slot, &plan, &RuntimeListElement::Static(2)),
        Ok(Some(1))
    );
    assert_eq!(
        ListAccess::construction_order_key(&slot, &plan, RuntimeListElement::Static(2)),
        Ok(-1)
    );
    assert_eq!(
        ListAccess::precedence_duration(&slot, &plan, RuntimeListElement::Static(2)),
        Ok(1)
    );
}
