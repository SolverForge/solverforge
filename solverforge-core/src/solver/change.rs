//! ProblemChange API for real-time planning.
//!
//! This module provides the ability to dynamically add, remove, or modify
//! planning entities and problem facts during solving.
//!
//! # Architecture
//!
//! Problem changes are queued and applied between solver moves. When a problem
//! change is applied:
//!
//! 1. The solver clones the current best solution
//! 2. The problem change is applied via the ProblemChangeDirector
//! 3. Variable listeners are triggered
//! 4. The score is recalculated
//! 5. Solving resumes from the modified state
//!
//! # Example
//!
//! ```ignore
//! use solverforge_core::solver::change::{ProblemChange, ProblemChangeDirector};
//!
//! struct AddEntityChange {
//!     entity_id: String,
//!     entity_data: serde_json::Value,
//! }
//!
//! impl ProblemChange for AddEntityChange {
//!     fn do_change(&self, solution: &mut serde_json::Value, director: &mut dyn ProblemChangeDirector) {
//!         director.add_entity(&self.entity_id, self.entity_data.clone(), |entities| {
//!             entities.as_array_mut().unwrap().push(self.entity_data.clone());
//!         });
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// A problem change represents a modification to the planning solution during solving.
///
/// Problem changes allow real-time planning by dynamically adding, removing, or modifying
/// planning entities and problem facts while the solver is running.
///
/// All modifications to the solution must be performed through the `ProblemChangeDirector`
/// to ensure that variable listeners are properly notified and the score is correctly updated.
pub trait ProblemChange: Send + Sync + Debug {
    /// Apply the change to the working solution.
    ///
    /// # Arguments
    ///
    /// * `solution` - The working solution to modify (as JSON value)
    /// * `director` - The director through which all modifications must be made
    fn do_change(&self, solution: &mut serde_json::Value, director: &mut dyn ProblemChangeDirector);

    /// Convert to a serializable DTO for transmission to the solver service.
    fn to_dto(&self) -> ProblemChangeDto;
}

/// Type alias for consumer functions used by ProblemChangeDirector.
pub type ChangeConsumer = Box<dyn FnOnce(&mut serde_json::Value) + Send>;

/// Director for applying problem changes to the working solution.
///
/// All modifications to the solution during a problem change must go through
/// this director to ensure proper tracking and variable listener notification.
pub trait ProblemChangeDirector: Send {
    /// Add a new planning entity to the solution.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - Unique identifier for the entity (typically the @PlanningId value)
    /// * `entity` - The entity data as JSON
    /// * `consumer` - Function that adds the entity to the appropriate collection in the solution
    fn add_entity(&mut self, entity_id: &str, entity: serde_json::Value, consumer: ChangeConsumer);

    /// Remove a planning entity from the solution.
    ///
    /// The entity is looked up by its ID and the working copy is passed to the consumer.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The ID of the entity to remove
    /// * `consumer` - Function that removes the entity from the appropriate collection
    fn remove_entity(&mut self, entity_id: &str, consumer: ChangeConsumer);

    /// Change a planning variable on an entity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The ID of the entity to modify
    /// * `variable_name` - Name of the planning variable to change
    /// * `consumer` - Function that updates the variable value
    fn change_variable(&mut self, entity_id: &str, variable_name: &str, consumer: ChangeConsumer);

    /// Add a new problem fact to the solution.
    ///
    /// # Arguments
    ///
    /// * `fact_id` - Unique identifier for the problem fact
    /// * `fact` - The problem fact data as JSON
    /// * `consumer` - Function that adds the fact to the appropriate collection
    fn add_problem_fact(
        &mut self,
        fact_id: &str,
        fact: serde_json::Value,
        consumer: ChangeConsumer,
    );

    /// Remove a problem fact from the solution.
    ///
    /// # Arguments
    ///
    /// * `fact_id` - The ID of the problem fact to remove
    /// * `consumer` - Function that removes the fact from the appropriate collection
    fn remove_problem_fact(&mut self, fact_id: &str, consumer: ChangeConsumer);

    /// Change a property on an entity or problem fact.
    ///
    /// # Arguments
    ///
    /// * `object_id` - The ID of the entity or problem fact
    /// * `consumer` - Function that updates the property
    fn change_problem_property(&mut self, object_id: &str, consumer: ChangeConsumer);

    /// Look up a working object by its external ID.
    ///
    /// Returns a clone of the working object, or an error if not found.
    ///
    /// # Arguments
    ///
    /// * `external_id` - The ID of the object to look up
    fn look_up_working_object_or_fail(
        &self,
        external_id: &str,
    ) -> Result<serde_json::Value, ProblemChangeError>;

    /// Look up a working object by its external ID.
    ///
    /// Returns `None` if the object is not found, rather than failing.
    ///
    /// # Arguments
    ///
    /// * `external_id` - The ID of the object to look up
    fn look_up_working_object(&self, external_id: &str) -> Option<serde_json::Value>;

    /// Trigger variable listeners for changes made so far.
    ///
    /// This is called automatically after the entire problem change is processed,
    /// but can be called manually to trigger listeners mid-change.
    fn update_shadow_variables(&mut self);
}

/// Error type for problem change operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProblemChangeError {
    /// The requested object was not found in the working solution.
    ObjectNotFound { id: String },
    /// The object type is not supported for lookup.
    UnsupportedType { type_name: String },
    /// A validation error occurred during the change.
    ValidationError { message: String },
    /// The change could not be applied.
    ApplicationError { message: String },
}

impl std::fmt::Display for ProblemChangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProblemChangeError::ObjectNotFound { id } => {
                write!(f, "Object not found: {}", id)
            }
            ProblemChangeError::UnsupportedType { type_name } => {
                write!(f, "Unsupported type for lookup: {}", type_name)
            }
            ProblemChangeError::ValidationError { message } => {
                write!(f, "Validation error: {}", message)
            }
            ProblemChangeError::ApplicationError { message } => {
                write!(f, "Application error: {}", message)
            }
        }
    }
}

impl std::error::Error for ProblemChangeError {}

/// Serializable DTO for transmitting problem changes to the solver service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProblemChangeDto {
    /// Add a planning entity.
    #[serde(rename = "addEntity")]
    AddEntity {
        /// The entity type/class name.
        entity_class: String,
        /// Unique identifier for the entity.
        entity_id: String,
        /// The entity data as JSON.
        entity: serde_json::Value,
        /// The field path in the solution where entities are stored.
        collection_path: String,
    },

    /// Remove a planning entity.
    #[serde(rename = "removeEntity")]
    RemoveEntity {
        /// The entity type/class name.
        entity_class: String,
        /// Unique identifier of the entity to remove.
        entity_id: String,
        /// The field path in the solution where entities are stored.
        collection_path: String,
    },

    /// Change a planning variable.
    #[serde(rename = "changeVariable")]
    ChangeVariable {
        /// The entity type/class name.
        entity_class: String,
        /// Unique identifier of the entity.
        entity_id: String,
        /// Name of the planning variable to change.
        variable_name: String,
        /// The new value for the variable (null to unassign).
        new_value: serde_json::Value,
    },

    /// Add a problem fact.
    #[serde(rename = "addProblemFact")]
    AddProblemFact {
        /// The problem fact type/class name.
        fact_class: String,
        /// Unique identifier for the fact.
        fact_id: String,
        /// The problem fact data as JSON.
        fact: serde_json::Value,
        /// The field path in the solution where facts are stored.
        collection_path: String,
    },

    /// Remove a problem fact.
    #[serde(rename = "removeProblemFact")]
    RemoveProblemFact {
        /// The problem fact type/class name.
        fact_class: String,
        /// Unique identifier of the fact to remove.
        fact_id: String,
        /// The field path in the solution where facts are stored.
        collection_path: String,
    },

    /// Change a property on an entity or problem fact.
    #[serde(rename = "changeProblemProperty")]
    ChangeProblemProperty {
        /// The object type/class name.
        object_class: String,
        /// Unique identifier of the object.
        object_id: String,
        /// Name of the property to change.
        property_name: String,
        /// The new value for the property.
        new_value: serde_json::Value,
    },

    /// A batch of problem changes to apply atomically.
    #[serde(rename = "batch")]
    Batch {
        /// The changes to apply in order.
        changes: Vec<ProblemChangeDto>,
    },
}

impl ProblemChangeDto {
    /// Create an AddEntity change.
    pub fn add_entity(
        entity_class: impl Into<String>,
        entity_id: impl Into<String>,
        entity: serde_json::Value,
        collection_path: impl Into<String>,
    ) -> Self {
        ProblemChangeDto::AddEntity {
            entity_class: entity_class.into(),
            entity_id: entity_id.into(),
            entity,
            collection_path: collection_path.into(),
        }
    }

    /// Create a RemoveEntity change.
    pub fn remove_entity(
        entity_class: impl Into<String>,
        entity_id: impl Into<String>,
        collection_path: impl Into<String>,
    ) -> Self {
        ProblemChangeDto::RemoveEntity {
            entity_class: entity_class.into(),
            entity_id: entity_id.into(),
            collection_path: collection_path.into(),
        }
    }

    /// Create a ChangeVariable change.
    pub fn change_variable(
        entity_class: impl Into<String>,
        entity_id: impl Into<String>,
        variable_name: impl Into<String>,
        new_value: serde_json::Value,
    ) -> Self {
        ProblemChangeDto::ChangeVariable {
            entity_class: entity_class.into(),
            entity_id: entity_id.into(),
            variable_name: variable_name.into(),
            new_value,
        }
    }

    /// Create an AddProblemFact change.
    pub fn add_problem_fact(
        fact_class: impl Into<String>,
        fact_id: impl Into<String>,
        fact: serde_json::Value,
        collection_path: impl Into<String>,
    ) -> Self {
        ProblemChangeDto::AddProblemFact {
            fact_class: fact_class.into(),
            fact_id: fact_id.into(),
            fact,
            collection_path: collection_path.into(),
        }
    }

    /// Create a RemoveProblemFact change.
    pub fn remove_problem_fact(
        fact_class: impl Into<String>,
        fact_id: impl Into<String>,
        collection_path: impl Into<String>,
    ) -> Self {
        ProblemChangeDto::RemoveProblemFact {
            fact_class: fact_class.into(),
            fact_id: fact_id.into(),
            collection_path: collection_path.into(),
        }
    }

    /// Create a ChangeProblemProperty change.
    pub fn change_problem_property(
        object_class: impl Into<String>,
        object_id: impl Into<String>,
        property_name: impl Into<String>,
        new_value: serde_json::Value,
    ) -> Self {
        ProblemChangeDto::ChangeProblemProperty {
            object_class: object_class.into(),
            object_id: object_id.into(),
            property_name: property_name.into(),
            new_value,
        }
    }

    /// Create a batch of changes.
    pub fn batch(changes: Vec<ProblemChangeDto>) -> Self {
        ProblemChangeDto::Batch { changes }
    }
}

/// Default implementation of ProblemChangeDirector for local use.
///
/// This implementation tracks changes and maintains an index of working objects
/// for lookup operations.
#[derive(Debug)]
pub struct DefaultProblemChangeDirector {
    /// Index of object IDs to their JSON values.
    object_index: std::collections::HashMap<String, serde_json::Value>,
    /// Record of changes made.
    changes: Vec<ChangeRecord>,
    /// Whether shadow variables need updating.
    shadow_variables_dirty: bool,
}

/// Record of a change made through the director.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeRecord {
    /// A planning entity was added.
    EntityAdded {
        /// The ID of the added entity.
        id: String,
    },
    /// A planning entity was removed.
    EntityRemoved {
        /// The ID of the removed entity.
        id: String,
    },
    /// A planning variable was changed.
    VariableChanged {
        /// The ID of the entity whose variable changed.
        entity_id: String,
        /// The name of the variable that changed.
        variable: String,
    },
    /// A problem fact was added.
    FactAdded {
        /// The ID of the added fact.
        id: String,
    },
    /// A problem fact was removed.
    FactRemoved {
        /// The ID of the removed fact.
        id: String,
    },
    /// A property on an entity or fact was changed.
    PropertyChanged {
        /// The ID of the object whose property changed.
        object_id: String,
    },
}

impl DefaultProblemChangeDirector {
    /// Create a new director with the given working solution.
    ///
    /// # Arguments
    ///
    /// * `solution` - The working solution JSON
    /// * `id_field` - The field name used for object IDs (typically "id")
    pub fn new(solution: &serde_json::Value, id_field: &str) -> Self {
        let mut object_index = std::collections::HashMap::new();
        Self::index_objects(solution, id_field, &mut object_index);

        Self {
            object_index,
            changes: Vec::new(),
            shadow_variables_dirty: false,
        }
    }

    /// Recursively index all objects with IDs in the solution.
    fn index_objects(
        value: &serde_json::Value,
        id_field: &str,
        index: &mut std::collections::HashMap<String, serde_json::Value>,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::String(id)) = map.get(id_field) {
                    index.insert(id.clone(), value.clone());
                }
                for v in map.values() {
                    Self::index_objects(v, id_field, index);
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    Self::index_objects(v, id_field, index);
                }
            }
            _ => {}
        }
    }

    /// Get the changes recorded by this director.
    pub fn changes(&self) -> &[ChangeRecord] {
        &self.changes
    }

    /// Check if any changes were made.
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Update the object index with a new or modified object.
    pub fn update_index(&mut self, id: String, value: serde_json::Value) {
        self.object_index.insert(id, value);
    }

    /// Remove an object from the index.
    pub fn remove_from_index(&mut self, id: &str) {
        self.object_index.remove(id);
    }
}

impl ProblemChangeDirector for DefaultProblemChangeDirector {
    fn add_entity(&mut self, entity_id: &str, entity: serde_json::Value, consumer: ChangeConsumer) {
        self.object_index
            .insert(entity_id.to_string(), entity.clone());
        self.changes.push(ChangeRecord::EntityAdded {
            id: entity_id.to_string(),
        });
        self.shadow_variables_dirty = true;

        // The consumer is expected to add the entity to a mutable reference to the solution
        // In practice, this is called with a reference to the appropriate collection
        let mut entity_copy = entity;
        consumer(&mut entity_copy);
    }

    fn remove_entity(&mut self, entity_id: &str, consumer: ChangeConsumer) {
        if let Some(mut entity) = self.object_index.remove(entity_id) {
            consumer(&mut entity);
            self.changes.push(ChangeRecord::EntityRemoved {
                id: entity_id.to_string(),
            });
            self.shadow_variables_dirty = true;
        }
    }

    fn change_variable(&mut self, entity_id: &str, variable_name: &str, consumer: ChangeConsumer) {
        if let Some(entity) = self.object_index.get_mut(entity_id) {
            consumer(entity);
            self.changes.push(ChangeRecord::VariableChanged {
                entity_id: entity_id.to_string(),
                variable: variable_name.to_string(),
            });
            self.shadow_variables_dirty = true;
        }
    }

    fn add_problem_fact(
        &mut self,
        fact_id: &str,
        fact: serde_json::Value,
        consumer: ChangeConsumer,
    ) {
        self.object_index.insert(fact_id.to_string(), fact.clone());
        self.changes.push(ChangeRecord::FactAdded {
            id: fact_id.to_string(),
        });

        let mut fact_copy = fact;
        consumer(&mut fact_copy);
    }

    fn remove_problem_fact(&mut self, fact_id: &str, consumer: ChangeConsumer) {
        if let Some(mut fact) = self.object_index.remove(fact_id) {
            consumer(&mut fact);
            self.changes.push(ChangeRecord::FactRemoved {
                id: fact_id.to_string(),
            });
        }
    }

    fn change_problem_property(&mut self, object_id: &str, consumer: ChangeConsumer) {
        if let Some(object) = self.object_index.get_mut(object_id) {
            consumer(object);
            self.changes.push(ChangeRecord::PropertyChanged {
                object_id: object_id.to_string(),
            });
        }
    }

    fn look_up_working_object_or_fail(
        &self,
        external_id: &str,
    ) -> Result<serde_json::Value, ProblemChangeError> {
        self.object_index.get(external_id).cloned().ok_or_else(|| {
            ProblemChangeError::ObjectNotFound {
                id: external_id.to_string(),
            }
        })
    }

    fn look_up_working_object(&self, external_id: &str) -> Option<serde_json::Value> {
        self.object_index.get(external_id).cloned()
    }

    fn update_shadow_variables(&mut self) {
        // In the actual implementation, this would trigger variable listeners
        // For now, we just mark that shadow variables have been updated
        self.shadow_variables_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Test ProblemChangeError display
    #[test]
    fn test_error_display_object_not_found() {
        let err = ProblemChangeError::ObjectNotFound {
            id: "entity1".to_string(),
        };
        assert_eq!(format!("{}", err), "Object not found: entity1");
    }

    #[test]
    fn test_error_display_unsupported_type() {
        let err = ProblemChangeError::UnsupportedType {
            type_name: "UnknownType".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Unsupported type for lookup: UnknownType"
        );
    }

    #[test]
    fn test_error_display_validation() {
        let err = ProblemChangeError::ValidationError {
            message: "ID cannot be empty".to_string(),
        };
        assert_eq!(format!("{}", err), "Validation error: ID cannot be empty");
    }

    #[test]
    fn test_error_display_application() {
        let err = ProblemChangeError::ApplicationError {
            message: "Failed to apply change".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Application error: Failed to apply change"
        );
    }

    // Test ProblemChangeDto constructors
    #[test]
    fn test_add_entity_dto() {
        let dto = ProblemChangeDto::add_entity(
            "Lesson",
            "lesson1",
            json!({"id": "lesson1", "subject": "Math"}),
            "lessons",
        );

        match dto {
            ProblemChangeDto::AddEntity {
                entity_class,
                entity_id,
                entity,
                collection_path,
            } => {
                assert_eq!(entity_class, "Lesson");
                assert_eq!(entity_id, "lesson1");
                assert_eq!(entity["subject"], "Math");
                assert_eq!(collection_path, "lessons");
            }
            _ => panic!("Expected AddEntity"),
        }
    }

    #[test]
    fn test_remove_entity_dto() {
        let dto = ProblemChangeDto::remove_entity("Lesson", "lesson1", "lessons");

        match dto {
            ProblemChangeDto::RemoveEntity {
                entity_class,
                entity_id,
                collection_path,
            } => {
                assert_eq!(entity_class, "Lesson");
                assert_eq!(entity_id, "lesson1");
                assert_eq!(collection_path, "lessons");
            }
            _ => panic!("Expected RemoveEntity"),
        }
    }

    #[test]
    fn test_change_variable_dto() {
        let dto = ProblemChangeDto::change_variable("Lesson", "lesson1", "room", json!("Room101"));

        match dto {
            ProblemChangeDto::ChangeVariable {
                entity_class,
                entity_id,
                variable_name,
                new_value,
            } => {
                assert_eq!(entity_class, "Lesson");
                assert_eq!(entity_id, "lesson1");
                assert_eq!(variable_name, "room");
                assert_eq!(new_value, json!("Room101"));
            }
            _ => panic!("Expected ChangeVariable"),
        }
    }

    #[test]
    fn test_change_variable_dto_null() {
        let dto =
            ProblemChangeDto::change_variable("Lesson", "lesson1", "room", serde_json::Value::Null);

        match dto {
            ProblemChangeDto::ChangeVariable { new_value, .. } => {
                assert!(new_value.is_null());
            }
            _ => panic!("Expected ChangeVariable"),
        }
    }

    #[test]
    fn test_add_problem_fact_dto() {
        let dto = ProblemChangeDto::add_problem_fact(
            "Room",
            "room1",
            json!({"id": "room1", "capacity": 30}),
            "rooms",
        );

        match dto {
            ProblemChangeDto::AddProblemFact {
                fact_class,
                fact_id,
                fact,
                collection_path,
            } => {
                assert_eq!(fact_class, "Room");
                assert_eq!(fact_id, "room1");
                assert_eq!(fact["capacity"], 30);
                assert_eq!(collection_path, "rooms");
            }
            _ => panic!("Expected AddProblemFact"),
        }
    }

    #[test]
    fn test_remove_problem_fact_dto() {
        let dto = ProblemChangeDto::remove_problem_fact("Room", "room1", "rooms");

        match dto {
            ProblemChangeDto::RemoveProblemFact {
                fact_class,
                fact_id,
                collection_path,
            } => {
                assert_eq!(fact_class, "Room");
                assert_eq!(fact_id, "room1");
                assert_eq!(collection_path, "rooms");
            }
            _ => panic!("Expected RemoveProblemFact"),
        }
    }

    #[test]
    fn test_change_problem_property_dto() {
        let dto = ProblemChangeDto::change_problem_property("Room", "room1", "capacity", json!(50));

        match dto {
            ProblemChangeDto::ChangeProblemProperty {
                object_class,
                object_id,
                property_name,
                new_value,
            } => {
                assert_eq!(object_class, "Room");
                assert_eq!(object_id, "room1");
                assert_eq!(property_name, "capacity");
                assert_eq!(new_value, json!(50));
            }
            _ => panic!("Expected ChangeProblemProperty"),
        }
    }

    #[test]
    fn test_batch_dto() {
        let changes = vec![
            ProblemChangeDto::add_entity("Lesson", "lesson1", json!({"id": "lesson1"}), "lessons"),
            ProblemChangeDto::change_variable("Lesson", "lesson1", "room", json!("Room101")),
        ];

        let dto = ProblemChangeDto::batch(changes);

        match dto {
            ProblemChangeDto::Batch { changes } => {
                assert_eq!(changes.len(), 2);
            }
            _ => panic!("Expected Batch"),
        }
    }

    // Test DTO serialization
    #[test]
    fn test_add_entity_dto_serialization() {
        let dto = ProblemChangeDto::add_entity(
            "Lesson",
            "lesson1",
            json!({"id": "lesson1", "subject": "Math"}),
            "lessons",
        );

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains(r#""type":"addEntity""#));
        assert!(json.contains(r#""entity_class":"Lesson""#));
        assert!(json.contains(r#""entity_id":"lesson1""#));

        let parsed: ProblemChangeDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, parsed);
    }

    #[test]
    fn test_remove_entity_dto_serialization() {
        let dto = ProblemChangeDto::remove_entity("Lesson", "lesson1", "lessons");

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains(r#""type":"removeEntity""#));

        let parsed: ProblemChangeDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, parsed);
    }

    #[test]
    fn test_change_variable_dto_serialization() {
        let dto = ProblemChangeDto::change_variable("Lesson", "lesson1", "room", json!("Room101"));

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains(r#""type":"changeVariable""#));
        assert!(json.contains(r#""variable_name":"room""#));

        let parsed: ProblemChangeDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, parsed);
    }

    #[test]
    fn test_batch_dto_serialization() {
        let dto = ProblemChangeDto::batch(vec![
            ProblemChangeDto::add_entity("Lesson", "l1", json!({"id": "l1"}), "lessons"),
            ProblemChangeDto::remove_entity("Lesson", "l2", "lessons"),
        ]);

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains(r#""type":"batch""#));
        assert!(json.contains(r#""changes""#));

        let parsed: ProblemChangeDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, parsed);
    }

    // Test DefaultProblemChangeDirector
    #[test]
    fn test_director_new_indexes_objects() {
        let solution = json!({
            "lessons": [
                {"id": "lesson1", "subject": "Math"},
                {"id": "lesson2", "subject": "English"}
            ],
            "rooms": [
                {"id": "room1", "capacity": 30}
            ]
        });

        let director = DefaultProblemChangeDirector::new(&solution, "id");

        assert!(director.look_up_working_object("lesson1").is_some());
        assert!(director.look_up_working_object("lesson2").is_some());
        assert!(director.look_up_working_object("room1").is_some());
        assert!(director.look_up_working_object("nonexistent").is_none());
    }

    #[test]
    fn test_director_look_up_or_fail() {
        let solution = json!({
            "entities": [{"id": "e1", "value": 10}]
        });

        let director = DefaultProblemChangeDirector::new(&solution, "id");

        let result = director.look_up_working_object_or_fail("e1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["value"], 10);

        let result = director.look_up_working_object_or_fail("nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            ProblemChangeError::ObjectNotFound { id } => {
                assert_eq!(id, "nonexistent");
            }
            _ => panic!("Expected ObjectNotFound error"),
        }
    }

    #[test]
    fn test_director_add_entity() {
        let solution = json!({"entities": []});
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        let new_entity = json!({"id": "e1", "value": 42});
        director.add_entity("e1", new_entity.clone(), Box::new(|_| {}));

        assert!(director.look_up_working_object("e1").is_some());
        assert!(director.has_changes());

        let changes = director.changes();
        assert_eq!(changes.len(), 1);
        matches!(&changes[0], ChangeRecord::EntityAdded { id } if id == "e1");
    }

    #[test]
    fn test_director_remove_entity() {
        let solution = json!({
            "entities": [{"id": "e1", "value": 42}]
        });
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        assert!(director.look_up_working_object("e1").is_some());

        director.remove_entity("e1", Box::new(|_| {}));

        assert!(director.look_up_working_object("e1").is_none());
        assert!(director.has_changes());
    }

    #[test]
    fn test_director_change_variable() {
        let solution = json!({
            "entities": [{"id": "e1", "room": null}]
        });
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        director.change_variable(
            "e1",
            "room",
            Box::new(|entity| {
                entity["room"] = json!("Room101");
            }),
        );

        let entity = director.look_up_working_object("e1").unwrap();
        assert_eq!(entity["room"], json!("Room101"));
        assert!(director.has_changes());
    }

    #[test]
    fn test_director_add_problem_fact() {
        let solution = json!({"facts": []});
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        let fact = json!({"id": "f1", "name": "Fact1"});
        director.add_problem_fact("f1", fact, Box::new(|_| {}));

        assert!(director.look_up_working_object("f1").is_some());
        assert!(director.has_changes());
    }

    #[test]
    fn test_director_remove_problem_fact() {
        let solution = json!({
            "facts": [{"id": "f1", "name": "Fact1"}]
        });
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        director.remove_problem_fact("f1", Box::new(|_| {}));

        assert!(director.look_up_working_object("f1").is_none());
        assert!(director.has_changes());
    }

    #[test]
    fn test_director_change_problem_property() {
        let solution = json!({
            "facts": [{"id": "f1", "capacity": 30}]
        });
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        director.change_problem_property(
            "f1",
            Box::new(|obj| {
                obj["capacity"] = json!(50);
            }),
        );

        let fact = director.look_up_working_object("f1").unwrap();
        assert_eq!(fact["capacity"], json!(50));
        assert!(director.has_changes());
    }

    #[test]
    fn test_director_update_shadow_variables() {
        let solution = json!({"entities": []});
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        director.add_entity("e1", json!({"id": "e1"}), Box::new(|_| {}));
        assert!(director.shadow_variables_dirty);

        director.update_shadow_variables();
        assert!(!director.shadow_variables_dirty);
    }

    #[test]
    fn test_director_no_changes_initially() {
        let solution = json!({"entities": []});
        let director = DefaultProblemChangeDirector::new(&solution, "id");

        assert!(!director.has_changes());
        assert!(director.changes().is_empty());
    }

    #[test]
    fn test_director_multiple_changes() {
        let solution = json!({
            "entities": [{"id": "e1", "room": null}],
            "rooms": [{"id": "r1", "capacity": 30}]
        });
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        director.add_entity("e2", json!({"id": "e2", "room": null}), Box::new(|_| {}));
        director.change_variable(
            "e1",
            "room",
            Box::new(|e| {
                e["room"] = json!("r1");
            }),
        );
        director.remove_problem_fact("r1", Box::new(|_| {}));

        assert_eq!(director.changes().len(), 3);
    }

    #[test]
    fn test_director_nested_objects() {
        let solution = json!({
            "schedule": {
                "lessons": [
                    {
                        "id": "l1",
                        "teacher": {"id": "t1", "name": "Alice"}
                    }
                ]
            }
        });

        let director = DefaultProblemChangeDirector::new(&solution, "id");

        // Both lesson and nested teacher should be indexed
        assert!(director.look_up_working_object("l1").is_some());
        assert!(director.look_up_working_object("t1").is_some());
    }

    #[test]
    fn test_director_update_and_remove_from_index() {
        let solution = json!({"entities": []});
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        director.update_index("manual1".to_string(), json!({"id": "manual1", "value": 1}));
        assert!(director.look_up_working_object("manual1").is_some());

        director.remove_from_index("manual1");
        assert!(director.look_up_working_object("manual1").is_none());
    }

    // Test error serialization
    #[test]
    fn test_error_serialization() {
        let err = ProblemChangeError::ObjectNotFound {
            id: "test".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: ProblemChangeError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, parsed);
    }

    // Test edge cases
    #[test]
    fn test_director_remove_nonexistent_entity() {
        let solution = json!({"entities": []});
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        // Should not panic, just do nothing
        director.remove_entity("nonexistent", Box::new(|_| {}));
        assert!(!director.has_changes());
    }

    #[test]
    fn test_director_change_variable_nonexistent() {
        let solution = json!({"entities": []});
        let mut director = DefaultProblemChangeDirector::new(&solution, "id");

        // Should not panic, just do nothing
        director.change_variable("nonexistent", "room", Box::new(|_| {}));
        assert!(!director.has_changes());
    }

    #[test]
    fn test_dto_with_complex_entity() {
        let complex_entity = json!({
            "id": "lesson1",
            "subject": "Advanced Mathematics",
            "teacher": {"id": "t1", "name": "Dr. Smith"},
            "students": [
                {"id": "s1", "name": "Alice"},
                {"id": "s2", "name": "Bob"}
            ],
            "schedule": {
                "day": "Monday",
                "period": 3
            }
        });

        let dto =
            ProblemChangeDto::add_entity("Lesson", "lesson1", complex_entity.clone(), "lessons");

        let json = serde_json::to_string(&dto).unwrap();
        let parsed: ProblemChangeDto = serde_json::from_str(&json).unwrap();

        match parsed {
            ProblemChangeDto::AddEntity { entity, .. } => {
                assert_eq!(entity, complex_entity);
            }
            _ => panic!("Expected AddEntity"),
        }
    }

    #[test]
    fn test_director_empty_solution() {
        let solution = json!({});
        let director = DefaultProblemChangeDirector::new(&solution, "id");

        assert!(director.look_up_working_object("any").is_none());
    }

    #[test]
    fn test_director_array_at_root() {
        let solution = json!([
            {"id": "e1", "value": 1},
            {"id": "e2", "value": 2}
        ]);

        let director = DefaultProblemChangeDirector::new(&solution, "id");

        assert!(director.look_up_working_object("e1").is_some());
        assert!(director.look_up_working_object("e2").is_some());
    }

    #[test]
    fn test_director_different_id_field() {
        let solution = json!({
            "entities": [
                {"uuid": "abc-123", "name": "Entity1"},
                {"uuid": "def-456", "name": "Entity2"}
            ]
        });

        let director = DefaultProblemChangeDirector::new(&solution, "uuid");

        assert!(director.look_up_working_object("abc-123").is_some());
        assert!(director.look_up_working_object("def-456").is_some());
        assert!(director.look_up_working_object("Entity1").is_none());
    }
}
