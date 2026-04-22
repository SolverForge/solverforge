use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug)]
pub(crate) struct ScalarVariableMetadata {
    pub field_name: String,
    pub allows_unassigned: bool,
    pub value_range_provider: Option<String>,
    pub countable_range: Option<(i64, i64)>,
    pub provider_is_entity_field: bool,
    pub nearby_value_distance_meter: Option<String>,
    pub nearby_entity_distance_meter: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ScalarEntityMetadata {
    pub variables: Vec<ScalarVariableMetadata>,
}

static SCALAR_ENTITY_REGISTRY: OnceLock<Mutex<HashMap<String, ScalarEntityMetadata>>> =
    OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, ScalarEntityMetadata>> {
    SCALAR_ENTITY_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn record_scalar_entity_metadata(
    entity_name: &str,
    variables: Vec<ScalarVariableMetadata>,
) {
    let mut registry = registry()
        .lock()
        .expect("solverforge scalar metadata registry should be available");
    registry.insert(entity_name.to_string(), ScalarEntityMetadata { variables });
}

pub(crate) fn lookup_scalar_entity_metadata(entity_name: &str) -> Option<ScalarEntityMetadata> {
    let registry = registry()
        .lock()
        .expect("solverforge scalar metadata registry should be available");
    registry.get(entity_name).cloned()
}
