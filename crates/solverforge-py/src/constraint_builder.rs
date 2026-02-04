//! Constraint builder for Python API.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use solverforge_core::score::HardSoftScore;
use solverforge_dynamic::{DynamicConstraint, DynamicDescriptor, Expr};

/// A constraint builder for defining constraint pipelines.
#[pyclass]
#[derive(Clone)]
pub struct ConstraintBuilder {
    pub(crate) name: String,
    pub(crate) weight: HardSoftScore,
    pub(crate) class_idx: Option<usize>,
    pub(crate) operations: Vec<ConstraintOp>,
}

/// Constraint operations in the pipeline.
///
/// `ForEach` and `Join` track which entity class they operate on.
/// This allows field lookup to use the correct class for each parameter.
#[derive(Clone)]
pub enum ConstraintOp {
    /// Iterate over entities of a class. The usize is the class index.
    /// This becomes parameter A (index 0) in expressions.
    ForEach(usize),
    /// Join with another class. The usize is the class index.
    /// Conditions are unparsed strings that will be parsed during build.
    /// This becomes parameter B (index 1) or C (index 2) depending on order.
    Join(usize, Vec<String>),
    Filter(String),
    DistinctPair,
    Penalize,
    Reward,
}

impl ConstraintBuilder {
    /// Create a new constraint builder.
    pub fn new(name: String, weight: HardSoftScore) -> Self {
        Self {
            name,
            weight,
            class_idx: None,
            operations: Vec::new(),
        }
    }
}

#[pymethods]
impl ConstraintBuilder {
    /// Iterate over all entities of a class by index.
    fn for_each_idx(&mut self, class_idx: usize) -> Self {
        self.class_idx = Some(class_idx);
        self.operations.push(ConstraintOp::ForEach(class_idx));
        self.clone()
    }

    /// Join with another class by index.
    fn join_idx(&mut self, class_idx: usize, condition: &str) -> Self {
        self.operations
            .push(ConstraintOp::Join(class_idx, vec![condition.to_string()]));
        self.clone()
    }

    /// Iterate over all entities of a class.
    fn for_each(&mut self, class_name: &str, descriptor: &DescriptorRef) -> PyResult<Self> {
        let class_idx = descriptor
            .0
            .entity_class_index(class_name)
            .ok_or_else(|| PyValueError::new_err(format!("Unknown class: {}", class_name)))?;
        self.class_idx = Some(class_idx);
        self.operations.push(ConstraintOp::ForEach(class_idx));
        Ok(self.clone())
    }

    /// Join with another class.
    #[pyo3(signature = (class_name, *conditions, descriptor))]
    fn join(
        &mut self,
        class_name: &str,
        conditions: &Bound<'_, pyo3::types::PyTuple>,
        descriptor: &DescriptorRef,
    ) -> PyResult<Self> {
        let class_idx = descriptor
            .0
            .entity_class_index(class_name)
            .ok_or_else(|| PyValueError::new_err(format!("Unknown class: {}", class_name)))?;
        let conds: Vec<String> = conditions
            .iter()
            .map(|c| c.extract::<String>())
            .collect::<PyResult<Vec<_>>>()?;
        self.operations.push(ConstraintOp::Join(class_idx, conds));
        Ok(self.clone())
    }

    /// Filter tuples using a predicate expression.
    fn filter(&mut self, predicate: &str) -> Self {
        self.operations
            .push(ConstraintOp::Filter(predicate.to_string()));
        self.clone()
    }

    /// Filter to distinct pairs (A < B to avoid duplicates).
    fn distinct_pair(&mut self) -> Self {
        self.operations.push(ConstraintOp::DistinctPair);
        self.clone()
    }

    /// Penalize matching tuples.
    fn penalize(&mut self) -> Self {
        self.operations.push(ConstraintOp::Penalize);
        self.clone()
    }

    /// Reward matching tuples.
    fn reward(&mut self) -> Self {
        self.operations.push(ConstraintOp::Reward);
        self.clone()
    }
}

/// Helper struct to pass descriptor reference to constraint builder methods.
#[pyclass]
pub struct DescriptorRef(pub DynamicDescriptor);

/// Build a boxed constraint from a ConstraintBuilder.
///
/// This function tracks which class each parameter (A, B, C) refers to
/// based on the order of ForEach and Join operations, enabling correct
/// field lookup when multiple classes have fields with the same name.
pub fn build_constraint(
    builder: &ConstraintBuilder,
    descriptor: &DynamicDescriptor,
) -> PyResult<
    Box<
        dyn solverforge_scoring::api::constraint_set::IncrementalConstraint<
                solverforge_dynamic::DynamicSolution,
                solverforge_core::score::HardSoftScore,
            > + Send
            + Sync,
    >,
> {
    let mut constraint = DynamicConstraint::new(builder.name.clone());

    // Track which class index each parameter (A=0, B=1, C=2) refers to.
    // ForEach sets param 0, first Join sets param 1, second Join sets param 2.
    let mut param_to_class: Vec<usize> = Vec::new();

    for op in &builder.operations {
        match op {
            ConstraintOp::ForEach(class_idx) => {
                // ForEach always sets parameter A (index 0)
                param_to_class.clear();
                param_to_class.push(*class_idx);
                constraint = constraint.for_each(*class_idx);
            }
            ConstraintOp::Join(class_idx, conditions) => {
                // Join adds the next parameter (B=1 or C=2)
                param_to_class.push(*class_idx);
                let mut exprs = Vec::new();
                for cond in conditions {
                    exprs.push(parse_expr(cond, &param_to_class, descriptor)?);
                }
                constraint = constraint.join(*class_idx, exprs);
            }
            ConstraintOp::Filter(predicate) => {
                let expr = parse_expr(predicate, &param_to_class, descriptor)?;
                constraint = constraint.filter(expr);
            }
            ConstraintOp::DistinctPair => {
                // Use column < column for queens (field 0)
                let expr = Expr::lt(Expr::field(0, 0), Expr::field(1, 0));
                constraint = constraint.distinct_pair(expr);
            }
            ConstraintOp::Penalize => {
                constraint = constraint.penalize(builder.weight);
            }
            ConstraintOp::Reward => {
                constraint = constraint.reward(builder.weight);
            }
        }
    }

    // Build the constraint with the descriptor
    Ok(constraint.build(descriptor.clone()))
}

/// Parse a simple expression string.
///
/// `param_to_class` maps parameter indices (0=A, 1=B, 2=C) to their entity class indices.
/// This allows correct field lookup when multiple classes have fields with the same name.
pub fn parse_expr(
    expr_str: &str,
    param_to_class: &[usize],
    descriptor: &DynamicDescriptor,
) -> PyResult<Expr> {
    let expr_str = expr_str.trim();

    // Handle comparison operators
    for (op, constructor) in [
        ("==", Expr::eq as fn(Expr, Expr) -> Expr),
        ("!=", Expr::ne as fn(Expr, Expr) -> Expr),
        ("<=", Expr::le as fn(Expr, Expr) -> Expr),
        (">=", Expr::ge as fn(Expr, Expr) -> Expr),
        ("<", Expr::lt as fn(Expr, Expr) -> Expr),
        (">", Expr::gt as fn(Expr, Expr) -> Expr),
    ] {
        if let Some(pos) = expr_str.find(op) {
            let left = expr_str[..pos].trim();
            let right = expr_str[pos + op.len()..].trim();
            return Ok(constructor(
                parse_simple_expr(left, param_to_class, descriptor)?,
                parse_simple_expr(right, param_to_class, descriptor)?,
            ));
        }
    }

    parse_simple_expr(expr_str, param_to_class, descriptor)
}

/// Parse a simple expression (field reference or literal).
///
/// `param_to_class` maps parameter indices (0=A, 1=B, 2=C) to their entity class indices.
/// This allows correct field lookup when multiple classes have fields with the same name.
pub fn parse_simple_expr(
    expr_str: &str,
    param_to_class: &[usize],
    descriptor: &DynamicDescriptor,
) -> PyResult<Expr> {
    let expr_str = expr_str.trim();

    // Check for arithmetic expressions
    for (op, constructor) in [
        (" - ", Expr::sub as fn(Expr, Expr) -> Expr),
        (" + ", Expr::add as fn(Expr, Expr) -> Expr),
    ] {
        if let Some(pos) = expr_str.find(op) {
            let left = expr_str[..pos].trim();
            let right = expr_str[pos + op.len()..].trim();
            return Ok(constructor(
                parse_simple_expr(left, param_to_class, descriptor)?,
                parse_simple_expr(right, param_to_class, descriptor)?,
            ));
        }
    }

    // Check for integer literal
    if let Ok(v) = expr_str.parse::<i64>() {
        return Ok(Expr::int(v));
    }

    // Check for field reference: "A.field" or "B.field"
    if let Some(dot_pos) = expr_str.find('.') {
        let param_str = &expr_str[..dot_pos];
        let field_name = &expr_str[dot_pos + 1..];

        let param_idx: usize = match param_str {
            "A" | "a" | "0" => 0,
            "B" | "b" | "1" => 1,
            "C" | "c" | "2" => 2,
            _ => {
                return Err(PyValueError::new_err(format!(
                    "Unknown parameter: {}",
                    param_str
                )))
            }
        };

        // Look up the class index for this parameter
        let class_idx = param_to_class.get(param_idx).ok_or_else(|| {
            PyValueError::new_err(format!(
                "Parameter {} not yet defined (need ForEach or Join first)",
                param_str
            ))
        })?;

        // Get the class definition for this parameter's class
        let class_def = descriptor.entity_classes.get(*class_idx).ok_or_else(|| {
            PyValueError::new_err(format!("Invalid class index: {}", class_idx))
        })?;

        // Look up field in the correct class (not all classes)
        let field_idx = class_def.field_index(field_name).ok_or_else(|| {
            PyValueError::new_err(format!(
                "Unknown field '{}' in class '{}'",
                field_name, class_def.name
            ))
        })?;

        return Ok(Expr::field(param_idx, field_idx));
    }

    // Check for simple field name (assume param 0, use first class if available)
    if let Some(&class_idx) = param_to_class.first() {
        if let Some(class_def) = descriptor.entity_classes.get(class_idx) {
            if let Some(field_idx) = class_def.field_index(expr_str) {
                return Ok(Expr::field(0, field_idx));
            }
        }
    }

    Err(PyValueError::new_err(format!(
        "Cannot parse expression: {}",
        expr_str
    )))
}

// Tests for cross-class constraint field resolution are in
// solverforge-dynamic/src/constraint/tests.rs as the Python bindings
// cannot run cargo tests directly (pyo3 extension-module requires
// Python at link time). The functional tests verify that:
// 1. Same-named fields in different classes resolve to correct field indices
// 2. Parameter-to-class mapping correctly routes A.field, B.field, C.field
// 3. Error messages correctly identify the class when fields are unknown
