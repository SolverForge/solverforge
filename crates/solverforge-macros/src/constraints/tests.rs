use quote::quote;

use super::{ast::ConstraintProgram, parse, plan};

#[test]
fn parser_detects_same_binding_grouped_terminals_in_order() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let g = ConstraintFactory::<Plan, SoftScore>::new();
            let by_employee = g.for_each(shifts).group_by(employee, count());

            (
                by_employee.penalize(linear).named("linear"),
                by_employee.reward(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    assert_eq!(parsed.terminals.len(), 2);
    assert_eq!(
        parsed.terminals[0].stream_binding.to_string(),
        "by_employee"
    );
    assert_eq!(parsed.terminals[0].order, 0);
    assert_eq!(parsed.terminals[1].order, 1);
}

#[test]
fn planner_shares_only_when_all_terminals_use_one_binding() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let g = ConstraintFactory::<Plan, SoftScore>::new();
            let by_employee = g.for_each(shifts).group_by(employee, count());

            (
                by_employee.penalize(linear).named("linear"),
                by_employee.penalize(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::SharedGrouped { .. }));
}

#[test]
fn planner_leaves_mixed_tuple_unchanged() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let first = g.for_each(shifts).group_by(employee, count());
            let second = g.for_each(shifts).group_by(day, count());

            (
                first.penalize(linear).named("linear"),
                second.penalize(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::Passthrough(_)));
}

#[test]
fn fingerprint_normalizes_token_spacing() {
    let first: syn::Expr = syn::parse_quote! { g.for_each(shifts).group_by(employee, count()) };
    let second: syn::Expr = syn::parse_quote! {
        g . for_each ( shifts ) . group_by ( employee , count ( ) )
    };

    assert_eq!(
        super::fingerprint::expression_fingerprint(&first),
        super::fingerprint::expression_fingerprint(&second)
    );
    assert!(!quote!(#first).to_string().is_empty());
}
