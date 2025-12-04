use crate::constraints::WasmFunction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum Collector {
    #[serde(rename = "count")]
    Count {
        #[serde(skip_serializing_if = "Option::is_none")]
        distinct: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<WasmFunction>,
    },
    #[serde(rename = "sum")]
    Sum { map: WasmFunction },
    #[serde(rename = "average")]
    Average { map: WasmFunction },
    #[serde(rename = "min")]
    Min {
        map: WasmFunction,
        comparator: WasmFunction,
    },
    #[serde(rename = "max")]
    Max {
        map: WasmFunction,
        comparator: WasmFunction,
    },
    #[serde(rename = "toList")]
    ToList {
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<WasmFunction>,
    },
    #[serde(rename = "toSet")]
    ToSet {
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<WasmFunction>,
    },
    #[serde(rename = "compose")]
    Compose {
        collectors: Vec<Collector>,
        combiner: WasmFunction,
    },
    #[serde(rename = "conditionally")]
    Conditionally {
        predicate: WasmFunction,
        collector: Box<Collector>,
    },
    #[serde(rename = "collectAndThen")]
    CollectAndThen {
        collector: Box<Collector>,
        mapper: WasmFunction,
    },
    #[serde(rename = "loadBalance")]
    LoadBalance {
        map: WasmFunction,
        #[serde(skip_serializing_if = "Option::is_none")]
        load: Option<WasmFunction>,
    },
}

impl Collector {
    pub fn count() -> Self {
        Collector::Count {
            distinct: None,
            map: None,
        }
    }

    pub fn count_distinct() -> Self {
        Collector::Count {
            distinct: Some(true),
            map: None,
        }
    }

    pub fn count_with_map(map: WasmFunction) -> Self {
        Collector::Count {
            distinct: None,
            map: Some(map),
        }
    }

    pub fn sum(map: WasmFunction) -> Self {
        Collector::Sum { map }
    }

    pub fn average(map: WasmFunction) -> Self {
        Collector::Average { map }
    }

    pub fn min(map: WasmFunction, comparator: WasmFunction) -> Self {
        Collector::Min { map, comparator }
    }

    pub fn max(map: WasmFunction, comparator: WasmFunction) -> Self {
        Collector::Max { map, comparator }
    }

    pub fn to_list() -> Self {
        Collector::ToList { map: None }
    }

    pub fn to_list_with_map(map: WasmFunction) -> Self {
        Collector::ToList { map: Some(map) }
    }

    pub fn to_set() -> Self {
        Collector::ToSet { map: None }
    }

    pub fn to_set_with_map(map: WasmFunction) -> Self {
        Collector::ToSet { map: Some(map) }
    }

    pub fn compose(collectors: Vec<Collector>, combiner: WasmFunction) -> Self {
        Collector::Compose {
            collectors,
            combiner,
        }
    }

    pub fn conditionally(predicate: WasmFunction, collector: Collector) -> Self {
        Collector::Conditionally {
            predicate,
            collector: Box::new(collector),
        }
    }

    pub fn collect_and_then(collector: Collector, mapper: WasmFunction) -> Self {
        Collector::CollectAndThen {
            collector: Box::new(collector),
            mapper,
        }
    }

    pub fn load_balance(map: WasmFunction) -> Self {
        Collector::LoadBalance { map, load: None }
    }

    pub fn load_balance_with_load(map: WasmFunction, load: WasmFunction) -> Self {
        Collector::LoadBalance {
            map,
            load: Some(load),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count() {
        let collector = Collector::count();
        match collector {
            Collector::Count { distinct, map } => {
                assert!(distinct.is_none());
                assert!(map.is_none());
            }
            _ => panic!("Expected Count collector"),
        }
    }

    #[test]
    fn test_count_distinct() {
        let collector = Collector::count_distinct();
        match collector {
            Collector::Count { distinct, .. } => {
                assert_eq!(distinct, Some(true));
            }
            _ => panic!("Expected Count collector"),
        }
    }

    #[test]
    fn test_count_with_map() {
        let collector = Collector::count_with_map(WasmFunction::new("get_id"));
        match collector {
            Collector::Count { map, .. } => {
                assert!(map.is_some());
                assert_eq!(map.unwrap().name(), "get_id");
            }
            _ => panic!("Expected Count collector"),
        }
    }

    #[test]
    fn test_sum() {
        let collector = Collector::sum(WasmFunction::new("get_value"));
        match collector {
            Collector::Sum { map } => {
                assert_eq!(map.name(), "get_value");
            }
            _ => panic!("Expected Sum collector"),
        }
    }

    #[test]
    fn test_average() {
        let collector = Collector::average(WasmFunction::new("get_score"));
        match collector {
            Collector::Average { map } => {
                assert_eq!(map.name(), "get_score");
            }
            _ => panic!("Expected Average collector"),
        }
    }

    #[test]
    fn test_min() {
        let collector = Collector::min(
            WasmFunction::new("get_time"),
            WasmFunction::new("compare_time"),
        );
        match collector {
            Collector::Min { map, comparator } => {
                assert_eq!(map.name(), "get_time");
                assert_eq!(comparator.name(), "compare_time");
            }
            _ => panic!("Expected Min collector"),
        }
    }

    #[test]
    fn test_max() {
        let collector = Collector::max(
            WasmFunction::new("get_priority"),
            WasmFunction::new("compare_priority"),
        );
        match collector {
            Collector::Max { map, comparator } => {
                assert_eq!(map.name(), "get_priority");
                assert_eq!(comparator.name(), "compare_priority");
            }
            _ => panic!("Expected Max collector"),
        }
    }

    #[test]
    fn test_to_list() {
        let collector = Collector::to_list();
        match collector {
            Collector::ToList { map } => {
                assert!(map.is_none());
            }
            _ => panic!("Expected ToList collector"),
        }
    }

    #[test]
    fn test_to_list_with_map() {
        let collector = Collector::to_list_with_map(WasmFunction::new("get_name"));
        match collector {
            Collector::ToList { map } => {
                assert!(map.is_some());
            }
            _ => panic!("Expected ToList collector"),
        }
    }

    #[test]
    fn test_to_set() {
        let collector = Collector::to_set();
        match collector {
            Collector::ToSet { map } => {
                assert!(map.is_none());
            }
            _ => panic!("Expected ToSet collector"),
        }
    }

    #[test]
    fn test_compose() {
        let collector = Collector::compose(
            vec![
                Collector::count(),
                Collector::sum(WasmFunction::new("get_value")),
            ],
            WasmFunction::new("combine"),
        );
        match collector {
            Collector::Compose {
                collectors,
                combiner,
            } => {
                assert_eq!(collectors.len(), 2);
                assert_eq!(combiner.name(), "combine");
            }
            _ => panic!("Expected Compose collector"),
        }
    }

    #[test]
    fn test_conditionally() {
        let collector = Collector::conditionally(WasmFunction::new("is_valid"), Collector::count());
        match collector {
            Collector::Conditionally {
                predicate,
                collector,
            } => {
                assert_eq!(predicate.name(), "is_valid");
                matches!(*collector, Collector::Count { .. });
            }
            _ => panic!("Expected Conditionally collector"),
        }
    }

    #[test]
    fn test_collect_and_then() {
        let collector =
            Collector::collect_and_then(Collector::count(), WasmFunction::new("to_string"));
        match collector {
            Collector::CollectAndThen { collector, mapper } => {
                matches!(*collector, Collector::Count { .. });
                assert_eq!(mapper.name(), "to_string");
            }
            _ => panic!("Expected CollectAndThen collector"),
        }
    }

    #[test]
    fn test_load_balance() {
        let collector = Collector::load_balance(WasmFunction::new("get_employee"));
        match collector {
            Collector::LoadBalance { map, load } => {
                assert_eq!(map.name(), "get_employee");
                assert!(load.is_none());
            }
            _ => panic!("Expected LoadBalance collector"),
        }
    }

    #[test]
    fn test_load_balance_with_load() {
        let collector = Collector::load_balance_with_load(
            WasmFunction::new("get_employee"),
            WasmFunction::new("get_load"),
        );
        match collector {
            Collector::LoadBalance { map, load } => {
                assert_eq!(map.name(), "get_employee");
                assert!(load.is_some());
            }
            _ => panic!("Expected LoadBalance collector"),
        }
    }

    #[test]
    fn test_count_json_serialization() {
        let collector = Collector::count();
        let json = serde_json::to_string(&collector).unwrap();
        assert!(json.contains("\"name\":\"count\""));

        let parsed: Collector = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, collector);
    }

    #[test]
    fn test_sum_json_serialization() {
        let collector = Collector::sum(WasmFunction::new("get_value"));
        let json = serde_json::to_string(&collector).unwrap();
        assert!(json.contains("\"name\":\"sum\""));
        assert!(json.contains("\"map\":\"get_value\""));

        let parsed: Collector = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, collector);
    }

    #[test]
    fn test_compose_json_serialization() {
        let collector = Collector::compose(vec![Collector::count()], WasmFunction::new("wrap"));
        let json = serde_json::to_string(&collector).unwrap();
        assert!(json.contains("\"name\":\"compose\""));
        assert!(json.contains("\"collectors\""));
        assert!(json.contains("\"combiner\":\"wrap\""));

        let parsed: Collector = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, collector);
    }

    #[test]
    fn test_conditionally_json_serialization() {
        let collector = Collector::conditionally(WasmFunction::new("pred"), Collector::count());
        let json = serde_json::to_string(&collector).unwrap();
        assert!(json.contains("\"name\":\"conditionally\""));
        assert!(json.contains("\"predicate\":\"pred\""));
        assert!(json.contains("\"collector\""));

        let parsed: Collector = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, collector);
    }

    #[test]
    fn test_collect_and_then_json_serialization() {
        let collector =
            Collector::collect_and_then(Collector::count(), WasmFunction::new("transform"));
        let json = serde_json::to_string(&collector).unwrap();
        assert!(json.contains("\"name\":\"collectAndThen\""));
        assert!(json.contains("\"mapper\":\"transform\""));

        let parsed: Collector = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, collector);
    }

    #[test]
    fn test_load_balance_json_serialization() {
        let collector = Collector::load_balance(WasmFunction::new("get_item"));
        let json = serde_json::to_string(&collector).unwrap();
        assert!(json.contains("\"name\":\"loadBalance\""));

        let parsed: Collector = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, collector);
    }

    #[test]
    fn test_nested_collectors_json() {
        let collector = Collector::compose(
            vec![
                Collector::conditionally(
                    WasmFunction::new("is_valid"),
                    Collector::sum(WasmFunction::new("get_value")),
                ),
                Collector::collect_and_then(Collector::count(), WasmFunction::new("double")),
            ],
            WasmFunction::new("combine_results"),
        );
        let json = serde_json::to_string(&collector).unwrap();
        let parsed: Collector = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, collector);
    }

    #[test]
    fn test_collector_clone() {
        let collector = Collector::sum(WasmFunction::new("get_value"));
        let cloned = collector.clone();
        assert_eq!(collector, cloned);
    }

    #[test]
    fn test_collector_debug() {
        let collector = Collector::count();
        let debug = format!("{:?}", collector);
        assert!(debug.contains("Count"));
    }
}
