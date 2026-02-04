//! Async solver manager for Python API.

use std::time::Duration;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use solverforge_core::score::HardSoftScore;
use solverforge_dynamic::{
    DynamicConstraintSet, DynamicDescriptor, DynamicEntity, DynamicSolution, DynamicSolverManager,
    DynamicValue, EntityClassDef, FieldDef, SolveConfig, SolveStatus, ValueRangeDef,
};

use crate::constraint_builder::{parse_expr, ConstraintBuilder, ConstraintOp};
use crate::convert::{parse_field_type, parse_weight, py_to_dynamic};
use crate::solve_result::PySolveResult;

// Python wrapper for async solver manager.
#[pyclass]
pub struct SolverManager {
    inner: Option<DynamicSolverManager>,
    descriptor: DynamicDescriptor,
    entities: Vec<Vec<DynamicEntity>>,
    constraints: Vec<ConstraintBuilder>,
}

#[pymethods]
impl SolverManager {
    // Creates a new SolverManager.
    #[new]
    fn new() -> Self {
        Self {
            inner: Some(DynamicSolverManager::new()),
            descriptor: DynamicDescriptor::new(),
            entities: Vec::new(),
            constraints: Vec::new(),
        }
    }

    // Define an entity class (same as Solver).
    #[pyo3(signature = (name, fields))]
    fn entity_class(&mut self, name: &str, fields: &Bound<'_, PyList>) -> PyResult<()> {
        let mut field_defs = Vec::new();

        for field_spec in fields.iter() {
            let tuple = field_spec.downcast::<pyo3::types::PyTuple>()?;
            let field_name: String = tuple.get_item(0)?.extract()?;
            let field_type_str: String = tuple.get_item(1)?.extract()?;
            let field_type = parse_field_type(&field_type_str)?;

            let field_def = if tuple.len() > 2 {
                let options_item = tuple.get_item(2)?;
                let options = options_item.downcast::<PyDict>()?;
                if options.get_item("planning_variable")?.is_some() {
                    let value_range: String = options
                        .get_item("value_range")?
                        .ok_or_else(|| {
                            PyValueError::new_err("Planning variable must have 'value_range'")
                        })?
                        .extract()?;
                    FieldDef::planning_variable(field_name, field_type, value_range)
                } else {
                    FieldDef::new(field_name, field_type)
                }
            } else {
                FieldDef::new(field_name, field_type)
            };

            field_defs.push(field_def);
        }

        self.descriptor
            .add_entity_class(EntityClassDef::new(name, field_defs));
        self.entities.push(Vec::new());

        Ok(())
    }

    // Define an integer range.
    fn int_range(&mut self, name: &str, start: i64, end: i64) {
        self.descriptor
            .add_value_range(name, ValueRangeDef::int_range(start, end));
    }

    // Add entities to a class.
    fn add_entities(&mut self, class_name: &str, data: &Bound<'_, PyList>) -> PyResult<()> {
        let class_idx = self
            .descriptor
            .entity_class_index(class_name)
            .ok_or_else(|| {
                PyValueError::new_err(format!("Unknown entity class: {}", class_name))
            })?;

        let class_def = &self.descriptor.entity_classes[class_idx];
        let mut next_id = self.entities[class_idx].len() as i64;

        for item in data.iter() {
            let dict = item.downcast::<PyDict>()?;
            let mut fields = Vec::new();

            for field_def in &class_def.fields {
                let value = if let Some(py_val) = dict.get_item(&*field_def.name)? {
                    py_to_dynamic(&py_val)?
                } else {
                    DynamicValue::None
                };
                fields.push(value);
            }

            self.entities[class_idx].push(DynamicEntity::new(next_id, fields));
            next_id += 1;
        }

        Ok(())
    }

    // Start defining a constraint.
    fn constraint(&self, name: &str, weight: &str) -> PyResult<ConstraintBuilder> {
        let weight = parse_weight(weight)?;
        Ok(ConstraintBuilder {
            name: name.to_string(),
            weight,
            class_idx: None,
            operations: Vec::new(),
        })
    }

    /// Add a completed constraint.
    fn add_constraint(&mut self, builder: ConstraintBuilder) {
        self.constraints.push(builder);
    }

    /// Start solving asynchronously.
    #[pyo3(signature = (time_limit_seconds = 30))]
    fn solve_async(&mut self, time_limit_seconds: u64) -> PyResult<()> {
        // Build the solution
        let mut solution = DynamicSolution::new(self.descriptor.clone());
        for (class_idx, entities) in self.entities.iter().enumerate() {
            for entity in entities {
                solution.add_entity(class_idx, entity.clone());
            }
        }

        // Build constraints
        let mut constraint_set = DynamicConstraintSet::new();
        for builder in &self.constraints {
            let constraint = self.build_constraint_internal(builder)?;
            constraint_set.add(constraint);
        }

        let config = SolveConfig::with_time_limit(Duration::from_secs(time_limit_seconds));

        if let Some(ref mut manager) = self.inner {
            manager.solve_async(solution, constraint_set, config);
        }

        Ok(())
    }

    // Get current solve status.
    fn get_status(&self) -> String {
        if let Some(ref manager) = self.inner {
            match manager.status() {
                SolveStatus::NotStarted => "NOT_SOLVING".to_string(),
                SolveStatus::Solving => "SOLVING".to_string(),
                SolveStatus::Terminated => "TERMINATED".to_string(),
            }
        } else {
            "NOT_SOLVING".to_string()
        }
    }

    // Get the best solution found so far.
    fn get_best_solution(&self) -> PyResult<Option<PySolveResult>> {
        if let Some(ref manager) = self.inner {
            if let Some(solution) = manager.get_best_solution() {
                let score = solution.score.unwrap_or(HardSoftScore::ZERO);
                return Ok(Some(PySolveResult::new(
                    format!("{}hard/{}soft", score.hard(), score.soft()),
                    score.hard(),
                    score.soft(),
                    score.hard() >= 0,
                    0, // Not available from snapshot
                    0,
                    0,
                    solution,
                )));
            }
        }
        Ok(None)
    }

    // Request termination of the solve.
    fn terminate(&mut self) {
        if let Some(ref mut manager) = self.inner {
            manager.terminate();
        }
    }

    // Check if termination was requested.
    fn is_terminating(&self) -> bool {
        if let Some(ref manager) = self.inner {
            manager.is_terminating()
        } else {
            false
        }
    }
}

impl SolverManager {
    fn build_constraint_internal(
        &self,
        builder: &ConstraintBuilder,
    ) -> PyResult<
        Box<
            dyn solverforge_scoring::api::constraint_set::IncrementalConstraint<
                    solverforge_dynamic::DynamicSolution,
                    solverforge_core::score::HardSoftScore,
                > + Send
                + Sync,
        >,
    > {
        use solverforge_dynamic::{DynamicConstraint, Expr};

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
                        exprs.push(parse_expr(cond, &param_to_class, &self.descriptor)?);
                    }
                    constraint = constraint.join(*class_idx, exprs);
                }
                ConstraintOp::Filter(predicate) => {
                    let expr = parse_expr(predicate, &param_to_class, &self.descriptor)?;
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

        // Build the constraint with the descriptor
        Ok(constraint.build(self.descriptor.clone()))
    }
}
