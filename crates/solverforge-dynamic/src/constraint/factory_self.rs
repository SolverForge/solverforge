//! Factory functions for building self-join bi and tri constraints.
//!
//! Zero-fallback policy: if JIT compilation fails, we panic in debug and
//! log a critical error + panic in release. No interpreter fallback.

use std::sync::Arc;

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::constraint::nary_incremental::{
    IncrementalBiConstraint, IncrementalTriConstraint,
};

use super::closures_extract::make_extractor;
use crate::descriptor::DynamicDescriptor;
use crate::expr::Expr;
use crate::jit;
use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};

/// Enforce zero-fallback: JIT compilation must succeed.
/// Panics with a clear message in both debug and release builds.
fn require_jit<T>(result: Result<T, jit::JitError>, context: &str) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            let msg = format!(
                "CRITICAL: JIT compilation failed for {context}: {e}. \
                 Zero-fallback policy: all constraint expressions must be JIT-compilable."
            );
            // In release, a logging framework would capture this before the panic.
            // For now, eprintln ensures visibility in all contexts.
            eprintln!("{msg}");
            panic!("{msg}");
        }
    }
}

/// Builds a bi-constraint (self-join on single entity class).
///
/// All closures are JIT-compiled. If compilation fails, the solver panics
/// rather than silently falling back to the interpreter.
#[allow(clippy::too_many_arguments)]
pub fn build_bi_self_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx: usize,
    key_expr: Expr,
    filter_expr: Expr,
    weight_expr: Expr,
    _descriptor: DynamicDescriptor,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    let extractor = make_extractor(class_idx);

    // --- Key extractor: JIT, reads from flat buffer via solution ---
    let jit_key = Arc::new(require_jit(jit::compile_1(&key_expr), "key extractor"));
    let ci_key = class_idx;
    let key_extractor = Box::new(
        move |solution: &DynamicSolution,
              _entity: &DynamicEntity,
              entity_idx: usize|
              -> DynamicValue {
            let ptr = solution.flat_entity_ptr(ci_key, entity_idx);
            DynamicValue::I64(jit_key.call_1(ptr))
        },
    )
        as Box<dyn Fn(&DynamicSolution, &DynamicEntity, usize) -> DynamicValue + Send + Sync>;

    // --- Filter: JIT ---
    let jit_filter = Arc::new(require_jit(jit::compile_2(&filter_expr), "bi filter"));
    let ci_filter = class_idx;
    let filter = Box::new(
        move |solution: &DynamicSolution, a: &DynamicEntity, b: &DynamicEntity| {
            let a_loc = solution.get_entity_location(a.id);
            let b_loc = solution.get_entity_location(b.id);
            let (Some((_, a_idx)), Some((_, b_idx))) = (a_loc, b_loc) else {
                return false;
            };
            let a_ptr = solution.flat_entity_ptr(ci_filter, a_idx);
            let b_ptr = solution.flat_entity_ptr(ci_filter, b_idx);
            jit_filter.call_2(a_ptr, b_ptr) != 0
        },
    )
        as Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

    // --- Weight: JIT ---
    let jit_weight = Arc::new(require_jit(jit::compile_2(&weight_expr), "bi weight"));
    let ci_weight = class_idx;
    let hard = is_hard;
    let weight = Box::new(
        move |solution: &DynamicSolution, a_idx: usize, b_idx: usize| -> HardSoftScore {
            let a_ptr = solution.flat_entity_ptr(ci_weight, a_idx);
            let b_ptr = solution.flat_entity_ptr(ci_weight, b_idx);
            let w = jit_weight.call_2(a_ptr, b_ptr);
            if hard {
                HardSoftScore::of_hard(w)
            } else {
                HardSoftScore::of_soft(w)
            }
        },
    )
        as Box<dyn Fn(&DynamicSolution, usize, usize) -> HardSoftScore + Send + Sync>;

    Box::new(IncrementalBiConstraint::new(
        constraint_ref,
        impact_type,
        extractor,
        key_extractor,
        filter,
        weight,
        is_hard,
    ))
}

/// Builds a tri-constraint (self-join on single entity class).
///
/// Key extractor is JIT-compiled. Filter and weight remain interpreter
/// (tri constraints are rare; JIT for 3-param not yet implemented).
#[allow(clippy::too_many_arguments)]
pub fn build_tri_self_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx: usize,
    key_expr: Expr,
    filter_expr: Expr,
    weight_expr: Expr,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    use super::closures_extract::make_key_extractor;
    use super::closures_tri::{make_tri_filter, make_tri_weight};

    let extractor = make_extractor(class_idx);
    let key_extractor = make_key_extractor(key_expr, descriptor.clone());
    let filter = make_tri_filter(filter_expr, class_idx);
    let weight = make_tri_weight(weight_expr, class_idx, descriptor, is_hard);

    Box::new(IncrementalTriConstraint::new(
        constraint_ref,
        impact_type,
        extractor,
        key_extractor,
        filter,
        weight,
        is_hard,
    ))
}
