#[cfg(test)]
mod tests {
    use crate::domain::{
        DomainClass, DomainModelBuilder, FieldDescriptor, FieldType, PlanningAnnotation,
        PrimitiveType, ScoreType,
    };
    use crate::wasm::generator::{PredicateDefinition, WasmModuleBuilder};
    use crate::wasm::{Expr, Expression, FieldAccessExt, HostFunctionRegistry};

    fn create_test_model() -> crate::domain::DomainModel {
        DomainModelBuilder::new()
            .add_class(
                DomainClass::new("Lesson")
                    .with_annotation(PlanningAnnotation::PlanningEntity)
                    .with_field(FieldDescriptor::new(
                        "id",
                        FieldType::Primitive(PrimitiveType::Int),
                    ))
                    .with_field(FieldDescriptor::new(
                        "roomId",
                        FieldType::Primitive(PrimitiveType::Int),
                    )),
            )
            .add_class(
                DomainClass::new("Timetable")
                    .with_annotation(PlanningAnnotation::PlanningSolution)
                    .with_field(
                        FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                            .with_annotation(PlanningAnnotation::planning_score()),
                    ),
            )
            .build()
    }

    #[test]
    fn test_build_minimal_module() {
        let model = create_test_model();
        let builder = WasmModuleBuilder::new().with_domain_model(model);
        let wasm_bytes = builder.build().unwrap();

        assert_eq!(&wasm_bytes[0..4], b"\0asm");
        assert_eq!(&wasm_bytes[4..8], &[1, 0, 0, 0]);
    }

    #[test]
    fn test_predicate_generation() {
        let model = create_test_model();
        let left = Expr::param(0).get("Lesson", "roomId");
        let right = Expr::param(1).get("Lesson", "roomId");
        let predicate = PredicateDefinition::from_expression("same_room", 2, Expr::eq(left, right));

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
    }

    #[test]
    fn test_build_base64() {
        let model = create_test_model();
        let builder = WasmModuleBuilder::new().with_domain_model(model);
        let base64 = builder.build_base64().unwrap();

        assert!(base64.starts_with("AGFzbQ"));
    }

    #[test]
    fn test_always_true_predicate() {
        let model = create_test_model();
        let predicate = PredicateDefinition::from_expression("always_true", 1, Expr::bool(true));

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
    }

    #[test]
    fn test_missing_domain_model() {
        let builder = WasmModuleBuilder::new();
        let result = builder.build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Domain model not set"));
    }

    #[test]
    fn test_predicate_missing_class() {
        let model = create_test_model();
        let left = Expr::param(0).get("NonExistent", "field");
        let right = Expr::param(1).get("NonExistent", "field");
        let predicate = PredicateDefinition::from_expression("bad_pred", 2, Expr::eq(left, right));

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_predicate_missing_field() {
        let model = create_test_model();
        let left = Expr::param(0).get("Lesson", "nonexistent");
        let right = Expr::param(1).get("Lesson", "nonexistent");
        let predicate = PredicateDefinition::from_expression("bad_pred", 2, Expr::eq(left, right));

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_configuration() {
        let model = create_test_model();
        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .with_initial_memory(32)
            .with_max_memory(Some(512));

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
    }

    #[test]
    fn test_comparison_expression_variants() {
        let model = create_test_model();

        let comparisons: Vec<(&str, Expression)> = vec![
            (
                "eq",
                Expr::eq(
                    Expr::param(0).get("Lesson", "id"),
                    Expr::param(1).get("Lesson", "id"),
                ),
            ),
            (
                "ne",
                Expr::ne(
                    Expr::param(0).get("Lesson", "id"),
                    Expr::param(1).get("Lesson", "id"),
                ),
            ),
            (
                "lt",
                Expr::lt(
                    Expr::param(0).get("Lesson", "id"),
                    Expr::param(1).get("Lesson", "id"),
                ),
            ),
            (
                "le",
                Expr::le(
                    Expr::param(0).get("Lesson", "id"),
                    Expr::param(1).get("Lesson", "id"),
                ),
            ),
            (
                "gt",
                Expr::gt(
                    Expr::param(0).get("Lesson", "id"),
                    Expr::param(1).get("Lesson", "id"),
                ),
            ),
            (
                "ge",
                Expr::ge(
                    Expr::param(0).get("Lesson", "id"),
                    Expr::param(1).get("Lesson", "id"),
                ),
            ),
            ("true", Expr::bool(true)),
            ("false", Expr::bool(false)),
        ];

        for (name, expr) in comparisons {
            let predicate = PredicateDefinition::from_expression(name, 2, expr);
            let builder = WasmModuleBuilder::new()
                .with_domain_model(model.clone())
                .add_predicate(predicate);

            let result = builder.build();
            assert!(result.is_ok(), "Failed for comparison: {}", name);
        }
    }

    #[test]
    fn test_expression_based_predicate() {
        let model = create_test_model();

        let left = Expr::param(0).get("Lesson", "roomId");
        let right = Expr::param(1).get("Lesson", "roomId");
        let expr = Expr::eq(left, right);

        let predicate = PredicateDefinition::from_expression("same_room_expr", 2, expr);

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
        assert_eq!(&wasm_bytes[0..4], b"\0asm");
    }

    #[test]
    fn test_expression_with_host_function() {
        let model = create_test_model();

        let left = Expr::param(0).get("Lesson", "id");
        let right = Expr::param(1).get("Lesson", "id");
        let expr = Expr::string_equals(left, right);

        let predicate = PredicateDefinition::from_expression("test_host_call", 2, expr);

        let registry = HostFunctionRegistry::with_standard_functions();

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .with_host_functions(registry)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
        assert_eq!(&wasm_bytes[0..4], b"\0asm");

        assert!(wasm_bytes.windows(2).any(|w| w[0] == 2 && w[1] > 0));
    }

    #[test]
    fn test_expression_with_logical_operations() {
        let model = create_test_model();

        let id_check = Expr::gt(Expr::param(0).get("Lesson", "id"), Expr::int(0));
        let room_match = Expr::eq(
            Expr::param(0).get("Lesson", "roomId"),
            Expr::param(1).get("Lesson", "roomId"),
        );
        let expr = Expr::and(id_check, room_match);

        let predicate = PredicateDefinition::from_expression("complex_predicate", 2, expr);

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
    }

    #[test]
    fn test_expression_with_if_then_else() {
        let model = create_test_model();

        let expr = Expr::if_then_else(
            Expr::gt(Expr::param(0).get("Lesson", "id"), Expr::int(0)),
            Expr::param(0).get("Lesson", "roomId"),
            Expr::int(0),
        );

        let predicate = PredicateDefinition::from_expression("conditional_pred", 1, expr);

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
        assert_eq!(&wasm_bytes[0..4], b"\0asm");
    }
}
