use crate::wasm::Expression;

/// Fluent builder for constructing expression trees
///
/// Provides a convenient API for building complex expressions without
/// excessive nesting of Box::new() calls.
///
/// # Example
/// ```
/// use solverforge_core::wasm::{Expression, Expr};
/// use solverforge_core::wasm::FieldAccessExt;
///
/// // Build: param(0).employee != null
/// let shift = Expr::param(0);
/// let employee = shift.get("Shift", "employee");
/// let predicate = Expr::is_not_null(employee);
/// ```
pub struct Expr;

impl Expr {
    // ===== Literals =====

    /// Create an integer literal
    pub fn int(value: i64) -> Expression {
        Expression::IntLiteral { value }
    }

    /// Create a boolean literal
    pub fn bool(value: bool) -> Expression {
        Expression::BoolLiteral { value }
    }

    /// Create a null literal
    pub fn null() -> Expression {
        Expression::Null
    }

    // ===== Parameter Access =====

    /// Access a function parameter by index
    pub fn param(index: u32) -> Expression {
        Expression::Param { index }
    }

    // ===== Comparisons =====

    /// Equal comparison (==)
    pub fn eq(left: Expression, right: Expression) -> Expression {
        Expression::Eq {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Not equal comparison (!=)
    pub fn ne(left: Expression, right: Expression) -> Expression {
        Expression::Ne {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Less than comparison (<)
    pub fn lt(left: Expression, right: Expression) -> Expression {
        Expression::Lt {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Less than or equal comparison (<=)
    pub fn le(left: Expression, right: Expression) -> Expression {
        Expression::Le {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Greater than comparison (>)
    pub fn gt(left: Expression, right: Expression) -> Expression {
        Expression::Gt {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Greater than or equal comparison (>=)
    pub fn ge(left: Expression, right: Expression) -> Expression {
        Expression::Ge {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    // ===== Logical Operations =====

    /// Logical AND (&&)
    pub fn and(left: Expression, right: Expression) -> Expression {
        Expression::And {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Logical OR (||)
    pub fn or(left: Expression, right: Expression) -> Expression {
        Expression::Or {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Logical NOT (!)
    pub fn not(operand: Expression) -> Expression {
        Expression::Not {
            operand: Box::new(operand),
        }
    }

    /// Null check (is null)
    pub fn is_null(operand: Expression) -> Expression {
        Expression::IsNull {
            operand: Box::new(operand),
        }
    }

    /// Not-null check (is not null)
    pub fn is_not_null(operand: Expression) -> Expression {
        Expression::IsNotNull {
            operand: Box::new(operand),
        }
    }

    // ===== Arithmetic Operations =====

    /// Addition (+)
    pub fn add(left: Expression, right: Expression) -> Expression {
        Expression::Add {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Subtraction (-)
    pub fn sub(left: Expression, right: Expression) -> Expression {
        Expression::Sub {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Multiplication (*)
    pub fn mul(left: Expression, right: Expression) -> Expression {
        Expression::Mul {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Division (/)
    pub fn div(left: Expression, right: Expression) -> Expression {
        Expression::Div {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    // ===== Host Function Calls =====

    /// Call a host function
    pub fn host_call(function_name: impl Into<String>, args: Vec<Expression>) -> Expression {
        Expression::HostCall {
            function_name: function_name.into(),
            args,
        }
    }

    // ===== List Operations =====

    /// Check if a list contains an element
    ///
    /// Generates a loop that iterates through the list and checks for equality.
    /// Returns true if the element is found, false otherwise.
    pub fn list_contains(list: Expression, element: Expression) -> Expression {
        Expression::ListContains {
            list: Box::new(list),
            element: Box::new(element),
        }
    }

    // ===== Convenience Methods =====

    /// String equality via host function
    ///
    /// Equivalent to: hstringEquals(left, right)
    pub fn string_equals(left: Expression, right: Expression) -> Expression {
        Self::host_call("hstringEquals", vec![left, right])
    }

    /// Check if two time ranges overlap
    ///
    /// Equivalent to: start1 < end2 && start2 < end1
    pub fn ranges_overlap(
        start1: Expression,
        end1: Expression,
        start2: Expression,
        end2: Expression,
    ) -> Expression {
        Self::and(Self::lt(start1, end2), Self::lt(start2, end1))
    }

    // ===== Conditional =====

    /// If-then-else conditional expression
    pub fn if_then_else(
        condition: Expression,
        then_branch: Expression,
        else_branch: Expression,
    ) -> Expression {
        Expression::IfThenElse {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    }
}

/// Extension trait for chaining field access
///
/// Allows for fluent syntax like: `Expr::param(0).get("Class", "field")`
pub trait FieldAccessExt {
    /// Access a field on this expression
    fn get(self, class_name: &str, field_name: &str) -> Expression;
}

impl FieldAccessExt for Expression {
    fn get(self, class_name: &str, field_name: &str) -> Expression {
        Expression::FieldAccess {
            object: Box::new(self),
            class_name: class_name.into(),
            field_name: field_name.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_literal() {
        let expr = Expr::int(42);
        assert_eq!(expr, Expression::IntLiteral { value: 42 });
    }

    #[test]
    fn test_bool_literal() {
        let expr = Expr::bool(true);
        assert_eq!(expr, Expression::BoolLiteral { value: true });
    }

    #[test]
    fn test_null() {
        let expr = Expr::null();
        assert_eq!(expr, Expression::Null);
    }

    #[test]
    fn test_param() {
        let expr = Expr::param(0);
        assert_eq!(expr, Expression::Param { index: 0 });
    }

    #[test]
    fn test_field_access_chaining() {
        let expr = Expr::param(0).get("Employee", "name");

        match expr {
            Expression::FieldAccess {
                object,
                class_name,
                field_name,
            } => {
                assert_eq!(class_name, "Employee");
                assert_eq!(field_name, "name");
                assert_eq!(*object, Expression::Param { index: 0 });
            }
            _ => panic!("Expected FieldAccess"),
        }
    }

    #[test]
    fn test_eq() {
        let expr = Expr::eq(Expr::int(1), Expr::int(2));

        match expr {
            Expression::Eq { left, right } => {
                assert_eq!(*left, Expression::IntLiteral { value: 1 });
                assert_eq!(*right, Expression::IntLiteral { value: 2 });
            }
            _ => panic!("Expected Eq"),
        }
    }

    #[test]
    fn test_and() {
        let expr = Expr::and(Expr::bool(true), Expr::bool(false));

        match expr {
            Expression::And { left, right } => {
                assert_eq!(*left, Expression::BoolLiteral { value: true });
                assert_eq!(*right, Expression::BoolLiteral { value: false });
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn test_is_not_null() {
        let expr = Expr::is_not_null(Expr::param(0));

        match expr {
            Expression::IsNotNull { operand } => {
                assert_eq!(*operand, Expression::Param { index: 0 });
            }
            _ => panic!("Expected IsNotNull"),
        }
    }

    #[test]
    fn test_add() {
        let expr = Expr::add(Expr::int(10), Expr::int(20));

        match expr {
            Expression::Add { left, right } => {
                assert_eq!(*left, Expression::IntLiteral { value: 10 });
                assert_eq!(*right, Expression::IntLiteral { value: 20 });
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_host_call() {
        let expr = Expr::host_call("test_func", vec![Expr::int(1), Expr::int(2)]);

        match expr {
            Expression::HostCall {
                function_name,
                args,
            } => {
                assert_eq!(function_name, "test_func");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected HostCall"),
        }
    }

    #[test]
    fn test_string_equals() {
        let left = Expr::param(0).get("Employee", "skill");
        let right = Expr::param(1).get("Shift", "requiredSkill");
        let expr = Expr::string_equals(left, right);

        match expr {
            Expression::HostCall {
                function_name,
                args,
            } => {
                assert_eq!(function_name, "hstringEquals");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected HostCall"),
        }
    }

    #[test]
    fn test_ranges_overlap() {
        let expr = Expr::ranges_overlap(Expr::int(0), Expr::int(10), Expr::int(5), Expr::int(15));

        match expr {
            Expression::And { left, right } => {
                // start1 < end2
                match *left {
                    Expression::Lt { .. } => {}
                    _ => panic!("Expected Lt in left side"),
                }
                // start2 < end1
                match *right {
                    Expression::Lt { .. } => {}
                    _ => panic!("Expected Lt in right side"),
                }
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn test_complex_predicate_builder() {
        // Build: param(0).employee != null && hstringEquals(employee.skill, shift.requiredSkill)
        let shift = Expr::param(0);
        let employee = shift.clone().get("Shift", "employee");

        let predicate = Expr::and(
            Expr::is_not_null(employee.clone()),
            Expr::not(Expr::string_equals(
                employee.get("Employee", "skill"),
                shift.get("Shift", "requiredSkill"),
            )),
        );

        // Should be parseable
        match predicate {
            Expression::And { .. } => {}
            _ => panic!("Expected And at top level"),
        }
    }

    #[test]
    fn test_nested_field_access() {
        // Build: param(0).shift.employee.name
        let expr = Expr::param(0)
            .get("Assignment", "shift")
            .get("Shift", "employee")
            .get("Employee", "name");

        match expr {
            Expression::FieldAccess {
                class_name,
                field_name,
                object,
            } => {
                assert_eq!(class_name, "Employee");
                assert_eq!(field_name, "name");

                match *object {
                    Expression::FieldAccess { .. } => {}
                    _ => panic!("Expected nested FieldAccess"),
                }
            }
            _ => panic!("Expected FieldAccess"),
        }
    }

    #[test]
    fn test_time_calculation() {
        // Build: (shift.start / 24)
        let expr = Expr::div(Expr::param(0).get("Shift", "start"), Expr::int(24));

        match expr {
            Expression::Div { left, right } => {
                match *left {
                    Expression::FieldAccess { .. } => {}
                    _ => panic!("Expected FieldAccess"),
                }
                assert_eq!(*right, Expression::IntLiteral { value: 24 });
            }
            _ => panic!("Expected Div"),
        }
    }

    #[test]
    fn test_if_then_else() {
        // Build: if x > 0 { 1 } else { 0 }
        let expr = Expr::if_then_else(
            Expr::gt(Expr::param(0), Expr::int(0)),
            Expr::int(1),
            Expr::int(0),
        );

        match expr {
            Expression::IfThenElse {
                condition,
                then_branch,
                else_branch,
            } => {
                match *condition {
                    Expression::Gt { .. } => {}
                    _ => panic!("Expected Gt"),
                }
                assert_eq!(*then_branch, Expression::IntLiteral { value: 1 });
                assert_eq!(*else_branch, Expression::IntLiteral { value: 0 });
            }
            _ => panic!("Expected IfThenElse"),
        }
    }

    #[test]
    fn test_nested_if_then_else() {
        // Build: if x > 0 { if x > 10 { 2 } else { 1 } } else { 0 }
        let expr = Expr::if_then_else(
            Expr::gt(Expr::param(0), Expr::int(0)),
            Expr::if_then_else(
                Expr::gt(Expr::param(0), Expr::int(10)),
                Expr::int(2),
                Expr::int(1),
            ),
            Expr::int(0),
        );

        match expr {
            Expression::IfThenElse { then_branch, .. } => match *then_branch {
                Expression::IfThenElse { .. } => {}
                _ => panic!("Expected nested IfThenElse"),
            },
            _ => panic!("Expected IfThenElse"),
        }
    }

    #[test]
    fn test_list_contains() {
        let list = Expr::param(0).get("Employee", "skills");
        let element = Expr::param(1).get("Shift", "requiredSkill");
        let expr = Expr::list_contains(list, element);

        match expr {
            Expression::ListContains { list, element } => {
                match *list {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "skills");
                    }
                    _ => panic!("Expected FieldAccess for list"),
                }
                match *element {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "requiredSkill");
                    }
                    _ => panic!("Expected FieldAccess for element"),
                }
            }
            _ => panic!("Expected ListContains"),
        }
    }
}
