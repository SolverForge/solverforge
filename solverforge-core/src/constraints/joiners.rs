use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmFunction {
    name: String,
    relation_function: Option<String>,
    hash_function: Option<String>,
    comparator_function: Option<String>,
}

impl Serialize for WasmFunction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.name)
    }
}

impl<'de> Deserialize<'de> for WasmFunction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Ok(WasmFunction::new(name))
    }
}

impl WasmFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            relation_function: None,
            hash_function: None,
            comparator_function: None,
        }
    }

    pub fn with_relation(mut self, relation: impl Into<String>) -> Self {
        self.relation_function = Some(relation.into());
        self
    }

    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.hash_function = Some(hash.into());
        self
    }

    pub fn with_comparator(mut self, comparator: impl Into<String>) -> Self {
        self.comparator_function = Some(comparator.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn relation_function(&self) -> Option<&str> {
        self.relation_function.as_deref()
    }

    pub fn hash_function(&self) -> Option<&str> {
        self.hash_function.as_deref()
    }

    pub fn comparator_function(&self) -> Option<&str> {
        self.comparator_function.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "relation")]
pub enum Joiner {
    #[serde(rename = "equal")]
    Equal {
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<WasmFunction>,
        #[serde(rename = "leftMap", skip_serializing_if = "Option::is_none")]
        left_map: Option<WasmFunction>,
        #[serde(rename = "rightMap", skip_serializing_if = "Option::is_none")]
        right_map: Option<WasmFunction>,
        #[serde(rename = "equal", skip_serializing_if = "Option::is_none")]
        relation_predicate: Option<WasmFunction>,
        #[serde(skip_serializing_if = "Option::is_none")]
        hasher: Option<WasmFunction>,
    },
    #[serde(rename = "lessThan")]
    LessThan {
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<WasmFunction>,
        #[serde(rename = "leftMap", skip_serializing_if = "Option::is_none")]
        left_map: Option<WasmFunction>,
        #[serde(rename = "rightMap", skip_serializing_if = "Option::is_none")]
        right_map: Option<WasmFunction>,
        comparator: WasmFunction,
    },
    #[serde(rename = "lessThanOrEqual")]
    LessThanOrEqual {
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<WasmFunction>,
        #[serde(rename = "leftMap", skip_serializing_if = "Option::is_none")]
        left_map: Option<WasmFunction>,
        #[serde(rename = "rightMap", skip_serializing_if = "Option::is_none")]
        right_map: Option<WasmFunction>,
        comparator: WasmFunction,
    },
    #[serde(rename = "greaterThan")]
    GreaterThan {
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<WasmFunction>,
        #[serde(rename = "leftMap", skip_serializing_if = "Option::is_none")]
        left_map: Option<WasmFunction>,
        #[serde(rename = "rightMap", skip_serializing_if = "Option::is_none")]
        right_map: Option<WasmFunction>,
        comparator: WasmFunction,
    },
    #[serde(rename = "greaterThanOrEqual")]
    GreaterThanOrEqual {
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<WasmFunction>,
        #[serde(rename = "leftMap", skip_serializing_if = "Option::is_none")]
        left_map: Option<WasmFunction>,
        #[serde(rename = "rightMap", skip_serializing_if = "Option::is_none")]
        right_map: Option<WasmFunction>,
        comparator: WasmFunction,
    },
    #[serde(rename = "overlapping")]
    Overlapping {
        #[serde(rename = "startMap", skip_serializing_if = "Option::is_none")]
        start_map: Option<WasmFunction>,
        #[serde(rename = "endMap", skip_serializing_if = "Option::is_none")]
        end_map: Option<WasmFunction>,
        #[serde(rename = "leftStartMap", skip_serializing_if = "Option::is_none")]
        left_start_map: Option<WasmFunction>,
        #[serde(rename = "leftEndMap", skip_serializing_if = "Option::is_none")]
        left_end_map: Option<WasmFunction>,
        #[serde(rename = "rightStartMap", skip_serializing_if = "Option::is_none")]
        right_start_map: Option<WasmFunction>,
        #[serde(rename = "rightEndMap", skip_serializing_if = "Option::is_none")]
        right_end_map: Option<WasmFunction>,
        #[serde(skip_serializing_if = "Option::is_none")]
        comparator: Option<WasmFunction>,
    },
    #[serde(rename = "filtering")]
    Filtering { filter: WasmFunction },
}

impl Joiner {
    pub fn equal(map: WasmFunction) -> Self {
        Joiner::Equal {
            map: Some(map),
            left_map: None,
            right_map: None,
            relation_predicate: None,
            hasher: None,
        }
    }

    pub fn equal_with_mappings(left_map: WasmFunction, right_map: WasmFunction) -> Self {
        Joiner::Equal {
            map: None,
            left_map: Some(left_map),
            right_map: Some(right_map),
            relation_predicate: None,
            hasher: None,
        }
    }

    pub fn equal_with_custom_equals(
        map: WasmFunction,
        relation_predicate: WasmFunction,
        hasher: WasmFunction,
    ) -> Self {
        Joiner::Equal {
            map: Some(map),
            left_map: None,
            right_map: None,
            relation_predicate: Some(relation_predicate),
            hasher: Some(hasher),
        }
    }

    pub fn less_than(map: WasmFunction, comparator: WasmFunction) -> Self {
        Joiner::LessThan {
            map: Some(map),
            left_map: None,
            right_map: None,
            comparator,
        }
    }

    pub fn less_than_with_mappings(
        left_map: WasmFunction,
        right_map: WasmFunction,
        comparator: WasmFunction,
    ) -> Self {
        Joiner::LessThan {
            map: None,
            left_map: Some(left_map),
            right_map: Some(right_map),
            comparator,
        }
    }

    pub fn less_than_or_equal(map: WasmFunction, comparator: WasmFunction) -> Self {
        Joiner::LessThanOrEqual {
            map: Some(map),
            left_map: None,
            right_map: None,
            comparator,
        }
    }

    pub fn less_than_or_equal_with_mappings(
        left_map: WasmFunction,
        right_map: WasmFunction,
        comparator: WasmFunction,
    ) -> Self {
        Joiner::LessThanOrEqual {
            map: None,
            left_map: Some(left_map),
            right_map: Some(right_map),
            comparator,
        }
    }

    pub fn greater_than(map: WasmFunction, comparator: WasmFunction) -> Self {
        Joiner::GreaterThan {
            map: Some(map),
            left_map: None,
            right_map: None,
            comparator,
        }
    }

    pub fn greater_than_with_mappings(
        left_map: WasmFunction,
        right_map: WasmFunction,
        comparator: WasmFunction,
    ) -> Self {
        Joiner::GreaterThan {
            map: None,
            left_map: Some(left_map),
            right_map: Some(right_map),
            comparator,
        }
    }

    pub fn greater_than_or_equal(map: WasmFunction, comparator: WasmFunction) -> Self {
        Joiner::GreaterThanOrEqual {
            map: Some(map),
            left_map: None,
            right_map: None,
            comparator,
        }
    }

    pub fn greater_than_or_equal_with_mappings(
        left_map: WasmFunction,
        right_map: WasmFunction,
        comparator: WasmFunction,
    ) -> Self {
        Joiner::GreaterThanOrEqual {
            map: None,
            left_map: Some(left_map),
            right_map: Some(right_map),
            comparator,
        }
    }

    pub fn overlapping(start_map: WasmFunction, end_map: WasmFunction) -> Self {
        Joiner::Overlapping {
            start_map: Some(start_map),
            end_map: Some(end_map),
            left_start_map: None,
            left_end_map: None,
            right_start_map: None,
            right_end_map: None,
            comparator: None,
        }
    }

    pub fn overlapping_with_mappings(
        left_start_map: WasmFunction,
        left_end_map: WasmFunction,
        right_start_map: WasmFunction,
        right_end_map: WasmFunction,
    ) -> Self {
        Joiner::Overlapping {
            start_map: None,
            end_map: None,
            left_start_map: Some(left_start_map),
            left_end_map: Some(left_end_map),
            right_start_map: Some(right_start_map),
            right_end_map: Some(right_end_map),
            comparator: None,
        }
    }

    pub fn overlapping_with_comparator(
        start_map: WasmFunction,
        end_map: WasmFunction,
        comparator: WasmFunction,
    ) -> Self {
        Joiner::Overlapping {
            start_map: Some(start_map),
            end_map: Some(end_map),
            left_start_map: None,
            left_end_map: None,
            right_start_map: None,
            right_end_map: None,
            comparator: Some(comparator),
        }
    }

    pub fn filtering(filter: WasmFunction) -> Self {
        Joiner::Filtering { filter }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_function_new() {
        let func = WasmFunction::new("get_timeslot");
        assert_eq!(func.name(), "get_timeslot");
        assert!(func.relation_function().is_none());
        assert!(func.hash_function().is_none());
        assert!(func.comparator_function().is_none());
    }

    #[test]
    fn test_wasm_function_with_relation() {
        let func = WasmFunction::new("get_value")
            .with_relation("equals_fn")
            .with_hash("hash_fn");
        assert_eq!(func.name(), "get_value");
        assert_eq!(func.relation_function(), Some("equals_fn"));
        assert_eq!(func.hash_function(), Some("hash_fn"));
    }

    #[test]
    fn test_wasm_function_with_comparator() {
        let func = WasmFunction::new("get_time").with_comparator("compare_times");
        assert_eq!(func.comparator_function(), Some("compare_times"));
    }

    #[test]
    fn test_equal_joiner() {
        let joiner = Joiner::equal(WasmFunction::new("get_timeslot"));
        match joiner {
            Joiner::Equal {
                map,
                left_map,
                right_map,
                ..
            } => {
                assert!(map.is_some());
                assert_eq!(map.unwrap().name(), "get_timeslot");
                assert!(left_map.is_none());
                assert!(right_map.is_none());
            }
            _ => panic!("Expected Equal joiner"),
        }
    }

    #[test]
    fn test_equal_with_mappings() {
        let joiner = Joiner::equal_with_mappings(
            WasmFunction::new("get_left_timeslot"),
            WasmFunction::new("get_right_timeslot"),
        );
        match joiner {
            Joiner::Equal {
                map,
                left_map,
                right_map,
                ..
            } => {
                assert!(map.is_none());
                assert_eq!(left_map.unwrap().name(), "get_left_timeslot");
                assert_eq!(right_map.unwrap().name(), "get_right_timeslot");
            }
            _ => panic!("Expected Equal joiner"),
        }
    }

    #[test]
    fn test_equal_with_custom_equals() {
        let joiner = Joiner::equal_with_custom_equals(
            WasmFunction::new("get_id"),
            WasmFunction::new("id_equals"),
            WasmFunction::new("id_hash"),
        );
        match joiner {
            Joiner::Equal {
                map,
                relation_predicate,
                hasher,
                ..
            } => {
                assert!(map.is_some());
                assert!(relation_predicate.is_some());
                assert!(hasher.is_some());
            }
            _ => panic!("Expected Equal joiner"),
        }
    }

    #[test]
    fn test_less_than_joiner() {
        let joiner = Joiner::less_than(
            WasmFunction::new("get_start_time"),
            WasmFunction::new("compare_time"),
        );
        match joiner {
            Joiner::LessThan {
                map, comparator, ..
            } => {
                assert!(map.is_some());
                assert_eq!(comparator.name(), "compare_time");
            }
            _ => panic!("Expected LessThan joiner"),
        }
    }

    #[test]
    fn test_less_than_with_mappings() {
        let joiner = Joiner::less_than_with_mappings(
            WasmFunction::new("get_left_time"),
            WasmFunction::new("get_right_time"),
            WasmFunction::new("compare_time"),
        );
        match joiner {
            Joiner::LessThan {
                map,
                left_map,
                right_map,
                ..
            } => {
                assert!(map.is_none());
                assert!(left_map.is_some());
                assert!(right_map.is_some());
            }
            _ => panic!("Expected LessThan joiner"),
        }
    }

    #[test]
    fn test_greater_than_joiner() {
        let joiner = Joiner::greater_than(
            WasmFunction::new("get_priority"),
            WasmFunction::new("compare_priority"),
        );
        match joiner {
            Joiner::GreaterThan { map, .. } => {
                assert!(map.is_some());
            }
            _ => panic!("Expected GreaterThan joiner"),
        }
    }

    #[test]
    fn test_overlapping_joiner() {
        let joiner =
            Joiner::overlapping(WasmFunction::new("get_start"), WasmFunction::new("get_end"));
        match joiner {
            Joiner::Overlapping {
                start_map,
                end_map,
                left_start_map,
                ..
            } => {
                assert!(start_map.is_some());
                assert!(end_map.is_some());
                assert!(left_start_map.is_none());
            }
            _ => panic!("Expected Overlapping joiner"),
        }
    }

    #[test]
    fn test_overlapping_with_mappings() {
        let joiner = Joiner::overlapping_with_mappings(
            WasmFunction::new("left_start"),
            WasmFunction::new("left_end"),
            WasmFunction::new("right_start"),
            WasmFunction::new("right_end"),
        );
        match joiner {
            Joiner::Overlapping {
                start_map,
                left_start_map,
                left_end_map,
                right_start_map,
                right_end_map,
                ..
            } => {
                assert!(start_map.is_none());
                assert!(left_start_map.is_some());
                assert!(left_end_map.is_some());
                assert!(right_start_map.is_some());
                assert!(right_end_map.is_some());
            }
            _ => panic!("Expected Overlapping joiner"),
        }
    }

    #[test]
    fn test_filtering_joiner() {
        let joiner = Joiner::filtering(WasmFunction::new("is_compatible"));
        match joiner {
            Joiner::Filtering { filter } => {
                assert_eq!(filter.name(), "is_compatible");
            }
            _ => panic!("Expected Filtering joiner"),
        }
    }

    #[test]
    fn test_equal_joiner_json_serialization() {
        let joiner = Joiner::equal(WasmFunction::new("get_timeslot"));
        let json = serde_json::to_string(&joiner).unwrap();
        assert!(json.contains("\"relation\":\"equal\""));
        assert!(json.contains("\"map\":\"get_timeslot\""));

        let parsed: Joiner = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, joiner);
    }

    #[test]
    fn test_less_than_joiner_json_serialization() {
        let joiner = Joiner::less_than(
            WasmFunction::new("get_time"),
            WasmFunction::new("compare_time"),
        );
        let json = serde_json::to_string(&joiner).unwrap();
        assert!(json.contains("\"relation\":\"lessThan\""));
        assert!(json.contains("\"comparator\":\"compare_time\""));

        let parsed: Joiner = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, joiner);
    }

    #[test]
    fn test_overlapping_joiner_json_serialization() {
        let joiner = Joiner::overlapping(WasmFunction::new("start"), WasmFunction::new("end"));
        let json = serde_json::to_string(&joiner).unwrap();
        assert!(json.contains("\"relation\":\"overlapping\""));
        assert!(json.contains("\"startMap\":\"start\""));
        assert!(json.contains("\"endMap\":\"end\""));

        let parsed: Joiner = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, joiner);
    }

    #[test]
    fn test_filtering_joiner_json_serialization() {
        let joiner = Joiner::filtering(WasmFunction::new("is_valid"));
        let json = serde_json::to_string(&joiner).unwrap();
        assert!(json.contains("\"relation\":\"filtering\""));
        assert!(json.contains("\"filter\":\"is_valid\""));

        let parsed: Joiner = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, joiner);
    }

    #[test]
    fn test_equal_with_left_right_json() {
        let joiner = Joiner::equal_with_mappings(
            WasmFunction::new("left_fn"),
            WasmFunction::new("right_fn"),
        );
        let json = serde_json::to_string(&joiner).unwrap();
        assert!(json.contains("\"leftMap\":\"left_fn\""));
        assert!(json.contains("\"rightMap\":\"right_fn\""));
        assert!(!json.contains("\"map\""));
    }

    #[test]
    fn test_joiner_clone() {
        let joiner = Joiner::equal(WasmFunction::new("get_value"));
        let cloned = joiner.clone();
        assert_eq!(joiner, cloned);
    }

    #[test]
    fn test_joiner_debug() {
        let joiner = Joiner::filtering(WasmFunction::new("test"));
        let debug = format!("{:?}", joiner);
        assert!(debug.contains("Filtering"));
        assert!(debug.contains("test"));
    }
}
