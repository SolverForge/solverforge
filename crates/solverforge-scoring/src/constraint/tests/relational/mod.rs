/* Relational join test subtree: independent-key join verification.

Fixtures, an independent nested-loop oracle, and per-phase regression
suites live here. The oracle never calls production indexes or traversal;
production results must agree with it on scores, row multisets, and counts.
*/

mod analysis;
mod compiled_conditions;
mod complement_producer;
mod condition_arity;
mod condition_mutations;
mod conditions;
mod deep_chain;
mod derived_right_input;
mod evaluation;
mod existence_producer;
mod filtered_operators;
mod fixtures;
mod flatten_producer;
mod fluent_chain;
mod fluent_deep_chain;
mod group_producer;
mod grouped_join_fluent;
mod handle_map;
mod identity;
mod independent_keys;
mod indexes;
mod join_depth;
mod merged_operators;
mod operator_terminal;
mod operator_tree;
mod oracle;
mod owned_projection;
mod quad_fluent_chain;
mod row_keys;
mod self_join_node;
