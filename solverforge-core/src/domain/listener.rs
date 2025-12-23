//! VariableListener API for custom shadow variable computations.
//!
//! This module provides the ability to define custom logic for shadow variable updates
//! when planning variables change.
//!
//! # Architecture
//!
//! Variable listeners are invoked by the solver when genuine planning variables change.
//! They are responsible for updating shadow variables based on the new state.
//!
//! There are two types of listeners:
//!
//! - [`VariableListener`] - For basic planning variables (single-valued)
//! - [`ListVariableListener`] - For list planning variables (multi-valued)
//!
//! # Example
//!
//! ```ignore
//! use solverforge_core::domain::listener::{VariableListener, VariableListenerContext};
//!
//! // Define a listener that updates arrival time based on previous task
//! struct ArrivalTimeListener;
//!
//! impl VariableListener for ArrivalTimeListener {
//!     fn before_variable_changed(&self, ctx: &mut VariableListenerContext, entity_id: &str) {
//!         // Store previous state if needed
//!     }
//!
//!     fn after_variable_changed(&self, ctx: &mut VariableListenerContext, entity_id: &str) {
//!         // Recalculate arrival time based on previous task's departure
//!         let entity = ctx.get_entity(entity_id).unwrap();
//!         if let Some(prev_id) = entity.get("previousTask").and_then(|v| v.as_str()) {
//!             let prev = ctx.get_entity(prev_id).unwrap();
//!             let departure = prev.get("departureTime").and_then(|v| v.as_i64()).unwrap_or(0);
//!             ctx.set_shadow_variable(entity_id, "arrivalTime", serde_json::json!(departure + 30));
//!         }
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Context provided to variable listeners during callbacks.
///
/// Provides access to the working solution state and methods to update shadow variables.
pub trait VariableListenerContext: Send {
    /// Get an entity by its ID.
    fn get_entity(&self, entity_id: &str) -> Option<&serde_json::Value>;

    /// Get a problem fact by its ID.
    fn get_problem_fact(&self, fact_id: &str) -> Option<&serde_json::Value>;

    /// Set a shadow variable value on an entity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The ID of the entity to update
    /// * `variable_name` - The name of the shadow variable
    /// * `value` - The new value for the shadow variable
    fn set_shadow_variable(
        &mut self,
        entity_id: &str,
        variable_name: &str,
        value: serde_json::Value,
    );

    /// Get the current value of a shadow variable.
    fn get_shadow_variable(
        &self,
        entity_id: &str,
        variable_name: &str,
    ) -> Option<&serde_json::Value>;

    /// Trigger downstream shadow variable updates.
    ///
    /// Call this when a shadow variable change should cascade to other shadow variables.
    fn trigger_variable_listeners(&mut self, entity_id: &str, variable_name: &str);
}

/// Listener for basic planning variables (single-valued).
///
/// Implement this trait to define custom logic for updating shadow variables
/// when a basic planning variable changes.
///
/// # Important
///
/// - Only change shadow variable(s) for which this listener is configured
/// - Never change genuine variables or problem facts
/// - Keep implementations stateless when possible
pub trait VariableListener: Send + Sync + Debug {
    /// Called before the source variable is changed.
    ///
    /// Use this to capture the previous state if needed for the update calculation.
    fn before_variable_changed(&self, ctx: &mut dyn VariableListenerContext, entity_id: &str);

    /// Called after the source variable has changed.
    ///
    /// Use this to update shadow variable(s) based on the new state.
    fn after_variable_changed(&self, ctx: &mut dyn VariableListenerContext, entity_id: &str);

    /// Called before an entity is added to the solution.
    fn before_entity_added(&self, _ctx: &mut dyn VariableListenerContext, _entity_id: &str) {}

    /// Called after an entity is added to the solution.
    fn after_entity_added(&self, _ctx: &mut dyn VariableListenerContext, _entity_id: &str) {}

    /// Called before an entity is removed from the solution.
    fn before_entity_removed(&self, _ctx: &mut dyn VariableListenerContext, _entity_id: &str) {}

    /// Called after an entity is removed from the solution.
    fn after_entity_removed(&self, _ctx: &mut dyn VariableListenerContext, _entity_id: &str) {}

    /// Called when the working solution is reset.
    ///
    /// Stateful listeners should clear their state in this method.
    fn reset_working_solution(&mut self, _ctx: &mut dyn VariableListenerContext) {}

    /// Called when the listener is being disposed.
    ///
    /// Clean up any resources held by the listener.
    fn close(&mut self) {}

    /// Whether this listener requires unique entity events.
    ///
    /// When `true`, each before/after method is guaranteed to be called only once
    /// per entity instance per operation type. This has a performance cost.
    ///
    /// When `false` (default), the listener must handle potentially duplicate events.
    fn requires_unique_entity_events(&self) -> bool {
        false
    }
}

/// Listener for list planning variables (multi-valued).
///
/// Implement this trait to define custom logic for updating shadow variables
/// when a list planning variable changes.
///
/// # Important
///
/// - Only change shadow variable(s) for which this listener is configured
/// - Never change genuine variables or problem facts
/// - Keep implementations stateless when possible
pub trait ListVariableListener: Send + Sync + Debug {
    /// Called when an element is unassigned from the list variable.
    ///
    /// The listener should unset all shadow variables it manages for the element.
    fn after_list_variable_element_unassigned(
        &self,
        ctx: &mut dyn VariableListenerContext,
        element_id: &str,
    );

    /// Called before elements in the specified range will change.
    ///
    /// The range `[from_index, to_index)` contains all elements that are going to change,
    /// but may also contain elements that won't change.
    fn before_list_variable_changed(
        &self,
        ctx: &mut dyn VariableListenerContext,
        entity_id: &str,
        from_index: usize,
        to_index: usize,
    );

    /// Called after elements in the specified range have changed.
    ///
    /// The range `[from_index, to_index)` contains all elements that have changed,
    /// but may also contain elements that didn't change.
    fn after_list_variable_changed(
        &self,
        ctx: &mut dyn VariableListenerContext,
        entity_id: &str,
        from_index: usize,
        to_index: usize,
    );

    /// Called before an entity is added to the solution.
    fn before_entity_added(&self, _ctx: &mut dyn VariableListenerContext, _entity_id: &str) {}

    /// Called after an entity is added to the solution.
    fn after_entity_added(&self, _ctx: &mut dyn VariableListenerContext, _entity_id: &str) {}

    /// Called before an entity is removed from the solution.
    fn before_entity_removed(&self, _ctx: &mut dyn VariableListenerContext, _entity_id: &str) {}

    /// Called after an entity is removed from the solution.
    fn after_entity_removed(&self, _ctx: &mut dyn VariableListenerContext, _entity_id: &str) {}

    /// Called when the working solution is reset.
    fn reset_working_solution(&mut self, _ctx: &mut dyn VariableListenerContext) {}

    /// Called when the listener is being disposed.
    fn close(&mut self) {}
}

/// Registration information for a variable listener.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableListenerRegistration {
    /// The name of the shadow variable this listener updates.
    pub shadow_variable_name: String,
    /// The entity class containing the shadow variable.
    pub shadow_entity_class: String,
    /// The source variable(s) that trigger this listener.
    pub source_variables: Vec<SourceVariableRef>,
    /// The listener class name (for serialization to the service).
    pub listener_class: String,
    /// Whether this is a list variable listener.
    pub is_list_listener: bool,
}

/// Reference to a source variable that triggers a listener.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceVariableRef {
    /// The entity class containing the source variable.
    pub entity_class: String,
    /// The name of the source variable.
    pub variable_name: String,
}

impl SourceVariableRef {
    /// Create a new source variable reference.
    pub fn new(entity_class: impl Into<String>, variable_name: impl Into<String>) -> Self {
        Self {
            entity_class: entity_class.into(),
            variable_name: variable_name.into(),
        }
    }
}

impl VariableListenerRegistration {
    /// Create a new variable listener registration.
    pub fn new(
        shadow_variable_name: impl Into<String>,
        shadow_entity_class: impl Into<String>,
        listener_class: impl Into<String>,
    ) -> Self {
        Self {
            shadow_variable_name: shadow_variable_name.into(),
            shadow_entity_class: shadow_entity_class.into(),
            source_variables: Vec::new(),
            listener_class: listener_class.into(),
            is_list_listener: false,
        }
    }

    /// Create a registration for a list variable listener.
    pub fn list(
        shadow_variable_name: impl Into<String>,
        shadow_entity_class: impl Into<String>,
        listener_class: impl Into<String>,
    ) -> Self {
        Self {
            shadow_variable_name: shadow_variable_name.into(),
            shadow_entity_class: shadow_entity_class.into(),
            source_variables: Vec::new(),
            listener_class: listener_class.into(),
            is_list_listener: true,
        }
    }

    /// Add a source variable that triggers this listener.
    pub fn with_source(
        mut self,
        entity_class: impl Into<String>,
        variable_name: impl Into<String>,
    ) -> Self {
        self.source_variables
            .push(SourceVariableRef::new(entity_class, variable_name));
        self
    }

    /// Add multiple source variables.
    pub fn with_sources(mut self, sources: Vec<SourceVariableRef>) -> Self {
        self.source_variables.extend(sources);
        self
    }
}

/// Default implementation of VariableListenerContext for local use.
#[derive(Debug)]
pub struct DefaultVariableListenerContext {
    /// Index of entity IDs to their JSON values.
    entities: std::collections::HashMap<String, serde_json::Value>,
    /// Index of problem fact IDs to their JSON values.
    facts: std::collections::HashMap<String, serde_json::Value>,
    /// Pending shadow variable updates.
    pending_updates: Vec<ShadowVariableUpdate>,
    /// Pending listener triggers.
    pending_triggers: Vec<(String, String)>,
}

/// A pending shadow variable update.
#[derive(Debug, Clone)]
pub struct ShadowVariableUpdate {
    /// The ID of the entity to update.
    pub entity_id: String,
    /// The name of the shadow variable to update.
    pub variable_name: String,
    /// The new value for the shadow variable.
    pub value: serde_json::Value,
}

impl DefaultVariableListenerContext {
    /// Create a new context from a solution JSON.
    pub fn new(solution: &serde_json::Value, id_field: &str) -> Self {
        let mut entities = std::collections::HashMap::new();

        // Index all objects with IDs
        Self::index_objects(solution, id_field, &mut entities);

        Self {
            entities,
            facts: std::collections::HashMap::new(),
            pending_updates: Vec::new(),
            pending_triggers: Vec::new(),
        }
    }

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

    /// Add an entity to the context.
    pub fn add_entity(&mut self, id: String, entity: serde_json::Value) {
        self.entities.insert(id, entity);
    }

    /// Add a problem fact to the context.
    pub fn add_fact(&mut self, id: String, fact: serde_json::Value) {
        self.facts.insert(id, fact);
    }

    /// Get pending shadow variable updates.
    pub fn pending_updates(&self) -> &[ShadowVariableUpdate] {
        &self.pending_updates
    }

    /// Get pending listener triggers.
    pub fn pending_triggers(&self) -> &[(String, String)] {
        &self.pending_triggers
    }

    /// Clear pending updates and triggers.
    pub fn clear_pending(&mut self) {
        self.pending_updates.clear();
        self.pending_triggers.clear();
    }

    /// Apply pending updates to the entities.
    pub fn apply_pending_updates(&mut self) {
        for update in std::mem::take(&mut self.pending_updates) {
            if let Some(entity) = self.entities.get_mut(&update.entity_id) {
                if let Some(obj) = entity.as_object_mut() {
                    obj.insert(update.variable_name, update.value);
                }
            }
        }
    }
}

impl VariableListenerContext for DefaultVariableListenerContext {
    fn get_entity(&self, entity_id: &str) -> Option<&serde_json::Value> {
        self.entities.get(entity_id)
    }

    fn get_problem_fact(&self, fact_id: &str) -> Option<&serde_json::Value> {
        self.facts.get(fact_id)
    }

    fn set_shadow_variable(
        &mut self,
        entity_id: &str,
        variable_name: &str,
        value: serde_json::Value,
    ) {
        self.pending_updates.push(ShadowVariableUpdate {
            entity_id: entity_id.to_string(),
            variable_name: variable_name.to_string(),
            value,
        });
    }

    fn get_shadow_variable(
        &self,
        entity_id: &str,
        variable_name: &str,
    ) -> Option<&serde_json::Value> {
        self.entities
            .get(entity_id)
            .and_then(|e| e.get(variable_name))
    }

    fn trigger_variable_listeners(&mut self, entity_id: &str, variable_name: &str) {
        self.pending_triggers
            .push((entity_id.to_string(), variable_name.to_string()));
    }
}

/// DTO for transmitting listener callbacks to/from the solver service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ListenerCallbackDto {
    /// Before a basic variable changes.
    #[serde(rename = "beforeVariableChanged")]
    BeforeVariableChanged {
        listener_class: String,
        entity_id: String,
        variable_name: String,
    },

    /// After a basic variable changes.
    #[serde(rename = "afterVariableChanged")]
    AfterVariableChanged {
        listener_class: String,
        entity_id: String,
        variable_name: String,
    },

    /// Before a list variable changes.
    #[serde(rename = "beforeListVariableChanged")]
    BeforeListVariableChanged {
        listener_class: String,
        entity_id: String,
        variable_name: String,
        from_index: usize,
        to_index: usize,
    },

    /// After a list variable changes.
    #[serde(rename = "afterListVariableChanged")]
    AfterListVariableChanged {
        listener_class: String,
        entity_id: String,
        variable_name: String,
        from_index: usize,
        to_index: usize,
    },

    /// An element was unassigned from a list variable.
    #[serde(rename = "afterListVariableElementUnassigned")]
    AfterListVariableElementUnassigned {
        listener_class: String,
        element_id: String,
    },

    /// Before an entity is added.
    #[serde(rename = "beforeEntityAdded")]
    BeforeEntityAdded {
        listener_class: String,
        entity_id: String,
    },

    /// After an entity is added.
    #[serde(rename = "afterEntityAdded")]
    AfterEntityAdded {
        listener_class: String,
        entity_id: String,
    },

    /// Before an entity is removed.
    #[serde(rename = "beforeEntityRemoved")]
    BeforeEntityRemoved {
        listener_class: String,
        entity_id: String,
    },

    /// After an entity is removed.
    #[serde(rename = "afterEntityRemoved")]
    AfterEntityRemoved {
        listener_class: String,
        entity_id: String,
    },

    /// Reset the working solution.
    #[serde(rename = "resetWorkingSolution")]
    ResetWorkingSolution { listener_class: String },
}

impl ListenerCallbackDto {
    pub fn before_variable_changed(
        listener_class: impl Into<String>,
        entity_id: impl Into<String>,
        variable_name: impl Into<String>,
    ) -> Self {
        ListenerCallbackDto::BeforeVariableChanged {
            listener_class: listener_class.into(),
            entity_id: entity_id.into(),
            variable_name: variable_name.into(),
        }
    }

    pub fn after_variable_changed(
        listener_class: impl Into<String>,
        entity_id: impl Into<String>,
        variable_name: impl Into<String>,
    ) -> Self {
        ListenerCallbackDto::AfterVariableChanged {
            listener_class: listener_class.into(),
            entity_id: entity_id.into(),
            variable_name: variable_name.into(),
        }
    }

    pub fn before_list_variable_changed(
        listener_class: impl Into<String>,
        entity_id: impl Into<String>,
        variable_name: impl Into<String>,
        from_index: usize,
        to_index: usize,
    ) -> Self {
        ListenerCallbackDto::BeforeListVariableChanged {
            listener_class: listener_class.into(),
            entity_id: entity_id.into(),
            variable_name: variable_name.into(),
            from_index,
            to_index,
        }
    }

    pub fn after_list_variable_changed(
        listener_class: impl Into<String>,
        entity_id: impl Into<String>,
        variable_name: impl Into<String>,
        from_index: usize,
        to_index: usize,
    ) -> Self {
        ListenerCallbackDto::AfterListVariableChanged {
            listener_class: listener_class.into(),
            entity_id: entity_id.into(),
            variable_name: variable_name.into(),
            from_index,
            to_index,
        }
    }

    pub fn after_list_variable_element_unassigned(
        listener_class: impl Into<String>,
        element_id: impl Into<String>,
    ) -> Self {
        ListenerCallbackDto::AfterListVariableElementUnassigned {
            listener_class: listener_class.into(),
            element_id: element_id.into(),
        }
    }

    pub fn before_entity_added(
        listener_class: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        ListenerCallbackDto::BeforeEntityAdded {
            listener_class: listener_class.into(),
            entity_id: entity_id.into(),
        }
    }

    pub fn after_entity_added(
        listener_class: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        ListenerCallbackDto::AfterEntityAdded {
            listener_class: listener_class.into(),
            entity_id: entity_id.into(),
        }
    }

    pub fn before_entity_removed(
        listener_class: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        ListenerCallbackDto::BeforeEntityRemoved {
            listener_class: listener_class.into(),
            entity_id: entity_id.into(),
        }
    }

    pub fn after_entity_removed(
        listener_class: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        ListenerCallbackDto::AfterEntityRemoved {
            listener_class: listener_class.into(),
            entity_id: entity_id.into(),
        }
    }

    pub fn reset_working_solution(listener_class: impl Into<String>) -> Self {
        ListenerCallbackDto::ResetWorkingSolution {
            listener_class: listener_class.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Test SourceVariableRef
    #[test]
    fn test_source_variable_ref_new() {
        let ref1 = SourceVariableRef::new("Lesson", "room");
        assert_eq!(ref1.entity_class, "Lesson");
        assert_eq!(ref1.variable_name, "room");
    }

    // Test VariableListenerRegistration
    #[test]
    fn test_registration_new() {
        let reg = VariableListenerRegistration::new("arrivalTime", "Task", "ArrivalTimeListener");
        assert_eq!(reg.shadow_variable_name, "arrivalTime");
        assert_eq!(reg.shadow_entity_class, "Task");
        assert_eq!(reg.listener_class, "ArrivalTimeListener");
        assert!(!reg.is_list_listener);
        assert!(reg.source_variables.is_empty());
    }

    #[test]
    fn test_registration_list() {
        let reg = VariableListenerRegistration::list("index", "Task", "IndexListener");
        assert!(reg.is_list_listener);
    }

    #[test]
    fn test_registration_with_source() {
        let reg = VariableListenerRegistration::new("arrivalTime", "Task", "ArrivalTimeListener")
            .with_source("Vehicle", "taskList");
        assert_eq!(reg.source_variables.len(), 1);
        assert_eq!(reg.source_variables[0].entity_class, "Vehicle");
        assert_eq!(reg.source_variables[0].variable_name, "taskList");
    }

    #[test]
    fn test_registration_with_sources() {
        let reg = VariableListenerRegistration::new("arrivalTime", "Task", "ArrivalTimeListener")
            .with_sources(vec![
                SourceVariableRef::new("Vehicle", "taskList"),
                SourceVariableRef::new("Task", "previousTask"),
            ]);
        assert_eq!(reg.source_variables.len(), 2);
    }

    #[test]
    fn test_registration_json_serialization() {
        let reg = VariableListenerRegistration::new("arrivalTime", "Task", "ArrivalTimeListener")
            .with_source("Vehicle", "taskList");

        let json = serde_json::to_string(&reg).unwrap();
        let parsed: VariableListenerRegistration = serde_json::from_str(&json).unwrap();
        assert_eq!(reg, parsed);
    }

    // Test DefaultVariableListenerContext
    #[test]
    fn test_context_new() {
        let solution = json!({
            "tasks": [
                {"id": "t1", "name": "Task1"},
                {"id": "t2", "name": "Task2"}
            ]
        });

        let ctx = DefaultVariableListenerContext::new(&solution, "id");
        assert!(ctx.get_entity("t1").is_some());
        assert!(ctx.get_entity("t2").is_some());
        assert!(ctx.get_entity("t3").is_none());
    }

    #[test]
    fn test_context_get_entity() {
        let solution = json!({
            "tasks": [{"id": "t1", "value": 42}]
        });

        let ctx = DefaultVariableListenerContext::new(&solution, "id");
        let entity = ctx.get_entity("t1").unwrap();
        assert_eq!(entity["value"], 42);
    }

    #[test]
    fn test_context_add_entity() {
        let mut ctx = DefaultVariableListenerContext::new(&json!({}), "id");
        ctx.add_entity("e1".to_string(), json!({"id": "e1", "value": 10}));
        assert!(ctx.get_entity("e1").is_some());
    }

    #[test]
    fn test_context_add_fact() {
        let mut ctx = DefaultVariableListenerContext::new(&json!({}), "id");
        ctx.add_fact("f1".to_string(), json!({"id": "f1", "name": "Fact1"}));
        assert!(ctx.get_problem_fact("f1").is_some());
    }

    #[test]
    fn test_context_set_shadow_variable() {
        let solution = json!({
            "tasks": [{"id": "t1", "arrivalTime": null}]
        });

        let mut ctx = DefaultVariableListenerContext::new(&solution, "id");
        ctx.set_shadow_variable("t1", "arrivalTime", json!(100));

        assert_eq!(ctx.pending_updates().len(), 1);
        let update = &ctx.pending_updates()[0];
        assert_eq!(update.entity_id, "t1");
        assert_eq!(update.variable_name, "arrivalTime");
        assert_eq!(update.value, json!(100));
    }

    #[test]
    fn test_context_apply_pending_updates() {
        let solution = json!({
            "tasks": [{"id": "t1", "arrivalTime": null}]
        });

        let mut ctx = DefaultVariableListenerContext::new(&solution, "id");
        ctx.set_shadow_variable("t1", "arrivalTime", json!(100));
        ctx.apply_pending_updates();

        let entity = ctx.get_entity("t1").unwrap();
        assert_eq!(entity["arrivalTime"], json!(100));
    }

    #[test]
    fn test_context_get_shadow_variable() {
        let solution = json!({
            "tasks": [{"id": "t1", "arrivalTime": 50}]
        });

        let ctx = DefaultVariableListenerContext::new(&solution, "id");
        let value = ctx.get_shadow_variable("t1", "arrivalTime");
        assert_eq!(value, Some(&json!(50)));
    }

    #[test]
    fn test_context_trigger_variable_listeners() {
        let mut ctx = DefaultVariableListenerContext::new(&json!({}), "id");
        ctx.trigger_variable_listeners("t1", "arrivalTime");

        assert_eq!(ctx.pending_triggers().len(), 1);
        assert_eq!(
            ctx.pending_triggers()[0],
            ("t1".to_string(), "arrivalTime".to_string())
        );
    }

    #[test]
    fn test_context_clear_pending() {
        let mut ctx = DefaultVariableListenerContext::new(&json!({}), "id");
        ctx.set_shadow_variable("t1", "arrivalTime", json!(100));
        ctx.trigger_variable_listeners("t1", "arrivalTime");

        assert!(!ctx.pending_updates().is_empty());
        assert!(!ctx.pending_triggers().is_empty());

        ctx.clear_pending();

        assert!(ctx.pending_updates().is_empty());
        assert!(ctx.pending_triggers().is_empty());
    }

    // Test ListenerCallbackDto
    #[test]
    fn test_callback_before_variable_changed() {
        let dto = ListenerCallbackDto::before_variable_changed("MyListener", "e1", "room");
        match dto {
            ListenerCallbackDto::BeforeVariableChanged {
                listener_class,
                entity_id,
                variable_name,
            } => {
                assert_eq!(listener_class, "MyListener");
                assert_eq!(entity_id, "e1");
                assert_eq!(variable_name, "room");
            }
            _ => panic!("Expected BeforeVariableChanged"),
        }
    }

    #[test]
    fn test_callback_after_variable_changed() {
        let dto = ListenerCallbackDto::after_variable_changed("MyListener", "e1", "room");
        match dto {
            ListenerCallbackDto::AfterVariableChanged {
                listener_class,
                entity_id,
                variable_name,
            } => {
                assert_eq!(listener_class, "MyListener");
                assert_eq!(entity_id, "e1");
                assert_eq!(variable_name, "room");
            }
            _ => panic!("Expected AfterVariableChanged"),
        }
    }

    #[test]
    fn test_callback_before_list_variable_changed() {
        let dto =
            ListenerCallbackDto::before_list_variable_changed("MyListener", "e1", "tasks", 0, 5);
        match dto {
            ListenerCallbackDto::BeforeListVariableChanged {
                listener_class,
                entity_id,
                variable_name,
                from_index,
                to_index,
            } => {
                assert_eq!(listener_class, "MyListener");
                assert_eq!(entity_id, "e1");
                assert_eq!(variable_name, "tasks");
                assert_eq!(from_index, 0);
                assert_eq!(to_index, 5);
            }
            _ => panic!("Expected BeforeListVariableChanged"),
        }
    }

    #[test]
    fn test_callback_after_list_variable_changed() {
        let dto =
            ListenerCallbackDto::after_list_variable_changed("MyListener", "e1", "tasks", 2, 7);
        match dto {
            ListenerCallbackDto::AfterListVariableChanged {
                from_index,
                to_index,
                ..
            } => {
                assert_eq!(from_index, 2);
                assert_eq!(to_index, 7);
            }
            _ => panic!("Expected AfterListVariableChanged"),
        }
    }

    #[test]
    fn test_callback_after_list_element_unassigned() {
        let dto =
            ListenerCallbackDto::after_list_variable_element_unassigned("MyListener", "elem1");
        match dto {
            ListenerCallbackDto::AfterListVariableElementUnassigned {
                listener_class,
                element_id,
            } => {
                assert_eq!(listener_class, "MyListener");
                assert_eq!(element_id, "elem1");
            }
            _ => panic!("Expected AfterListVariableElementUnassigned"),
        }
    }

    #[test]
    fn test_callback_entity_lifecycle() {
        let before_add = ListenerCallbackDto::before_entity_added("Listener", "e1");
        let after_add = ListenerCallbackDto::after_entity_added("Listener", "e1");
        let before_remove = ListenerCallbackDto::before_entity_removed("Listener", "e1");
        let after_remove = ListenerCallbackDto::after_entity_removed("Listener", "e1");

        matches!(before_add, ListenerCallbackDto::BeforeEntityAdded { .. });
        matches!(after_add, ListenerCallbackDto::AfterEntityAdded { .. });
        matches!(
            before_remove,
            ListenerCallbackDto::BeforeEntityRemoved { .. }
        );
        matches!(after_remove, ListenerCallbackDto::AfterEntityRemoved { .. });
    }

    #[test]
    fn test_callback_reset_working_solution() {
        let dto = ListenerCallbackDto::reset_working_solution("MyListener");
        match dto {
            ListenerCallbackDto::ResetWorkingSolution { listener_class } => {
                assert_eq!(listener_class, "MyListener");
            }
            _ => panic!("Expected ResetWorkingSolution"),
        }
    }

    #[test]
    fn test_callback_json_serialization() {
        let callbacks = vec![
            ListenerCallbackDto::before_variable_changed("L", "e1", "var"),
            ListenerCallbackDto::after_variable_changed("L", "e1", "var"),
            ListenerCallbackDto::before_list_variable_changed("L", "e1", "list", 0, 5),
            ListenerCallbackDto::after_list_variable_changed("L", "e1", "list", 0, 5),
            ListenerCallbackDto::after_list_variable_element_unassigned("L", "elem"),
            ListenerCallbackDto::before_entity_added("L", "e1"),
            ListenerCallbackDto::after_entity_added("L", "e1"),
            ListenerCallbackDto::before_entity_removed("L", "e1"),
            ListenerCallbackDto::after_entity_removed("L", "e1"),
            ListenerCallbackDto::reset_working_solution("L"),
        ];

        for cb in callbacks {
            let json = serde_json::to_string(&cb).unwrap();
            let parsed: ListenerCallbackDto = serde_json::from_str(&json).unwrap();
            assert_eq!(cb, parsed);
        }
    }

    #[test]
    fn test_callback_json_format() {
        let dto = ListenerCallbackDto::after_variable_changed("MyListener", "e1", "room");
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains(r#""type":"afterVariableChanged""#));
        assert!(json.contains(r#""listener_class":"MyListener""#));
    }

    // Test nested solution indexing
    #[test]
    fn test_context_nested_solution() {
        let solution = json!({
            "vehicles": [
                {
                    "id": "v1",
                    "tasks": [
                        {"id": "t1", "name": "Task1"},
                        {"id": "t2", "name": "Task2"}
                    ]
                }
            ]
        });

        let ctx = DefaultVariableListenerContext::new(&solution, "id");
        assert!(ctx.get_entity("v1").is_some());
        assert!(ctx.get_entity("t1").is_some());
        assert!(ctx.get_entity("t2").is_some());
    }

    // Test multiple shadow variable updates
    #[test]
    fn test_context_multiple_updates() {
        let solution = json!({
            "tasks": [
                {"id": "t1", "arrivalTime": null, "departureTime": null},
                {"id": "t2", "arrivalTime": null, "departureTime": null}
            ]
        });

        let mut ctx = DefaultVariableListenerContext::new(&solution, "id");
        ctx.set_shadow_variable("t1", "arrivalTime", json!(100));
        ctx.set_shadow_variable("t1", "departureTime", json!(150));
        ctx.set_shadow_variable("t2", "arrivalTime", json!(160));

        assert_eq!(ctx.pending_updates().len(), 3);

        ctx.apply_pending_updates();

        let t1 = ctx.get_entity("t1").unwrap();
        assert_eq!(t1["arrivalTime"], json!(100));
        assert_eq!(t1["departureTime"], json!(150));

        let t2 = ctx.get_entity("t2").unwrap();
        assert_eq!(t2["arrivalTime"], json!(160));
    }
}
