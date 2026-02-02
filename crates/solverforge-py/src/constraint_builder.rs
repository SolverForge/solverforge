//! Constraint builder for Python API.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use solverforge_core::score::HardSoftScore;
use solverforge_dynamic::{DynamicConstraint, DynamicDescriptor, Expr};

use crate::convert::parse_weight;

/// A constraint builder for defining constraint pipelines.
#[pyclass]
#[derive(Clone)]
pub struct ConstraintBuilder {
    pub(crate) name: String,
    pub(crate) weight: HardSoftScore,
    pub(crate) class_idx: Option<usize>,
    pub(crate) operations: Vec<ConstraintOp>,
}

#[derive(Clone)]
pub enum ConstraintOp {
    ForEach(usize),
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

/// Build a DynamicConstraint from a ConstraintBuilder.
pub fn build_constraint(
    builder: &ConstraintBuilder,
    descriptor: &DynamicDescriptor,
) -> PyResult<DynamicConstraint> {
    let mut constraint = DynamicConstraint::new(builder.name.clone());

    for op in &builder.operations {
        match op {
            ConstraintOp::ForEach(class_idx) => {
                constraint = constraint.for_each(*class_idx);
            }
            ConstraintOp::Join(class_idx, conditions) => {
                let mut exprs = Vec::new();
                for cond in conditions {
                    exprs.push(parse_expr(cond, builder.class_idx, descriptor)?);
                }
                constraint = constraint.join(*class_idx, exprs);
            }
            ConstraintOp::Filter(predicate) => {
                let expr = parse_expr(predicate, builder.class_idx, descriptor)?;
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

    Ok(constraint)
}

/// Parse a simple expression string.
pub fn parse_expr(
    expr_str: &str,
    _class_idx: Option<usize>,
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
                parse_simple_expr(left, descriptor)?,
                parse_simple_expr(right, descriptor)?,
            ));
        }
    }

    parse_simple_expr(expr_str, descriptor)
}

/// Parse a simple expression (field reference or literal).
pub fn parse_simple_expr(expr_str: &str, descriptor: &DynamicDescriptor) -> PyResult<Expr> {
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
                parse_simple_expr(left, descriptor)?,
                parse_simple_expr(right, descriptor)?,
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

        let param_idx = match param_str {
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

        // Find field index - search all entity classes
        for class_def in &descriptor.entity_classes {
            if let Some(field_idx) = class_def.field_index(field_name) {
                return Ok(Expr::field(param_idx, field_idx));
            }
        }

        return Err(PyValueError::new_err(format!(
            "Unknown field: {}",
            field_name
        )));
    }

    // Check for simple field name (assume param 0)
    for class_def in &descriptor.entity_classes {
        if let Some(field_idx) = class_def.field_index(expr_str) {
            return Ok(Expr::field(0, field_idx));
        }
    }

    Err(PyValueError::new_err(format!(
        "Cannot parse expression: {}",
        expr_str
    )))
}
