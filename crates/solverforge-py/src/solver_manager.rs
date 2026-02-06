//! SolverManager: the single Python entry point for SolverForge.

use std::time::Duration;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use solverforge_core::score::HardSoftScore;
use solverforge_dynamic::{
    DynamicConstraintSet, DynamicDescriptor, DynamicEntity, DynamicSolution, DynamicSolverManager,
    DynamicValue, EntityClassDef, FieldDef, SolveConfig, SolveStatus, ValueRangeDef,
};

use crate::constraint_builder::{build_constraint, ConstraintBuilder};
use crate::convert::{parse_field_type, parse_weight, py_to_dynamic};
use crate::solve_result::PySolveResult;

#[pyclass]
pub struct SolverManager {
    inner: Option<DynamicSolverManager>,
    pub(crate) descriptor: DynamicDescriptor,
    entities: Vec<Vec<DynamicEntity>>,
    constraints: Vec<ConstraintBuilder>,
}

#[pymethods]
impl SolverManager {
    #[new]
    fn new() -> Self {
        Self {
            inner: Some(DynamicSolverManager::new()),
            descriptor: DynamicDescriptor::new(),
            entities: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Define an entity class with fields.
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

    /// Define an integer range [start, end).
    fn int_range(&mut self, name: &str, start: i64, end: i64) {
        self.descriptor
            .add_value_range(name, ValueRangeDef::int_range(start, end));
    }

    /// Define a value range with explicit values.
    fn value_range(&mut self, name: &str, values: &Bound<'_, PyList>) -> PyResult<()> {
        let dynamic_values: PyResult<Vec<_>> = values.iter().map(|v| py_to_dynamic(&v)).collect();
        self.descriptor
            .add_value_range(name, ValueRangeDef::Explicit(dynamic_values?));
        Ok(())
    }

    /// Add entities to a class.
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

    /// Create a new constraint builder.
    fn constraint(&self, name: &str, weight: &str) -> PyResult<ConstraintBuilder> {
        let weight = parse_weight(weight)?;
        Ok(ConstraintBuilder::new(name.to_string(), weight))
    }

    /// Add a completed constraint.
    fn add_constraint(&mut self, builder: ConstraintBuilder) {
        self.constraints.push(builder);
    }

    /// Start solving. Launches a background Rust thread and returns immediately.
    #[pyo3(signature = (time_limit_seconds = 30))]
    fn solve(&mut self, time_limit_seconds: u64) -> PyResult<()> {
        let mut solution = DynamicSolution::new(self.descriptor.clone());
        for (class_idx, entities) in self.entities.iter().enumerate() {
            for entity in entities {
                solution.add_entity(class_idx, entity.clone());
            }
        }

        let mut constraint_set = DynamicConstraintSet::new();
        for builder in &self.constraints {
            let constraint = build_constraint(builder, &self.descriptor)?;
            constraint_set.add(constraint);
        }

        let config = SolveConfig::with_time_limit(Duration::from_secs(time_limit_seconds));

        if let Some(ref mut manager) = self.inner {
            manager.solve_async(solution, constraint_set, config);
        }

        Ok(())
    }

    /// Get current solve status.
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

    /// Get the best solution found so far.
    fn get_best_solution(&self) -> PyResult<Option<PySolveResult>> {
        if let Some(ref manager) = self.inner {
            if let Some(solution) = manager.get_best_solution() {
                let score = solution.score.unwrap_or(HardSoftScore::ZERO);
                return Ok(Some(PySolveResult::new(
                    format!("{}hard/{}soft", score.hard(), score.soft()),
                    score.hard(),
                    score.soft(),
                    score.hard() >= 0,
                    0,
                    0,
                    0,
                    solution,
                )));
            }
        }
        Ok(None)
    }

    /// Request termination of the solve.
    fn terminate(&mut self) {
        if let Some(ref mut manager) = self.inner {
            manager.terminate();
        }
    }

    /// Check if termination was requested.
    fn is_terminating(&self) -> bool {
        if let Some(ref manager) = self.inner {
            manager.is_terminating()
        } else {
            false
        }
    }
}
