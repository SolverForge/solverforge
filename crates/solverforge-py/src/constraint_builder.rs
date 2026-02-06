//! Constraint builder for Python API.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use solverforge_core::score::HardSoftScore;
use solverforge_dynamic::{DynamicConstraint, DynamicDescriptor, Expr};

use crate::solver_manager::SolverManager;

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
    /// Iterate over all entities of a class by name.
    fn for_each(&mut self, class_name: &str, solver: &SolverManager) -> PyResult<Self> {
        let class_idx = solver
            .descriptor
            .entity_class_index(class_name)
            .ok_or_else(|| PyValueError::new_err(format!("Unknown class: {}", class_name)))?;
        self.class_idx = Some(class_idx);
        self.operations.push(ConstraintOp::ForEach(class_idx));
        Ok(self.clone())
    }

    /// Iterate over all entities of a class by index.
    fn for_each_idx(&mut self, class_idx: usize) -> Self {
        self.class_idx = Some(class_idx);
        self.operations.push(ConstraintOp::ForEach(class_idx));
        self.clone()
    }

    /// Join with another class by name.
    #[pyo3(signature = (class_name, *conditions, solver))]
    fn join(
        &mut self,
        class_name: &str,
        conditions: &Bound<'_, pyo3::types::PyTuple>,
        solver: &SolverManager,
    ) -> PyResult<Self> {
        let class_idx = solver
            .descriptor
            .entity_class_index(class_name)
            .ok_or_else(|| PyValueError::new_err(format!("Unknown class: {}", class_name)))?;
        let conds: Vec<String> = conditions
            .iter()
            .map(|c| c.extract::<String>())
            .collect::<PyResult<Vec<_>>>()?;
        self.operations.push(ConstraintOp::Join(class_idx, conds));
        Ok(self.clone())
    }

    /// Join with another class by index.
    fn join_idx(&mut self, class_idx: usize, condition: &str) -> Self {
        self.operations
            .push(ConstraintOp::Join(class_idx, vec![condition.to_string()]));
        self.clone()
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

/// Build a boxed incremental constraint from a ConstraintBuilder.
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
    let mut param_to_class: Vec<usize> = Vec::new();

    for op in &builder.operations {
        match op {
            ConstraintOp::ForEach(class_idx) => {
                param_to_class.clear();
                param_to_class.push(*class_idx);
                constraint = constraint.for_each(*class_idx);
            }
            ConstraintOp::Join(class_idx, conditions) => {
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

    Ok(constraint.build(descriptor.clone()))
}

/// Parse an expression string into an Expr AST node.
pub fn parse_expr(
    expr_str: &str,
    param_to_class: &[usize],
    descriptor: &DynamicDescriptor,
) -> PyResult<Expr> {
    let expr_str = expr_str.trim();

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

fn parse_simple_expr(
    expr_str: &str,
    param_to_class: &[usize],
    descriptor: &DynamicDescriptor,
) -> PyResult<Expr> {
    let expr_str = expr_str.trim();

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

    if let Ok(v) = expr_str.parse::<i64>() {
        return Ok(Expr::int(v));
    }

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

        let class_idx = param_to_class.get(param_idx).ok_or_else(|| {
            PyValueError::new_err(format!(
                "Parameter {} not yet defined (need for_each or join first)",
                param_str
            ))
        })?;

        let class_def = descriptor
            .entity_classes
            .get(*class_idx)
            .ok_or_else(|| PyValueError::new_err(format!("Invalid class index: {}", class_idx)))?;

        let field_idx = class_def.field_index(field_name).ok_or_else(|| {
            PyValueError::new_err(format!(
                "Unknown field '{}' in class '{}'",
                field_name, class_def.name
            ))
        })?;

        return Ok(Expr::field(param_idx, field_idx));
    }

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
