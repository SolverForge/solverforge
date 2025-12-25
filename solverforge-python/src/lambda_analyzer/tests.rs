//! Tests for lambda analyzer.

use super::*;
use pyo3::types::PyDict;

fn init_python() {
    pyo3::Python::initialize();
}

#[test]
fn test_generate_lambda_name_unique() {
    let name1 = generate_lambda_name("test");
    let name2 = generate_lambda_name("test");
    assert_ne!(name1, name2);
    assert!(name1.starts_with("test_"));
    assert!(name2.starts_with("test_"));
}

#[test]
fn test_lambda_info_param_count() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.field", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();
        assert_eq!(info.param_count, 1);
    });
}

#[test]
fn test_lambda_info_param_count_two() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda a, b: a.field", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();
        assert_eq!(info.param_count, 2);
    });
}

#[test]
fn test_analyze_simple_field_access() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.timeslot", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Lesson").unwrap();

        match &info.expression {
            Expression::FieldAccess {
                field_name,
                class_name,
                ..
            } => {
                assert_eq!(field_name, "timeslot");
                assert_eq!(class_name, "Lesson");
            }
            _ => panic!("Expected FieldAccess, got {:?}", info.expression),
        }
    });
}

#[test]
fn test_analyze_is_not_none() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.room is not None", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Lesson").unwrap();

        match &info.expression {
            Expression::IsNotNull { operand } => match operand.as_ref() {
                Expression::FieldAccess { field_name, .. } => {
                    assert_eq!(field_name, "room");
                }
                _ => panic!("Expected FieldAccess inside IsNotNull"),
            },
            _ => panic!("Expected IsNotNull, got {:?}", info.expression),
        }
    });
}

#[test]
fn test_analyze_is_none() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.room is None", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        assert!(matches!(info.expression, Expression::IsNull { .. }));
    });
}

#[test]
fn test_analyze_comparison_gt() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.count > 5", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        match &info.expression {
            Expression::Gt { left, right } => {
                assert!(matches!(left.as_ref(), Expression::FieldAccess { .. }));
                assert!(matches!(
                    right.as_ref(),
                    Expression::IntLiteral { value: 5 }
                ));
            }
            _ => panic!("Expected Gt, got {:?}", info.expression),
        }
    });
}

#[test]
fn test_analyze_comparison_eq() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.status == 1", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        assert!(matches!(info.expression, Expression::Eq { .. }));
    });
}

#[test]
fn test_analyze_and_expression() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(
            c"f = lambda x: x.room is not None and x.timeslot is not None",
            None,
            Some(&locals),
        )
        .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        assert!(matches!(info.expression, Expression::And { .. }));
    });
}

#[test]
fn test_analyze_or_expression() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.a > 0 or x.b > 0", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        assert!(matches!(info.expression, Expression::Or { .. }));
    });
}

#[test]
fn test_analyze_not_expression() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: not x.active", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        assert!(matches!(info.expression, Expression::Not { .. }));
    });
}

#[test]
fn test_analyze_arithmetic_add() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.value + 10", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        assert!(matches!(info.expression, Expression::Add { .. }));
    });
}

#[test]
fn test_analyze_arithmetic_sub() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.value - 5", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        assert!(matches!(info.expression, Expression::Sub { .. }));
    });
}

#[test]
fn test_analyze_arithmetic_mul() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.value * 2", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        assert!(matches!(info.expression, Expression::Mul { .. }));
    });
}

#[test]
fn test_analyze_arithmetic_div() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.value / 2", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        assert!(matches!(info.expression, Expression::Div { .. }));
    });
}

#[test]
fn test_analyze_bi_lambda() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda a, b: a.room == b.room", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        match &info.expression {
            Expression::Eq { left, right } => {
                // Verify both sides are field accesses from different params
                match (left.as_ref(), right.as_ref()) {
                    (
                        Expression::FieldAccess {
                            object: left_obj, ..
                        },
                        Expression::FieldAccess {
                            object: right_obj, ..
                        },
                    ) => {
                        assert!(matches!(left_obj.as_ref(), Expression::Param { index: 0 }));
                        assert!(matches!(right_obj.as_ref(), Expression::Param { index: 1 }));
                    }
                    _ => panic!("Expected field accesses"),
                }
            }
            _ => panic!("Expected Eq expression"),
        }
    });
}

#[test]
fn test_analyze_bi_lambda_direct_param_add() {
    // Tests LOAD_FAST_LOAD_FAST bytecode (Python 3.12+)
    // This is used by compose() combiner lambdas like: lambda a, b: a + b
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda a, b: a + b", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        match &info.expression {
            Expression::Add { left, right } => {
                assert!(matches!(left.as_ref(), Expression::Param { index: 0 }));
                assert!(matches!(right.as_ref(), Expression::Param { index: 1 }));
            }
            _ => panic!("Expected Add expression, got {:?}", info.expression),
        }
    });
}

#[test]
fn test_analyze_tri_lambda_arithmetic() {
    // Tests three-parameter lambda with arithmetic
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda a, b, c: a + b + c", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        // Should be Add(Add(a, b), c)
        match &info.expression {
            Expression::Add { left, right } => {
                // right should be Param 2 (c)
                assert!(matches!(right.as_ref(), Expression::Param { index: 2 }));
                // left should be Add(a, b)
                match left.as_ref() {
                    Expression::Add {
                        left: l2,
                        right: r2,
                    } => {
                        assert!(matches!(l2.as_ref(), Expression::Param { index: 0 }));
                        assert!(matches!(r2.as_ref(), Expression::Param { index: 1 }));
                    }
                    _ => panic!("Expected nested Add"),
                }
            }
            _ => panic!("Expected Add expression, got {:?}", info.expression),
        }
    });
}

#[test]
fn test_analyze_nested_field_access() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.employee.name", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        match &info.expression {
            Expression::FieldAccess {
                field_name, object, ..
            } => {
                assert_eq!(field_name, "name");
                // The object should be another FieldAccess
                match object.as_ref() {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "employee");
                    }
                    _ => panic!("Expected nested FieldAccess"),
                }
            }
            _ => panic!("Expected FieldAccess"),
        }
    });
}

#[test]
fn test_lambda_info_new_analyzes_immediately() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.field", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

        // Expression should be populated with FieldAccess
        assert!(matches!(info.expression, Expression::FieldAccess { .. }));
    });
}

#[test]
fn test_lambda_info_to_wasm_function() {
    init_python();
    Python::attach(|py| {
        let locals = PyDict::new(py);
        py.run(c"f = lambda x: x.field", None, Some(&locals))
            .unwrap();
        let func = locals.get_item("f").unwrap().unwrap();

        let info = LambdaInfo::new(py, func.unbind(), "equal_map", "Entity").unwrap();
        let wasm_func = info.to_wasm_function();

        assert!(wasm_func.name().starts_with("equal_map_"));
    });
}

#[test]
fn test_extract_lambda_from_filter_call() {
    let source = ".filter(lambda vehicle: vehicle.calculate_total_demand() > vehicle.capacity)";
    let result = extract_lambda_from_source(source);
    assert_eq!(
        result,
        "_ = lambda vehicle: vehicle.calculate_total_demand() > vehicle.capacity"
    );
}

#[test]
fn test_extract_lambda_from_penalize_call() {
    let source = "        .penalize(HardSoftScore.ONE_HARD, lambda vehicle: vehicle.demand - 10)";
    let result = extract_lambda_from_source(source);
    assert_eq!(result, "_ = lambda vehicle: vehicle.demand - 10");
}

#[test]
fn test_extract_lambda_simple() {
    let source = "lambda x: x.field";
    let result = extract_lambda_from_source(source);
    assert_eq!(result, "_ = lambda x: x.field");
}

#[test]
fn test_extract_lambda_with_nested_parens() {
    let source = ".filter(lambda x: (x.a + x.b) > 0)";
    let result = extract_lambda_from_source(source);
    assert_eq!(result, "_ = lambda x: (x.a + x.b) > 0");
}

#[test]
fn test_extract_lambda_second_arg() {
    let source = ".penalize(Score.ONE, lambda x: x.value)";
    let result = extract_lambda_from_source(source);
    assert_eq!(result, "_ = lambda x: x.value");
}

#[test]
fn test_extract_lambda_no_lambda() {
    let source = "some_other_code()";
    let result = extract_lambda_from_source(source);
    assert_eq!(result, "some_other_code()");
}

// ========================================================================
// Method Introspection Tests
// ========================================================================
// NOTE: Registry tests are in registry.rs
// ========================================================================

#[test]
fn test_analyze_method_body_simple_field_return() {
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests

        // Define a method that returns a field
        let locals = PyDict::new(py);
        py.run(
            c"class Vehicle:\n    def get_capacity(self):\n        return self.capacity",
            None,
            Some(&locals),
        )
        .unwrap();
        let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();
        register_class(py, "Vehicle", &vehicle_class);

        let method = get_method_from_class(py, "Vehicle", "get_capacity").unwrap();
        let expr = analyze_method_body(py, &method, "Vehicle").unwrap();

        // Should be FieldAccess on self (param 0)
        match expr {
            Expression::FieldAccess {
                object,
                field_name,
                class_name,
            } => {
                assert_eq!(field_name, "capacity");
                assert_eq!(class_name, "Vehicle");
                assert!(matches!(*object, Expression::Param { index: 0 }));
            }
            _ => panic!("Expected FieldAccess, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_method_body_arithmetic() {
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests

        // Define a method with arithmetic: self.demand - self.capacity
        let locals = PyDict::new(py);
        py.run(
            c"class Vehicle:\n    def get_excess(self):\n        return self.demand - self.capacity",
            None,
            Some(&locals),
        )
        .unwrap();
        let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();
        register_class(py, "Vehicle", &vehicle_class);

        let method = get_method_from_class(py, "Vehicle", "get_excess").unwrap();
        let expr = analyze_method_body(py, &method, "Vehicle").unwrap();

        // Should be Sub(FieldAccess(demand), FieldAccess(capacity))
        match expr {
            Expression::Sub { left, right } => {
                match *left {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "demand");
                    }
                    _ => panic!("Expected FieldAccess on left"),
                }
                match *right {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "capacity");
                    }
                    _ => panic!("Expected FieldAccess on right"),
                }
            }
            _ => panic!("Expected Sub expression, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_method_body_with_param() {
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests

        // Define a method with extra parameter: def add_value(self, x): return self.value + x
        let locals = PyDict::new(py);
        py.run(
            c"class Entity:\n    def add_value(self, x):\n        return self.value + x",
            None,
            Some(&locals),
        )
        .unwrap();
        let entity_class = locals.get_item("Entity").unwrap().unwrap();
        register_class(py, "Entity", &entity_class);

        let method = get_method_from_class(py, "Entity", "add_value").unwrap();
        let expr = analyze_method_body(py, &method, "Entity").unwrap();

        // Should be Add(FieldAccess(self.value), Param(1))
        match expr {
            Expression::Add { left, right } => {
                match *left {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "value");
                    }
                    _ => panic!("Expected FieldAccess on left"),
                }
                // x is param index 1 (self is 0)
                assert!(matches!(*right, Expression::Param { index: 1 }));
            }
            _ => panic!("Expected Add expression, got {:?}", expr),
        }
    });
}

// ========================================================================
// Expression Substitution Tests
// ========================================================================

#[test]
fn test_substitute_param_simple() {
    // Param(0) -> FieldAccess
    let expr = Expression::Param { index: 0 };
    let substitute = Expression::FieldAccess {
        object: Box::new(Expression::Param { index: 0 }),
        class_name: "Vehicle".to_string(),
        field_name: "id".to_string(),
    };

    let result = substitute_param(expr, 0, &substitute);
    assert!(matches!(result, Expression::FieldAccess { .. }));
}

#[test]
fn test_substitute_param_no_match() {
    // Param(1) should not be replaced when substituting index 0
    let expr = Expression::Param { index: 1 };
    let substitute = Expression::IntLiteral { value: 42 };

    let result = substitute_param(expr, 0, &substitute);
    assert!(matches!(result, Expression::Param { index: 1 }));
}

#[test]
fn test_substitute_param_in_field_access() {
    // FieldAccess(Param(0), "capacity") -> FieldAccess(FieldAccess(Param(0), "vehicle"), "capacity")
    let expr = Expression::FieldAccess {
        object: Box::new(Expression::Param { index: 0 }),
        class_name: "Vehicle".to_string(),
        field_name: "capacity".to_string(),
    };
    let substitute = Expression::FieldAccess {
        object: Box::new(Expression::Param { index: 0 }),
        class_name: "Route".to_string(),
        field_name: "vehicle".to_string(),
    };

    let result = substitute_param(expr, 0, &substitute);
    match result {
        Expression::FieldAccess {
            object, field_name, ..
        } => {
            assert_eq!(field_name, "capacity");
            // The object should now be the substitute (another FieldAccess)
            assert!(matches!(*object, Expression::FieldAccess { .. }));
        }
        _ => panic!("Expected FieldAccess"),
    }
}

#[test]
fn test_substitute_param_in_binary_op() {
    // Add(Param(0), IntLiteral(10)) with Param(0) -> FieldAccess
    let expr = Expression::Add {
        left: Box::new(Expression::Param { index: 0 }),
        right: Box::new(Expression::IntLiteral { value: 10 }),
    };
    let substitute = Expression::FieldAccess {
        object: Box::new(Expression::Param { index: 0 }),
        class_name: "Entity".to_string(),
        field_name: "value".to_string(),
    };

    let result = substitute_param(expr, 0, &substitute);
    match result {
        Expression::Add { left, right } => {
            assert!(matches!(*left, Expression::FieldAccess { .. }));
            assert!(matches!(*right, Expression::IntLiteral { value: 10 }));
        }
        _ => panic!("Expected Add"),
    }
}

#[test]
fn test_substitute_param_preserves_literals() {
    let expr = Expression::IntLiteral { value: 42 };
    let substitute = Expression::Param { index: 99 };

    let result = substitute_param(expr, 0, &substitute);
    assert!(matches!(result, Expression::IntLiteral { value: 42 }));
}

#[test]
fn test_substitute_param_method_inlining_scenario() {
    // Simulate method inlining:
    // Method: def get_excess(self): return self.demand - self.capacity
    // Analyzed as: Sub(FieldAccess(Param(0), "demand"), FieldAccess(Param(0), "capacity"))
    //
    // Lambda: lambda v: v.get_excess() > 0
    // When inlining, we substitute Param(0) in method body with Param(0) from lambda
    // (which represents 'v')

    let method_body = Expression::Sub {
        left: Box::new(Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }), // self
            class_name: "Vehicle".to_string(),
            field_name: "demand".to_string(),
        }),
        right: Box::new(Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }), // self
            class_name: "Vehicle".to_string(),
            field_name: "capacity".to_string(),
        }),
    };

    // The calling object in lambda is Param(0) (the 'v' parameter)
    let calling_object = Expression::Param { index: 0 };

    // After substitution, self references become lambda parameter references
    let inlined = substitute_param(method_body, 0, &calling_object);

    match inlined {
        Expression::Sub { left, right } => {
            // Both should still be FieldAccess with Param(0) as object
            match (*left, *right) {
                (
                    Expression::FieldAccess {
                        object: l_obj,
                        field_name: l_field,
                        ..
                    },
                    Expression::FieldAccess {
                        object: r_obj,
                        field_name: r_field,
                        ..
                    },
                ) => {
                    assert_eq!(l_field, "demand");
                    assert_eq!(r_field, "capacity");
                    assert!(matches!(*l_obj, Expression::Param { index: 0 }));
                    assert!(matches!(*r_obj, Expression::Param { index: 0 }));
                }
                _ => panic!("Expected FieldAccess on both sides"),
            }
        }
        _ => panic!("Expected Sub expression"),
    }
}

// ========================================================================
// AST Method Inlining Tests
// ========================================================================

#[test]
fn test_ast_method_call_error_when_unregistered() {
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests

        // Lambda that calls an unregistered method: lambda v: v.get_name()
        let locals = PyDict::new(py);
        py.run(
            c"
lambda_func = lambda v: v.get_name()
",
            None,
            Some(&locals),
        )
        .unwrap();

        let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
        // Should return error since get_name method cannot be inlined
        let result = LambdaInfo::new(py, lambda_obj.clone().unbind(), "test", "Entity");
        assert!(result.is_err(), "Expected error when method not registered");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Cannot inline method"),
            "Error should mention inlining failure: {}",
            err_msg
        );
    });
}

#[test]
fn test_ast_method_call_with_inlining() {
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests

        // Register Entity class with a method
        let locals = PyDict::new(py);
        py.run(
            c"
class Entity:
    def is_available(self):
        return self.status == 'active'

lambda_func = lambda e: e.is_available()
",
            None,
            Some(&locals),
        )
        .unwrap();

        let entity_class = locals.get_item("Entity").unwrap().unwrap();
        register_class(py, "Entity", &entity_class);

        let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
        let lambda_info =
            LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Entity").unwrap();

        // String comparisons ARE now inlined with StringLiteral support
        match &lambda_info.expression {
            Expression::Eq { left, right } => {
                // Left should be FieldAccess to status
                match **left {
                    Expression::FieldAccess { ref field_name, .. } => {
                        assert_eq!(field_name, "status");
                    }
                    _ => panic!("Expected FieldAccess on left, got {:?}", left),
                }
                // Right should be StringLiteral("active")
                match **right {
                    Expression::StringLiteral { ref value } => {
                        assert_eq!(value, "active");
                    }
                    _ => panic!("Expected StringLiteral on right, got {:?}", right),
                }
            }
            _ => panic!(
                "Expected Eq expression with StringLiteral, got {:?}",
                lambda_info.expression
            ),
        }
    });
}

#[test]
fn test_ast_method_call_with_arguments() {
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests

        // Register class with a method that takes arguments
        // Use unique class name to avoid race condition with other tests
        let locals = PyDict::new(py);
        py.run(
            c"
class EntityWithArgs:
    def check_value(self, threshold):
        return self.value > threshold

lambda_func = lambda e, t: e.check_value(t)
",
            None,
            Some(&locals),
        )
        .unwrap();

        let entity_class = locals.get_item("EntityWithArgs").unwrap().unwrap();
        register_class(py, "EntityWithArgs", &entity_class);

        let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
        let lambda_info =
            LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "EntityWithArgs").unwrap();

        // Should inline the method with parameter substitution
        match &lambda_info.expression {
            Expression::Gt { left, right } => {
                // Left should be FieldAccess to value
                match **left {
                    Expression::FieldAccess { ref field_name, .. } => {
                        assert_eq!(field_name, "value");
                    }
                    _ => panic!("Expected FieldAccess on left"),
                }
                // Right should be Param(1) (the threshold argument)
                assert!(matches!(**right, Expression::Param { index: 1 }));
            }
            _ => panic!("Expected Gt comparison, got {:?}", lambda_info.expression),
        }
    });
}

#[test]
fn test_ast_method_call_inlined_in_comparison() {
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests
        // Use unique class name to avoid collision
        let locals = PyDict::new(py);
        py.run(
            c"
class EntityPriority:
    def get_priority(self):
        return self.priority

lambda_func = lambda e: e.get_priority() > 5
",
            None,
            Some(&locals),
        )
        .unwrap();

        let entity_class = locals.get_item("EntityPriority").unwrap().unwrap();
        register_class(py, "EntityPriority", &entity_class);

        let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
        let lambda_info =
            LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "EntityPriority").unwrap();

        // Should produce Gt(FieldAccess(priority), IntLiteral(5))
        match &lambda_info.expression {
            Expression::Gt { left, right } => {
                match **left {
                    Expression::FieldAccess { ref field_name, .. } => {
                        assert_eq!(field_name, "priority");
                    }
                    _ => panic!("Expected FieldAccess"),
                }
                assert!(matches!(**right, Expression::IntLiteral { value: 5 }));
            }
            _ => panic!("Expected Gt expression"),
        }
    });
}

// ========================================================================
// Integration Tests for Method Analysis
// ========================================================================

#[test]
fn test_integration_method_inlining_with_registration() {
    // Complete flow: register class -> create lambda with method call -> verify inlining
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests

        // Define and register a domain class
        let locals = PyDict::new(py);
        py.run(
            c"
class Vehicle:
    def is_valid(self):
        return self.status == 'valid'

lambda_func = lambda v: v.is_valid()
",
            None,
            Some(&locals),
        )
        .unwrap();

        let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();
        register_class(py, "Vehicle", &vehicle_class);

        // Verify the class is registered by looking it up
        let method = get_method_from_class(py, "Vehicle", "is_valid");
        assert!(
            method.is_some(),
            "Method should be found after registration"
        );

        // Analyze lambda with the method call
        let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
        let lambda_info =
            LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Vehicle").unwrap();

        // Should have inlined the method call to an Eq expression
        match &lambda_info.expression {
            Expression::Eq { left, right: _ } => match **left {
                Expression::FieldAccess { ref field_name, .. } => {
                    assert_eq!(field_name, "status");
                }
                _ => panic!("Expected field access"),
            },
            _ => panic!("Expected inlined Eq expression"),
        }
    });
}

#[test]
fn test_integration_method_with_multiple_fields() {
    // Test inlining method that references multiple fields
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests

        let locals = PyDict::new(py);
        py.run(
            c"
class Shift:
    def is_overbooked(self):
        return self.hours > self.max_hours

lambda_func = lambda s: s.is_overbooked()
",
            None,
            Some(&locals),
        )
        .unwrap();

        let shift_class = locals.get_item("Shift").unwrap().unwrap();
        register_class(py, "Shift", &shift_class);

        let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
        let lambda_info =
            LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Shift").unwrap();

        // Should inline to Gt(FieldAccess(hours), FieldAccess(max_hours))
        match &lambda_info.expression {
            Expression::Gt { left, right } => match (&**left, &**right) {
                (
                    Expression::FieldAccess {
                        field_name: left_field,
                        ..
                    },
                    Expression::FieldAccess {
                        field_name: right_field,
                        ..
                    },
                ) => {
                    assert_eq!(left_field, "hours");
                    assert_eq!(right_field, "max_hours");
                }
                _ => panic!("Expected FieldAccess on both sides"),
            },
            _ => panic!("Expected Gt expression"),
        }
    });
}

#[test]
fn test_integration_method_chain_through_parameters() {
    // Test method call with arguments that get properly substituted
    init_python();
    Python::attach(|py| {
        // NOTE: Don't clear registry - causes race condition with parallel tests

        let locals = PyDict::new(py);
        py.run(
            c"
class Employee:
    def meets_minimum_salary(self, min_salary):
        return self.salary >= min_salary

lambda_func = lambda e, threshold: e.meets_minimum_salary(threshold)
",
            None,
            Some(&locals),
        )
        .unwrap();

        let employee_class = locals.get_item("Employee").unwrap().unwrap();
        register_class(py, "Employee", &employee_class);

        let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
        let lambda_info =
            LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Employee").unwrap();

        // Should inline to Ge(FieldAccess(salary), Param(1))
        match &lambda_info.expression {
            Expression::Ge { left, right } => {
                match (&**left, &**right) {
                    (Expression::FieldAccess { field_name, .. }, Expression::Param { index })
                        if field_name == "salary" && *index == 1 =>
                    {
                        // Correct!
                    }
                    _ => panic!("Expected Ge(FieldAccess(salary), Param(1))"),
                }
            }
            _ => panic!("Expected Ge expression, got {:?}", lambda_info.expression),
        }
    });
}

#[test]
fn test_integration_registry_persistence() {
    // Test that registered classes persist across multiple lambda analyses
    // NOTE: Don't clear registry here - it causes race conditions with parallel tests
    init_python();
    Python::attach(|py| {
        // Use unique class name to avoid collision with other parallel tests
        let locals = PyDict::new(py);
        py.run(
            c"
class TaskPersistence:
    def is_completed(self):
        return self.status == 'done'

    def is_urgent(self):
        return self.priority > 5

lambda_completed = lambda t: t.is_completed()
lambda_urgent = lambda t: t.is_urgent()
",
            None,
            Some(&locals),
        )
        .unwrap();

        let task_class = locals.get_item("TaskPersistence").unwrap().unwrap();
        register_class(py, "TaskPersistence", &task_class);

        // Analyze first lambda
        let lambda1 = locals.get_item("lambda_completed").unwrap().unwrap();
        let info1 =
            LambdaInfo::new(py, lambda1.clone().unbind(), "filter", "TaskPersistence").unwrap();
        assert!(matches!(info1.expression, Expression::Eq { .. }));

        // Analyze second lambda - should still have access to registered class
        let lambda2 = locals.get_item("lambda_urgent").unwrap().unwrap();
        let info2 =
            LambdaInfo::new(py, lambda2.clone().unbind(), "filter", "TaskPersistence").unwrap();
        assert!(matches!(info2.expression, Expression::Gt { .. }));
    });
}
