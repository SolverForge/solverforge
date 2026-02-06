//! Factory functions for building self-join bi and tri constraints.
//!
//! Zero-fallback policy: all closures are JIT-compiled via `compile_n`.
//! If compilation fails, the process panics. No interpreter fallback.

use std::sync::Arc;

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::constraint::nary_incremental::{
    IncrementalBiConstraint, IncrementalTriConstraint,
};

use super::closures_extract::make_extractor;
use crate::expr::Expr;
use crate::jit;
use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};

/// Builds a bi-constraint (self-join on single entity class).
///
/// All closures are JIT-compiled. Zero fallback.
#[allow(clippy::too_many_arguments)]
pub fn build_bi_self_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx: usize,
    key_expr: Expr,
    filter_expr: Expr,
    weight_expr: Expr,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    let extractor = make_extractor(class_idx);

    // --- Key extractor: JIT compile_1, reads from flat buffer via solution ---
    let jit_key = Arc::new(jit::compile_1(&key_expr));
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

    // --- Filter: JIT compile_2 --- indices passed directly, zero HashMap lookups
    let jit_filter = Arc::new(jit::compile_2(&filter_expr));
    let ci_filter = class_idx;
    let filter = Box::new(
        move |solution: &DynamicSolution,
              _a: &DynamicEntity,
              _b: &DynamicEntity,
              a_idx: usize,
              b_idx: usize| {
            let a_ptr = solution.flat_entity_ptr(ci_filter, a_idx);
            let b_ptr = solution.flat_entity_ptr(ci_filter, b_idx);
            jit_filter.call_2(a_ptr, b_ptr) != 0
        },
    )
        as Box<
            dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, usize, usize) -> bool
                + Send
                + Sync,
        >;

    // --- Weight: JIT compile_2 ---
    let jit_weight = Arc::new(jit::compile_2(&weight_expr));
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
/// All closures are JIT-compiled via compile_n. Zero fallback.
#[allow(clippy::too_many_arguments)]
pub fn build_tri_self_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx: usize,
    key_expr: Expr,
    filter_expr: Expr,
    weight_expr: Expr,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    let extractor = make_extractor(class_idx);

    // --- Key extractor: JIT compile_1 ---
    let jit_key = Arc::new(jit::compile_1(&key_expr));
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

    // --- Filter: JIT compile_n(3) ---
    // TriFilter doesn't pass indices yet (monomorphization pending).
    // Use get_entity_location to recover indices. This will be eliminated
    // when TriFilter::test gets index params, same as the bi path.
    let jit_filter = Arc::new(jit::compile_n(&filter_expr, 3));
    let ci_f = class_idx;
    let filter = Box::new(
        move |solution: &DynamicSolution,
              a: &DynamicEntity,
              b: &DynamicEntity,
              c: &DynamicEntity| {
            let a_idx = solution
                .get_entity_location(a.id)
                .map(|(_, i)| i)
                .unwrap_or(0);
            let b_idx = solution
                .get_entity_location(b.id)
                .map(|(_, i)| i)
                .unwrap_or(0);
            let c_idx = solution
                .get_entity_location(c.id)
                .map(|(_, i)| i)
                .unwrap_or(0);
            let ptrs = [
                solution.flat_entity_ptr(ci_f, a_idx),
                solution.flat_entity_ptr(ci_f, b_idx),
                solution.flat_entity_ptr(ci_f, c_idx),
            ];
            jit_filter.call_n(&ptrs) != 0
        },
    )
        as Box<
            dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> bool
                + Send
                + Sync,
        >;

    // --- Weight: JIT compile_n(3) via index-based weight ---
    let jit_weight = Arc::new(jit::compile_n(&weight_expr, 3));
    let ci_w = class_idx;
    let hard = is_hard;
    let weight = Box::new(
        move |solution: &DynamicSolution,
              a_idx: usize,
              b_idx: usize,
              c_idx: usize|
              -> HardSoftScore {
            let ptrs = [
                solution.flat_entity_ptr(ci_w, a_idx),
                solution.flat_entity_ptr(ci_w, b_idx),
                solution.flat_entity_ptr(ci_w, c_idx),
            ];
            let w = jit_weight.call_n(&ptrs);
            if hard {
                HardSoftScore::of_hard(w)
            } else {
                HardSoftScore::of_soft(w)
            }
        },
    )
        as Box<dyn Fn(&DynamicSolution, usize, usize, usize) -> HardSoftScore + Send + Sync>;

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
