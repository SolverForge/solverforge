use crate::constraints::{Collector, Constraint, StreamComponent};
use serde::{Deserialize, Serialize};

/// Indicates the level of incremental scoring support for a constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IncrementalSupport {
    /// The constraint can be fully incrementalized - delta scoring is possible.
    /// Examples: forEach + filter + penalize, forEach + join (equality) + penalize
    FullyIncremental,

    /// The constraint requires full re-evaluation on every move.
    /// Examples: groupBy with loadBalance, complement, complex aggregators
    NonIncremental,
}

/// Analysis result for a constraint's incremental scoring eligibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintAnalysis {
    /// The constraint name
    pub name: String,

    /// Level of incremental support
    pub support: IncrementalSupport,

    /// Entity types affected by this constraint
    pub affected_entities: Vec<String>,

    /// Optional reason why the constraint is non-incremental
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Analyzes constraints to determine incremental scoring eligibility.
pub struct ConstraintAnalyzer;

impl ConstraintAnalyzer {
    /// Analyze a constraint to determine if it supports incremental scoring.
    ///
    /// # Incremental Eligibility Rules
    ///
    /// A constraint is **FullyIncremental** if it contains:
    /// - forEach + filter (pure predicates) + penalize/reward
    /// - forEach + join (equality joiners) + filter + penalize/reward
    /// - forEachUniquePair (equality joiners) + filter + penalize/reward
    ///
    /// A constraint is **NonIncremental** if it contains:
    /// - complement (requires full entity set knowledge)
    /// - groupBy with complex aggregators (loadBalance, compose)
    /// - Any component that requires global state
    ///
    /// # Arguments
    ///
    /// * `constraint` - The constraint to analyze
    ///
    /// # Returns
    ///
    /// A `ConstraintAnalysis` with the support level and affected entities.
    pub fn analyze(constraint: &Constraint) -> ConstraintAnalysis {
        let mut affected_entities = Vec::new();
        let mut support = IncrementalSupport::FullyIncremental;
        let mut reason = None;

        // Analyze each component in the constraint stream
        for component in &constraint.components {
            match component {
                // Collect entity types from forEach/join operations
                StreamComponent::ForEach { class_name } => {
                    if !affected_entities.contains(class_name) {
                        affected_entities.push(class_name.clone());
                    }
                }
                StreamComponent::ForEachUniquePair { class_name, .. } => {
                    if !affected_entities.contains(class_name) {
                        affected_entities.push(class_name.clone());
                    }
                }
                StreamComponent::Join { class_name, .. } => {
                    if !affected_entities.contains(class_name) {
                        affected_entities.push(class_name.clone());
                    }
                }

                // Complement requires full entity set - cannot be incremental
                StreamComponent::Complement { .. } => {
                    support = IncrementalSupport::NonIncremental;
                    reason =
                        Some("Complement operation requires full entity set knowledge".to_string());
                }

                // GroupBy with complex aggregators cannot be incremental
                StreamComponent::GroupBy { aggregators, .. } => {
                    // Check if any aggregator is complex (loadBalance, compose, etc.)
                    for aggregator in aggregators {
                        if Self::is_complex_collector(aggregator) {
                            support = IncrementalSupport::NonIncremental;
                            reason = Some(format!(
                                "GroupBy with complex aggregator '{}' requires full re-evaluation",
                                Self::collector_name(aggregator)
                            ));
                            break;
                        }
                    }
                }

                // Other components (Filter, Map, Penalize, Reward, etc.) are fine
                _ => {}
            }

            // Early exit if we've determined it's non-incremental
            if support == IncrementalSupport::NonIncremental {
                break;
            }
        }

        ConstraintAnalysis {
            name: constraint.name.clone(),
            support,
            affected_entities,
            reason,
        }
    }

    /// Check if a collector is complex and requires full re-evaluation.
    fn is_complex_collector(collector: &Collector) -> bool {
        // Complex collectors that require global state and full re-evaluation:
        // - LoadBalance: Requires fairness calculation across all entities
        // - Compose: Combines multiple collectors, may include complex ones
        // - Conditionally/CollectAndThen: Depends on inner collector
        //
        // Simple collectors that can be incrementalized:
        // - Count, Sum, Average, Min, Max, ToList, ToSet
        use crate::constraints::Collector;

        matches!(
            collector,
            Collector::LoadBalance { .. } | Collector::Compose { .. }
        )
    }

    /// Get a human-readable name for a collector.
    fn collector_name(collector: &Collector) -> String {
        use crate::constraints::Collector;

        match collector {
            Collector::Count { .. } => "count",
            Collector::Sum { .. } => "sum",
            Collector::Average { .. } => "average",
            Collector::Min { .. } => "min",
            Collector::Max { .. } => "max",
            Collector::ToList { .. } => "toList",
            Collector::ToSet { .. } => "toSet",
            Collector::Compose { .. } => "compose",
            Collector::Conditionally { .. } => "conditionally",
            Collector::CollectAndThen { .. } => "collectAndThen",
            Collector::LoadBalance { .. } => "loadBalance",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::{Collector, Joiner, WasmFunction};

    #[test]
    fn test_simple_foreach_filter_is_incremental() {
        // forEach("Shift").filter(predicate).penalize()
        let constraint = Constraint::new("requiredSkill")
            .with_component(StreamComponent::ForEach {
                class_name: "Shift".to_string(),
            })
            .with_component(StreamComponent::Filter {
                predicate: WasmFunction::new("has_required_skill"),
            })
            .with_component(StreamComponent::Penalize {
                weight: "ONE_HARD".to_string(),
                scale_by: None,
            });

        let analysis = ConstraintAnalyzer::analyze(&constraint);

        assert_eq!(analysis.support, IncrementalSupport::FullyIncremental);
        assert_eq!(analysis.affected_entities, vec!["Shift"]);
        assert_eq!(analysis.reason, None);
    }

    #[test]
    fn test_foreach_join_filter_is_incremental() {
        // forEach("Shift").join("Shift", equal(employee)).filter(predicate).penalize()
        let constraint = Constraint::new("noOverlappingShifts")
            .with_component(StreamComponent::ForEach {
                class_name: "Shift".to_string(),
            })
            .with_component(StreamComponent::Join {
                class_name: "Shift".to_string(),
                joiners: vec![Joiner::equal(WasmFunction::new("get_employee"))],
            })
            .with_component(StreamComponent::Filter {
                predicate: WasmFunction::new("shifts_overlap"),
            })
            .with_component(StreamComponent::Penalize {
                weight: "ONE_HARD".to_string(),
                scale_by: None,
            });

        let analysis = ConstraintAnalyzer::analyze(&constraint);

        assert_eq!(analysis.support, IncrementalSupport::FullyIncremental);
        assert_eq!(analysis.affected_entities, vec!["Shift"]);
        assert_eq!(analysis.reason, None);
    }

    #[test]
    fn test_foreach_unique_pair_is_incremental() {
        // forEachUniquePair("Shift", equal(employee)).filter(predicate).penalize()
        let constraint = Constraint::new("noOverlappingShifts")
            .with_component(StreamComponent::ForEachUniquePair {
                class_name: "Shift".to_string(),
                joiners: vec![Joiner::equal(WasmFunction::new("get_employee"))],
            })
            .with_component(StreamComponent::Filter {
                predicate: WasmFunction::new("shifts_overlap"),
            })
            .with_component(StreamComponent::Penalize {
                weight: "ONE_HARD".to_string(),
                scale_by: None,
            });

        let analysis = ConstraintAnalyzer::analyze(&constraint);

        assert_eq!(analysis.support, IncrementalSupport::FullyIncremental);
        assert_eq!(analysis.affected_entities, vec!["Shift"]);
        assert_eq!(analysis.reason, None);
    }

    #[test]
    fn test_complement_is_non_incremental() {
        // forEach("Shift").complement("Employee").penalize()
        let constraint = Constraint::new("unassignedEmployees")
            .with_component(StreamComponent::ForEach {
                class_name: "Shift".to_string(),
            })
            .with_component(StreamComponent::Complement {
                class_name: "Employee".to_string(),
            })
            .with_component(StreamComponent::Penalize {
                weight: "ONE_HARD".to_string(),
                scale_by: None,
            });

        let analysis = ConstraintAnalyzer::analyze(&constraint);

        assert_eq!(analysis.support, IncrementalSupport::NonIncremental);
        assert!(analysis
            .reason
            .unwrap()
            .contains("Complement operation requires full entity set"));
    }

    #[test]
    fn test_multiple_entities_tracked() {
        // forEach("Shift").join("Employee").penalize()
        let constraint = Constraint::new("multiEntity")
            .with_component(StreamComponent::ForEach {
                class_name: "Shift".to_string(),
            })
            .with_component(StreamComponent::Join {
                class_name: "Employee".to_string(),
                joiners: vec![],
            })
            .with_component(StreamComponent::Penalize {
                weight: "ONE_HARD".to_string(),
                scale_by: None,
            });

        let analysis = ConstraintAnalyzer::analyze(&constraint);

        assert_eq!(analysis.support, IncrementalSupport::FullyIncremental);
        assert_eq!(analysis.affected_entities, vec!["Shift", "Employee"]);
    }

    #[test]
    fn test_serialization_deserialization() {
        let analysis = ConstraintAnalysis {
            name: "testConstraint".to_string(),
            support: IncrementalSupport::FullyIncremental,
            affected_entities: vec!["Shift".to_string()],
            reason: None,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&analysis).unwrap();

        // Deserialize back
        let deserialized: ConstraintAnalysis = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, analysis);
    }

    #[test]
    fn test_serialization_with_reason() {
        let analysis = ConstraintAnalysis {
            name: "complexConstraint".to_string(),
            support: IncrementalSupport::NonIncremental,
            affected_entities: vec!["Shift".to_string()],
            reason: Some("Complex aggregator".to_string()),
        };

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: ConstraintAnalysis = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, analysis);
        assert_eq!(deserialized.reason, Some("Complex aggregator".to_string()));
    }

    #[test]
    fn test_groupby_with_loadbalance_is_non_incremental() {
        // forEach("Shift").groupBy(employee, loadBalance()).penalize()
        let constraint = Constraint::new("balanceEmployeeShifts")
            .with_component(StreamComponent::ForEach {
                class_name: "Shift".to_string(),
            })
            .with_component(StreamComponent::GroupBy {
                keys: vec![WasmFunction::new("get_employee")],
                aggregators: vec![Collector::load_balance(WasmFunction::new("identity"))],
            })
            .with_component(StreamComponent::Penalize {
                weight: "ONE_SOFT".to_string(),
                scale_by: None,
            });

        let analysis = ConstraintAnalyzer::analyze(&constraint);

        assert_eq!(analysis.support, IncrementalSupport::NonIncremental);
        assert!(analysis
            .reason
            .unwrap()
            .contains("GroupBy with complex aggregator 'loadBalance'"));
    }

    #[test]
    fn test_groupby_with_simple_aggregator_is_incremental() {
        // forEach("Shift").groupBy(employee, count()).penalize()
        let constraint = Constraint::new("countShiftsPerEmployee")
            .with_component(StreamComponent::ForEach {
                class_name: "Shift".to_string(),
            })
            .with_component(StreamComponent::GroupBy {
                keys: vec![WasmFunction::new("get_employee")],
                aggregators: vec![Collector::count()],
            })
            .with_component(StreamComponent::Penalize {
                weight: "ONE_SOFT".to_string(),
                scale_by: None,
            });

        let analysis = ConstraintAnalyzer::analyze(&constraint);

        // Simple aggregators like count could potentially be incremental
        // For now, we're classifying them as incremental in the analyzer
        assert_eq!(analysis.support, IncrementalSupport::FullyIncremental);
    }
}
