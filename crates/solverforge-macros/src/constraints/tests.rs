use super::{
    ast::{ConstraintProgram, TailMember},
    parse, plan,
};

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
    assert_eq!(parsed.tail_members.len(), 2);
    let TailMember::Terminal(first) = &parsed.tail_members[0] else {
        panic!("first tuple member should parse as a terminal");
    };
    let TailMember::Terminal(second) = &parsed.tail_members[1] else {
        panic!("second tuple member should parse as a terminal");
    };
    assert_eq!(first.source_binding, "by_employee");
    assert_eq!(first.order, 0);
    assert_eq!(second.order, 1);
}

#[test]
fn parser_accepts_nonliteral_terminal_names() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let by_employee = g.for_each(shifts).group_by(employee, count());
            let squared_name = "squared";

            (
                by_employee.penalize(linear).named(LINEAR_NAME),
                by_employee.reward(square).named(squared_name),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::SharedGrouped(_)));
}

#[test]
fn parser_preserves_semicolonless_prefix_expressions() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            if cfg!(debug_assertions) {
                validate_constraints();
            }
            let by_employee = g.for_each(shifts).group_by(employee, count());

            (
                by_employee.penalize(linear).named("linear"),
                by_employee.reward(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    assert_eq!(parsed.prefix_statements.len(), 2);
    assert!(parsed.tail.is_some());
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::SharedGrouped(_)));
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
    assert!(matches!(planned, ConstraintProgram::SharedGrouped(_)));
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
fn planner_shares_tuple_with_unsupported_member_in_place() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let by_employee = g.for_each(shifts).group_by(employee, count());

            (
                by_employee.penalize(linear).named("linear"),
                existing_constraint(),
                by_employee.penalize(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::SharedGrouped(_)));
}

#[test]
fn planner_leaves_direct_complemented_grouped_stream_unchanged() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let by_employee = g
                .for_each(shifts)
                .group_by(employee, count())
                .complement(employees, employee, zero);

            (
                by_employee.penalize(linear).named("linear"),
                by_employee.penalize(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::Passthrough(_)));
}

#[test]
fn planner_shares_projected_complemented_grouped_stream() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let by_employee = g
                .for_each(shifts)
                .project(ShiftProjection)
                .group_by(employee, count())
                .complement(employees, employee, zero);

            (
                by_employee.penalize(linear).named("linear"),
                by_employee.penalize(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::SharedGrouped(_)));
}

#[test]
fn planner_leaves_repeated_non_grouped_stream_unchanged() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let all_shifts = g.for_each(shifts);

            (
                all_shifts.penalize(linear).named("linear"),
                all_shifts.reward(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    assert_eq!(parsed.tail_members.len(), 2);
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::Passthrough(_)));
}

#[test]
fn planner_leaves_distinct_bindings_with_identical_source_text_unchanged() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let first = ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(shifts)
                .group_by(employee, count());
            let second = ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(shifts)
                .group_by(employee, count());

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
fn planner_leaves_shadow_sensitive_distinct_bindings_unchanged() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let threshold = 1;
            let first = ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(shifts)
                .filter(|shift| shift.load > threshold)
                .group_by(employee, count());
            let threshold = 5;
            let second = ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(shifts)
                .filter(|shift| shift.load > threshold)
                .group_by(employee, count());

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
fn planner_uses_most_recent_shadowed_binding() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let by_employee = g.for_each(shifts).group_by(employee, count());
            let by_employee = g.for_each(shifts);

            (
                by_employee.penalize(linear).named("linear"),
                by_employee.reward(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::Passthrough(_)));
}

#[test]
fn planner_shares_most_recent_shadowed_grouped_binding() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let by_employee = g.for_each(shifts);
            let by_employee = g.for_each(shifts).group_by(employee, count());

            (
                by_employee.penalize(linear).named("linear"),
                by_employee.reward(square).named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::SharedGrouped(_)));
}

#[test]
fn planner_tracks_all_repeated_grouped_bindings() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let by_employee = g.for_each(shifts).group_by(employee, count());
            let by_day = g.for_each(shifts).group_by(day, count());

            (
                by_employee.penalize(linear).named("employee linear"),
                by_day.penalize(linear).named("day linear"),
                by_employee.reward(square).named("employee square"),
                by_day.reward(square).named("day square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    let ConstraintProgram::SharedGrouped(program) = planned else {
        panic!("expected every repeated binding to be shared");
    };
    assert_eq!(program.bindings.len(), 2);
    assert_eq!(program.bindings[0], "by_employee");
    assert_eq!(program.bindings[1], "by_day");
}

#[test]
fn planner_leaves_inline_chains_unchanged() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            (
                ConstraintFactory::<Plan, SoftScore>::new()
                    .for_each(shifts)
                    .group_by(employee, count())
                    .penalize(linear)
                    .named("linear"),
                ConstraintFactory::<Plan, SoftScore>::new()
                    .for_each(shifts)
                    .group_by(employee, count())
                    .reward(square)
                    .named("square"),
            )
        }
    };

    let parsed = parse::parse_constraint_function(function).expect("parse");
    let planned = plan::plan(parsed);
    assert!(matches!(planned, ConstraintProgram::Passthrough(_)));
}

#[test]
fn parser_rejects_terminal_named_extra_arguments() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let by_employee = g.for_each(shifts).group_by(employee, count());

            by_employee.penalize(linear).named("linear", extra)
        }
    };

    let error = parse::parse_constraint_function(function).expect_err("parse should reject arity");
    assert!(error.to_string().contains("exactly one .named"));
}

#[test]
fn parser_rejects_terminal_weight_extra_arguments() {
    let function: syn::ItemFn = syn::parse_quote! {
        fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
            let by_employee = g.for_each(shifts).group_by(employee, count());

            by_employee.penalize(linear, extra).named("linear")
        }
    };

    let error = parse::parse_constraint_function(function).expect_err("parse should reject arity");
    assert!(error.to_string().contains("exactly one weight"));
}
