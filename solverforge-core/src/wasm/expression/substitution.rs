use super::Expression;

impl Expression {
    /// Substitute all occurrences of a parameter with a replacement expression.
    ///
    /// This is used for method inlining: when inlining `obj.method()`, we replace
    /// `Param(0)` (self) with `obj`, and other parameters with their call arguments.
    pub fn substitute_param(self, from_index: u32, substitute: &Expression) -> Expression {
        macro_rules! sub {
            ($e:expr) => {
                Box::new((*$e).substitute_param(from_index, substitute))
            };
        }

        macro_rules! binary {
            ($variant:ident, $left:expr, $right:expr) => {
                Expression::$variant {
                    left: sub!($left),
                    right: sub!($right),
                }
            };
        }

        macro_rules! unary {
            ($variant:ident, $operand:expr) => {
                Expression::$variant {
                    operand: sub!($operand),
                }
            };
        }

        match self {
            // Parameter substitution - the core operation
            Expression::Param { index } if index == from_index => substitute.clone(),
            Expression::Param { index } => Expression::Param { index },

            // Field access
            Expression::FieldAccess {
                object,
                class_name,
                field_name,
                field_type,
            } => Expression::FieldAccess {
                object: sub!(object),
                class_name,
                field_name,
                field_type,
            },

            // Comparisons (i32)
            Expression::Eq { left, right } => binary!(Eq, left, right),
            Expression::Ne { left, right } => binary!(Ne, left, right),
            Expression::Lt { left, right } => binary!(Lt, left, right),
            Expression::Le { left, right } => binary!(Le, left, right),
            Expression::Gt { left, right } => binary!(Gt, left, right),
            Expression::Ge { left, right } => binary!(Ge, left, right),

            // Comparisons (i64)
            Expression::Eq64 { left, right } => binary!(Eq64, left, right),
            Expression::Ne64 { left, right } => binary!(Ne64, left, right),
            Expression::Lt64 { left, right } => binary!(Lt64, left, right),
            Expression::Le64 { left, right } => binary!(Le64, left, right),
            Expression::Gt64 { left, right } => binary!(Gt64, left, right),
            Expression::Ge64 { left, right } => binary!(Ge64, left, right),

            // Arithmetic (i32)
            Expression::Add { left, right } => binary!(Add, left, right),
            Expression::Sub { left, right } => binary!(Sub, left, right),
            Expression::Mul { left, right } => binary!(Mul, left, right),
            Expression::Div { left, right } => binary!(Div, left, right),

            // Arithmetic (i64)
            Expression::Add64 { left, right } => binary!(Add64, left, right),
            Expression::Sub64 { left, right } => binary!(Sub64, left, right),
            Expression::Mul64 { left, right } => binary!(Mul64, left, right),
            Expression::Div64 { left, right } => binary!(Div64, left, right),

            // Arithmetic (f64)
            Expression::FloatAdd { left, right } => binary!(FloatAdd, left, right),
            Expression::FloatSub { left, right } => binary!(FloatSub, left, right),
            Expression::FloatMul { left, right } => binary!(FloatMul, left, right),
            Expression::FloatDiv { left, right } => binary!(FloatDiv, left, right),

            // Math functions - unary
            Expression::Sqrt { operand } => unary!(Sqrt, operand),
            Expression::FloatAbs { operand } => unary!(FloatAbs, operand),
            Expression::Round { operand } => unary!(Round, operand),
            Expression::Floor { operand } => unary!(Floor, operand),
            Expression::Ceil { operand } => unary!(Ceil, operand),
            Expression::Sin { operand } => unary!(Sin, operand),
            Expression::Cos { operand } => unary!(Cos, operand),
            Expression::Asin { operand } => unary!(Asin, operand),
            Expression::Acos { operand } => unary!(Acos, operand),
            Expression::Atan { operand } => unary!(Atan, operand),
            Expression::Radians { operand } => unary!(Radians, operand),
            Expression::IntToFloat { operand } => unary!(IntToFloat, operand),
            Expression::FloatToInt { operand } => unary!(FloatToInt, operand),

            // Math functions - binary
            Expression::Atan2 { y, x } => Expression::Atan2 {
                y: sub!(y),
                x: sub!(x),
            },

            // Logical operations
            Expression::And { left, right } => binary!(And, left, right),
            Expression::Or { left, right } => binary!(Or, left, right),
            Expression::Not { operand } => unary!(Not, operand),
            Expression::IsNull { operand } => unary!(IsNull, operand),
            Expression::IsNotNull { operand } => unary!(IsNotNull, operand),

            // Host calls
            Expression::HostCall {
                function_name,
                args,
            } => Expression::HostCall {
                function_name,
                args: args
                    .into_iter()
                    .map(|arg| arg.substitute_param(from_index, substitute))
                    .collect(),
            },

            // List operations
            Expression::ListContains { list, element } => Expression::ListContains {
                list: sub!(list),
                element: sub!(element),
            },
            Expression::Length { collection } => Expression::Length {
                collection: sub!(collection),
            },

            // Sum with index adjustment
            Expression::Sum {
                collection,
                item_var_name,
                item_param_index,
                item_class_name,
                accumulator_expr,
            } => {
                let new_index = if from_index < item_param_index {
                    item_param_index - 1
                } else {
                    item_param_index
                };

                Expression::Sum {
                    collection: sub!(collection),
                    item_var_name,
                    item_param_index: new_index,
                    item_class_name,
                    accumulator_expr: sub!(accumulator_expr),
                }
            }

            Expression::LastElement {
                collection,
                item_class_name,
            } => Expression::LastElement {
                collection: sub!(collection),
                item_class_name,
            },

            // Conditional
            Expression::IfThenElse {
                condition,
                then_branch,
                else_branch,
            } => Expression::IfThenElse {
                condition: sub!(condition),
                then_branch: sub!(then_branch),
                else_branch: sub!(else_branch),
            },
            Expression::IfThenElse64 {
                condition,
                then_branch,
                else_branch,
            } => Expression::IfThenElse64 {
                condition: sub!(condition),
                then_branch: sub!(then_branch),
                else_branch: sub!(else_branch),
            },

            // Type conversions
            Expression::I64ToI32 { operand } => unary!(I64ToI32, operand),
            Expression::I32ToI64 { operand } => unary!(I32ToI64, operand),

            // Literals - no params, return as-is
            Expression::Null
            | Expression::BoolLiteral { .. }
            | Expression::IntLiteral { .. }
            | Expression::Int64Literal { .. }
            | Expression::FloatLiteral { .. }
            | Expression::StringLiteral { .. } => self,
        }
    }
}
