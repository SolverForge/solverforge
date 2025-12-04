use crate::constraints::{Collector, Joiner, WasmFunction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StreamComponent {
    #[serde(rename = "forEach")]
    ForEach {
        #[serde(rename = "className")]
        class_name: String,
    },
    #[serde(rename = "forEachIncludingUnassigned")]
    ForEachIncludingUnassigned {
        #[serde(rename = "className")]
        class_name: String,
    },
    #[serde(rename = "forEachUniquePair")]
    ForEachUniquePair {
        #[serde(rename = "className")]
        class_name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        joiners: Vec<Joiner>,
    },
    #[serde(rename = "filter")]
    Filter { predicate: WasmFunction },
    #[serde(rename = "join")]
    Join {
        #[serde(rename = "className")]
        class_name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        joiners: Vec<Joiner>,
    },
    #[serde(rename = "ifExists")]
    IfExists {
        #[serde(rename = "className")]
        class_name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        joiners: Vec<Joiner>,
    },
    #[serde(rename = "ifNotExists")]
    IfNotExists {
        #[serde(rename = "className")]
        class_name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        joiners: Vec<Joiner>,
    },
    #[serde(rename = "groupBy")]
    GroupBy {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        keys: Vec<WasmFunction>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        aggregators: Vec<Collector>,
    },
    #[serde(rename = "map")]
    Map {
        #[serde(rename = "mapper")]
        mappers: Vec<WasmFunction>,
    },
    #[serde(rename = "flattenLast")]
    FlattenLast {
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<WasmFunction>,
    },
    #[serde(rename = "expand")]
    Expand {
        #[serde(rename = "mapper")]
        mappers: Vec<WasmFunction>,
    },
    #[serde(rename = "complement")]
    Complement {
        #[serde(rename = "className")]
        class_name: String,
    },
    #[serde(rename = "penalize")]
    Penalize {
        weight: String,
        #[serde(rename = "scaleBy", skip_serializing_if = "Option::is_none")]
        scale_by: Option<WasmFunction>,
    },
    #[serde(rename = "reward")]
    Reward {
        weight: String,
        #[serde(rename = "scaleBy", skip_serializing_if = "Option::is_none")]
        scale_by: Option<WasmFunction>,
    },
}

impl StreamComponent {
    pub fn for_each(class_name: impl Into<String>) -> Self {
        StreamComponent::ForEach {
            class_name: class_name.into(),
        }
    }

    pub fn for_each_including_unassigned(class_name: impl Into<String>) -> Self {
        StreamComponent::ForEachIncludingUnassigned {
            class_name: class_name.into(),
        }
    }

    pub fn for_each_unique_pair(class_name: impl Into<String>) -> Self {
        StreamComponent::ForEachUniquePair {
            class_name: class_name.into(),
            joiners: Vec::new(),
        }
    }

    pub fn for_each_unique_pair_with_joiners(
        class_name: impl Into<String>,
        joiners: Vec<Joiner>,
    ) -> Self {
        StreamComponent::ForEachUniquePair {
            class_name: class_name.into(),
            joiners,
        }
    }

    pub fn filter(predicate: WasmFunction) -> Self {
        StreamComponent::Filter { predicate }
    }

    pub fn join(class_name: impl Into<String>) -> Self {
        StreamComponent::Join {
            class_name: class_name.into(),
            joiners: Vec::new(),
        }
    }

    pub fn join_with_joiners(class_name: impl Into<String>, joiners: Vec<Joiner>) -> Self {
        StreamComponent::Join {
            class_name: class_name.into(),
            joiners,
        }
    }

    pub fn if_exists(class_name: impl Into<String>) -> Self {
        StreamComponent::IfExists {
            class_name: class_name.into(),
            joiners: Vec::new(),
        }
    }

    pub fn if_exists_with_joiners(class_name: impl Into<String>, joiners: Vec<Joiner>) -> Self {
        StreamComponent::IfExists {
            class_name: class_name.into(),
            joiners,
        }
    }

    pub fn if_not_exists(class_name: impl Into<String>) -> Self {
        StreamComponent::IfNotExists {
            class_name: class_name.into(),
            joiners: Vec::new(),
        }
    }

    pub fn if_not_exists_with_joiners(class_name: impl Into<String>, joiners: Vec<Joiner>) -> Self {
        StreamComponent::IfNotExists {
            class_name: class_name.into(),
            joiners,
        }
    }

    pub fn group_by(keys: Vec<WasmFunction>, aggregators: Vec<Collector>) -> Self {
        StreamComponent::GroupBy { keys, aggregators }
    }

    pub fn group_by_key(key: WasmFunction) -> Self {
        StreamComponent::GroupBy {
            keys: vec![key],
            aggregators: Vec::new(),
        }
    }

    pub fn group_by_collector(aggregator: Collector) -> Self {
        StreamComponent::GroupBy {
            keys: Vec::new(),
            aggregators: vec![aggregator],
        }
    }

    pub fn map(mappers: Vec<WasmFunction>) -> Self {
        StreamComponent::Map { mappers }
    }

    pub fn map_single(mapper: WasmFunction) -> Self {
        StreamComponent::Map {
            mappers: vec![mapper],
        }
    }

    pub fn flatten_last() -> Self {
        StreamComponent::FlattenLast { map: None }
    }

    pub fn flatten_last_with_map(map: WasmFunction) -> Self {
        StreamComponent::FlattenLast { map: Some(map) }
    }

    pub fn expand(mappers: Vec<WasmFunction>) -> Self {
        StreamComponent::Expand { mappers }
    }

    pub fn complement(class_name: impl Into<String>) -> Self {
        StreamComponent::Complement {
            class_name: class_name.into(),
        }
    }

    pub fn penalize(weight: impl Into<String>) -> Self {
        StreamComponent::Penalize {
            weight: weight.into(),
            scale_by: None,
        }
    }

    pub fn penalize_with_weigher(weight: impl Into<String>, scale_by: WasmFunction) -> Self {
        StreamComponent::Penalize {
            weight: weight.into(),
            scale_by: Some(scale_by),
        }
    }

    pub fn reward(weight: impl Into<String>) -> Self {
        StreamComponent::Reward {
            weight: weight.into(),
            scale_by: None,
        }
    }

    pub fn reward_with_weigher(weight: impl Into<String>, scale_by: WasmFunction) -> Self {
        StreamComponent::Reward {
            weight: weight.into(),
            scale_by: Some(scale_by),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_each() {
        let component = StreamComponent::for_each("Lesson");
        match component {
            StreamComponent::ForEach { class_name } => {
                assert_eq!(class_name, "Lesson");
            }
            _ => panic!("Expected ForEach"),
        }
    }

    #[test]
    fn test_for_each_including_unassigned() {
        let component = StreamComponent::for_each_including_unassigned("Lesson");
        match component {
            StreamComponent::ForEachIncludingUnassigned { class_name } => {
                assert_eq!(class_name, "Lesson");
            }
            _ => panic!("Expected ForEachIncludingUnassigned"),
        }
    }

    #[test]
    fn test_for_each_unique_pair() {
        let component = StreamComponent::for_each_unique_pair("Lesson");
        match component {
            StreamComponent::ForEachUniquePair {
                class_name,
                joiners,
            } => {
                assert_eq!(class_name, "Lesson");
                assert!(joiners.is_empty());
            }
            _ => panic!("Expected ForEachUniquePair"),
        }
    }

    #[test]
    fn test_for_each_unique_pair_with_joiners() {
        let component = StreamComponent::for_each_unique_pair_with_joiners(
            "Lesson",
            vec![Joiner::equal(WasmFunction::new("get_timeslot"))],
        );
        match component {
            StreamComponent::ForEachUniquePair { joiners, .. } => {
                assert_eq!(joiners.len(), 1);
            }
            _ => panic!("Expected ForEachUniquePair"),
        }
    }

    #[test]
    fn test_filter() {
        let component = StreamComponent::filter(WasmFunction::new("is_valid"));
        match component {
            StreamComponent::Filter { predicate } => {
                assert_eq!(predicate.name(), "is_valid");
            }
            _ => panic!("Expected Filter"),
        }
    }

    #[test]
    fn test_join() {
        let component = StreamComponent::join("Room");
        match component {
            StreamComponent::Join {
                class_name,
                joiners,
            } => {
                assert_eq!(class_name, "Room");
                assert!(joiners.is_empty());
            }
            _ => panic!("Expected Join"),
        }
    }

    #[test]
    fn test_join_with_joiners() {
        let component = StreamComponent::join_with_joiners(
            "Room",
            vec![Joiner::equal(WasmFunction::new("get_room"))],
        );
        match component {
            StreamComponent::Join { joiners, .. } => {
                assert_eq!(joiners.len(), 1);
            }
            _ => panic!("Expected Join"),
        }
    }

    #[test]
    fn test_if_exists() {
        let component = StreamComponent::if_exists("Conflict");
        match component {
            StreamComponent::IfExists { class_name, .. } => {
                assert_eq!(class_name, "Conflict");
            }
            _ => panic!("Expected IfExists"),
        }
    }

    #[test]
    fn test_if_not_exists() {
        let component = StreamComponent::if_not_exists("Conflict");
        match component {
            StreamComponent::IfNotExists { class_name, .. } => {
                assert_eq!(class_name, "Conflict");
            }
            _ => panic!("Expected IfNotExists"),
        }
    }

    #[test]
    fn test_group_by() {
        let component = StreamComponent::group_by(
            vec![WasmFunction::new("get_room")],
            vec![Collector::count()],
        );
        match component {
            StreamComponent::GroupBy { keys, aggregators } => {
                assert_eq!(keys.len(), 1);
                assert_eq!(aggregators.len(), 1);
            }
            _ => panic!("Expected GroupBy"),
        }
    }

    #[test]
    fn test_group_by_key() {
        let component = StreamComponent::group_by_key(WasmFunction::new("get_room"));
        match component {
            StreamComponent::GroupBy { keys, aggregators } => {
                assert_eq!(keys.len(), 1);
                assert!(aggregators.is_empty());
            }
            _ => panic!("Expected GroupBy"),
        }
    }

    #[test]
    fn test_group_by_collector() {
        let component = StreamComponent::group_by_collector(Collector::count());
        match component {
            StreamComponent::GroupBy { keys, aggregators } => {
                assert!(keys.is_empty());
                assert_eq!(aggregators.len(), 1);
            }
            _ => panic!("Expected GroupBy"),
        }
    }

    #[test]
    fn test_map() {
        let component =
            StreamComponent::map(vec![WasmFunction::new("get_a"), WasmFunction::new("get_b")]);
        match component {
            StreamComponent::Map { mappers } => {
                assert_eq!(mappers.len(), 2);
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_map_single() {
        let component = StreamComponent::map_single(WasmFunction::new("get_value"));
        match component {
            StreamComponent::Map { mappers } => {
                assert_eq!(mappers.len(), 1);
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_flatten_last() {
        let component = StreamComponent::flatten_last();
        match component {
            StreamComponent::FlattenLast { map } => {
                assert!(map.is_none());
            }
            _ => panic!("Expected FlattenLast"),
        }
    }

    #[test]
    fn test_flatten_last_with_map() {
        let component = StreamComponent::flatten_last_with_map(WasmFunction::new("get_items"));
        match component {
            StreamComponent::FlattenLast { map } => {
                assert!(map.is_some());
            }
            _ => panic!("Expected FlattenLast"),
        }
    }

    #[test]
    fn test_expand() {
        let component = StreamComponent::expand(vec![WasmFunction::new("get_extra")]);
        match component {
            StreamComponent::Expand { mappers } => {
                assert_eq!(mappers.len(), 1);
            }
            _ => panic!("Expected Expand"),
        }
    }

    #[test]
    fn test_complement() {
        let component = StreamComponent::complement("Timeslot");
        match component {
            StreamComponent::Complement { class_name } => {
                assert_eq!(class_name, "Timeslot");
            }
            _ => panic!("Expected Complement"),
        }
    }

    #[test]
    fn test_penalize() {
        let component = StreamComponent::penalize("1hard");
        match component {
            StreamComponent::Penalize { weight, scale_by } => {
                assert_eq!(weight, "1hard");
                assert!(scale_by.is_none());
            }
            _ => panic!("Expected Penalize"),
        }
    }

    #[test]
    fn test_penalize_with_weigher() {
        let component =
            StreamComponent::penalize_with_weigher("1hard", WasmFunction::new("get_weight"));
        match component {
            StreamComponent::Penalize { weight, scale_by } => {
                assert_eq!(weight, "1hard");
                assert!(scale_by.is_some());
            }
            _ => panic!("Expected Penalize"),
        }
    }

    #[test]
    fn test_reward() {
        let component = StreamComponent::reward("1soft");
        match component {
            StreamComponent::Reward { weight, scale_by } => {
                assert_eq!(weight, "1soft");
                assert!(scale_by.is_none());
            }
            _ => panic!("Expected Reward"),
        }
    }

    #[test]
    fn test_reward_with_weigher() {
        let component =
            StreamComponent::reward_with_weigher("1soft", WasmFunction::new("get_bonus"));
        match component {
            StreamComponent::Reward { scale_by, .. } => {
                assert!(scale_by.is_some());
            }
            _ => panic!("Expected Reward"),
        }
    }

    #[test]
    fn test_for_each_json_serialization() {
        let component = StreamComponent::for_each("Lesson");
        let json = serde_json::to_string(&component).unwrap();
        assert!(json.contains("\"kind\":\"forEach\""));
        assert!(json.contains("\"className\":\"Lesson\""));

        let parsed: StreamComponent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, component);
    }

    #[test]
    fn test_filter_json_serialization() {
        let component = StreamComponent::filter(WasmFunction::new("is_valid"));
        let json = serde_json::to_string(&component).unwrap();
        assert!(json.contains("\"kind\":\"filter\""));
        assert!(json.contains("\"predicate\":\"is_valid\""));

        let parsed: StreamComponent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, component);
    }

    #[test]
    fn test_join_json_serialization() {
        let component = StreamComponent::join_with_joiners(
            "Room",
            vec![Joiner::equal(WasmFunction::new("get_room"))],
        );
        let json = serde_json::to_string(&component).unwrap();
        assert!(json.contains("\"kind\":\"join\""));
        assert!(json.contains("\"className\":\"Room\""));
        assert!(json.contains("\"joiners\""));

        let parsed: StreamComponent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, component);
    }

    #[test]
    fn test_group_by_json_serialization() {
        let component = StreamComponent::group_by(
            vec![WasmFunction::new("get_room")],
            vec![Collector::count()],
        );
        let json = serde_json::to_string(&component).unwrap();
        assert!(json.contains("\"kind\":\"groupBy\""));
        assert!(json.contains("\"keys\""));
        assert!(json.contains("\"aggregators\""));

        let parsed: StreamComponent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, component);
    }

    #[test]
    fn test_penalize_json_serialization() {
        let component = StreamComponent::penalize("1hard");
        let json = serde_json::to_string(&component).unwrap();
        assert!(json.contains("\"kind\":\"penalize\""));
        assert!(json.contains("\"weight\":\"1hard\""));

        let parsed: StreamComponent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, component);
    }

    #[test]
    fn test_component_clone() {
        let component = StreamComponent::for_each("Lesson");
        let cloned = component.clone();
        assert_eq!(component, cloned);
    }

    #[test]
    fn test_component_debug() {
        let component = StreamComponent::for_each("Lesson");
        let debug = format!("{:?}", component);
        assert!(debug.contains("ForEach"));
    }
}
