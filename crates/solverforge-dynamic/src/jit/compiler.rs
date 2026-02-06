//! Cranelift JIT compiler for Expr trees.
//!
//! Compiles constraint expressions into native function pointers operating on
//! flat `*const i64` entity buffers. Each entity field occupies one i64 slot.
//!
//! # Memory Layout
//!
//! A `DynamicEntity` with fields `[I64(10), I64(3), I64(7)]` is flattened to:
//! ```text
//! offset 0:  10i64   (field 0)
//! offset 8:   3i64   (field 1)
//! offset 16:  7i64   (field 2)
//! ```
//!
//! String and complex fields are represented as i64 hash values or indices.
//! `DynamicValue::None` is encoded as `i64::MIN` (sentinel).
//!
//! # Compiled Function Signatures
//!
//! - `UniFilter`:  `fn(*const i64) -> i8`           (entity_a -> bool)
//! - `UniKey`:     `fn(*const i64) -> i64`           (entity_a -> key)
//! - `BiFilter`:   `fn(*const i64, *const i64) -> i8`  (entity_a, entity_b -> bool)
//! - `BiWeight`:   `fn(*const i64, *const i64) -> i64`  (entity_a, entity_b -> weight)

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::types::{I64, I8};
use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, Signature, UserFuncName};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

use crate::expr::Expr;

/// Sentinel value for `DynamicValue::None` in the flat buffer.
pub const NONE_SENTINEL: i64 = i64::MIN;

/// Compiled bi-filter: `fn(*const i64, *const i64) -> bool`
pub struct CompiledBiFilter {
    _module: JITModule,
    fn_ptr: unsafe fn(*const i64, *const i64) -> u8,
}

impl CompiledBiFilter {
    /// Evaluate the filter on two flat entity buffers.
    #[inline]
    pub fn call(&self, a: *const i64, b: *const i64) -> bool {
        unsafe { (self.fn_ptr)(a, b) != 0 }
    }
}

// SAFETY: The JITModule owns the code memory. The fn_ptr is valid for the
// lifetime of the module. We hold both together in the struct.
unsafe impl Send for CompiledBiFilter {}
unsafe impl Sync for CompiledBiFilter {}

/// Compiled uni key extractor: `fn(*const i64) -> i64`
pub struct CompiledUniKey {
    _module: JITModule,
    fn_ptr: unsafe fn(*const i64) -> i64,
}

impl CompiledUniKey {
    #[inline]
    pub fn call(&self, entity: *const i64) -> i64 {
        unsafe { (self.fn_ptr)(entity) }
    }
}

unsafe impl Send for CompiledUniKey {}
unsafe impl Sync for CompiledUniKey {}

/// Compiled bi weight: `fn(*const i64, *const i64) -> i64`
pub struct CompiledBiWeight {
    _module: JITModule,
    fn_ptr: unsafe fn(*const i64, *const i64) -> i64,
}

impl CompiledBiWeight {
    #[inline]
    pub fn call(&self, a: *const i64, b: *const i64) -> i64 {
        unsafe { (self.fn_ptr)(a, b) }
    }
}

unsafe impl Send for CompiledBiWeight {}
unsafe impl Sync for CompiledBiWeight {}

/// Errors from JIT compilation.
#[derive(Debug, thiserror::Error)]
pub enum JitError {
    #[error("Cranelift codegen error: {0}")]
    Codegen(String),
    #[error("Cranelift module error: {0}")]
    Module(#[from] cranelift_module::ModuleError),
    #[error("Unsupported expression for JIT: {0}")]
    Unsupported(String),
    #[error("Settings error: {0}")]
    Settings(#[from] settings::SetError),
}

/// JIT compiler that translates `Expr` trees to native code via Cranelift.
pub struct JitCompiler;

impl JitCompiler {
    /// Compile a bi-filter expression.
    ///
    /// The expression must evaluate to a boolean (i8: 0 or 1).
    /// Entity fields are accessed as `*(param_ptr + field_idx * 8)`.
    ///
    /// # Arguments
    /// * `expr` - Boolean expression referencing `Field { param_idx: 0|1, field_idx }`
    pub fn compile_bi_filter(expr: &Expr) -> Result<CompiledBiFilter, JitError> {
        let (module, fn_ptr) = compile_function(expr, FnShape::BiToBool)?;
        Ok(CompiledBiFilter {
            _module: module,
            fn_ptr: unsafe { std::mem::transmute(fn_ptr) },
        })
    }

    /// Compile a uni key extractor.
    ///
    /// The expression must evaluate to an i64.
    ///
    /// # Arguments
    /// * `expr` - Expression referencing `Field { param_idx: 0, field_idx }`
    pub fn compile_uni_key(expr: &Expr) -> Result<CompiledUniKey, JitError> {
        let (module, fn_ptr) = compile_function(expr, FnShape::UniToI64)?;
        Ok(CompiledUniKey {
            _module: module,
            fn_ptr: unsafe { std::mem::transmute(fn_ptr) },
        })
    }

    /// Compile a bi weight expression.
    ///
    /// The expression must evaluate to an i64.
    ///
    /// # Arguments
    /// * `expr` - Expression referencing `Field { param_idx: 0|1, field_idx }`
    pub fn compile_bi_weight(expr: &Expr) -> Result<CompiledBiWeight, JitError> {
        let (module, fn_ptr) = compile_function(expr, FnShape::BiToI64)?;
        Ok(CompiledBiWeight {
            _module: module,
            fn_ptr: unsafe { std::mem::transmute(fn_ptr) },
        })
    }
}

// ---------------------------------------------------------------------------
// Internal compilation machinery
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum FnShape {
    /// `fn(*const i64) -> i64`
    UniToI64,
    /// `fn(*const i64, *const i64) -> i8`
    BiToBool,
    /// `fn(*const i64, *const i64) -> i64`
    BiToI64,
}

fn make_jit_module() -> Result<JITModule, JitError> {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false")?;
    flag_builder.set("is_pic", "false")?;
    let isa_builder = cranelift_native::builder().map_err(|e| JitError::Codegen(e.to_string()))?;
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(|e| JitError::Codegen(e.to_string()))?;
    let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    let module = JITModule::new(builder);
    Ok(module)
}

fn build_signature(module: &JITModule, shape: FnShape) -> Signature {
    let ptr_type = module.target_config().pointer_type();
    let mut sig = Signature::new(CallConv::SystemV);
    match shape {
        FnShape::UniToI64 => {
            sig.params.push(AbiParam::new(ptr_type)); // entity_a
            sig.returns.push(AbiParam::new(I64));
        }
        FnShape::BiToBool => {
            sig.params.push(AbiParam::new(ptr_type)); // entity_a
            sig.params.push(AbiParam::new(ptr_type)); // entity_b
            sig.returns.push(AbiParam::new(I8));
        }
        FnShape::BiToI64 => {
            sig.params.push(AbiParam::new(ptr_type)); // entity_a
            sig.params.push(AbiParam::new(ptr_type)); // entity_b
            sig.returns.push(AbiParam::new(I64));
        }
    }
    sig
}

fn compile_function(expr: &Expr, shape: FnShape) -> Result<(JITModule, *const u8), JitError> {
    let mut module = make_jit_module()?;
    let sig = build_signature(&module, shape);

    let func_id = module.declare_function("jit_expr", Linkage::Local, &sig)?;

    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig.clone());
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut func, &mut func_ctx);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        // Entity pointer parameters
        let params: Vec<_> = builder.block_params(entry).to_vec();

        let result = emit_expr(&mut builder, expr, &params, shape)?;

        // Narrow to i8 for bool returns
        let ret = match shape {
            FnShape::BiToBool => {
                // result is i64 (0 or 1), truncate to i8
                builder.ins().ireduce(I8, result)
            }
            FnShape::UniToI64 | FnShape::BiToI64 => result,
        };

        builder.ins().return_(&[ret]);
        builder.finalize();
    }

    let mut ctx = Context::for_function(func);
    module
        .define_function(func_id, &mut ctx)
        .map_err(|e| JitError::Codegen(e.to_string()))?;
    module.clear_context(&mut ctx);
    module
        .finalize_definitions()
        .map_err(|e| JitError::Codegen(e.to_string()))?;

    let fn_ptr = module.get_finalized_function(func_id);

    Ok((module, fn_ptr))
}

/// Emit Cranelift IR for an expression, returning an i64 SSA value.
///
/// Boolean results are represented as i64 (0 or 1) internally, narrowed
/// to i8 only at the function return boundary.
fn emit_expr(
    builder: &mut FunctionBuilder,
    expr: &Expr,
    params: &[cranelift_codegen::ir::Value],
    shape: FnShape,
) -> Result<cranelift_codegen::ir::Value, JitError> {
    use Expr::*;
    match expr {
        // --- Leaf nodes ---
        Literal(crate::solution::DynamicValue::I64(n)) => Ok(builder.ins().iconst(I64, *n)),
        Literal(crate::solution::DynamicValue::Bool(b)) => Ok(builder.ins().iconst(I64, *b as i64)),
        Literal(crate::solution::DynamicValue::None) => {
            Ok(builder.ins().iconst(I64, NONE_SENTINEL))
        }

        // Field access: *(param_ptr + field_idx * 8)
        Field {
            param_idx,
            field_idx,
        } => {
            let ptr = get_param_ptr(params, *param_idx, shape)?;
            let offset = (*field_idx as i32) * 8;
            Ok(builder
                .ins()
                .load(I64, cranelift_codegen::ir::MemFlags::trusted(), ptr, offset))
        }

        // --- Comparison operators (return i64: 0 or 1) ---
        Eq(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            let cmp = builder.ins().icmp(IntCC::Equal, l, r);
            Ok(builder.ins().sextend(I64, cmp))
        }
        Ne(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            let cmp = builder.ins().icmp(IntCC::NotEqual, l, r);
            Ok(builder.ins().sextend(I64, cmp))
        }
        Lt(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            let cmp = builder.ins().icmp(IntCC::SignedLessThan, l, r);
            Ok(builder.ins().sextend(I64, cmp))
        }
        Le(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, l, r);
            Ok(builder.ins().sextend(I64, cmp))
        }
        Gt(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, l, r);
            Ok(builder.ins().sextend(I64, cmp))
        }
        Ge(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, l, r);
            Ok(builder.ins().sextend(I64, cmp))
        }

        // --- Logical operators ---
        And(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            Ok(builder.ins().band(l, r))
        }
        Or(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            Ok(builder.ins().bor(l, r))
        }
        Not(inner) => {
            let v = emit_expr(builder, inner, params, shape)?;
            let one = builder.ins().iconst(I64, 1);
            Ok(builder.ins().bxor(v, one))
        }

        // --- Arithmetic ---
        Add(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            Ok(builder.ins().iadd(l, r))
        }
        Sub(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            Ok(builder.ins().isub(l, r))
        }
        Mul(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            Ok(builder.ins().imul(l, r))
        }
        Neg(inner) => {
            let v = emit_expr(builder, inner, params, shape)?;
            Ok(builder.ins().ineg(v))
        }
        Abs(inner) => {
            let v = emit_expr(builder, inner, params, shape)?;
            let neg = builder.ins().ineg(v);
            let zero = builder.ins().iconst(I64, 0);
            let is_neg = builder.ins().icmp(IntCC::SignedLessThan, v, zero);
            Ok(builder.ins().select(is_neg, neg, v))
        }

        // --- Min / Max ---
        Min(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            let cmp = builder.ins().icmp(IntCC::SignedLessThan, l, r);
            Ok(builder.ins().select(cmp, l, r))
        }
        Max(left, right) => {
            let l = emit_expr(builder, left, params, shape)?;
            let r = emit_expr(builder, right, params, shape)?;
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, l, r);
            Ok(builder.ins().select(cmp, l, r))
        }

        // --- IsNone / IsNotNone (sentinel check) ---
        IsNone(inner) => {
            let v = emit_expr(builder, inner, params, shape)?;
            let sentinel = builder.ins().iconst(I64, NONE_SENTINEL);
            let cmp = builder.ins().icmp(IntCC::Equal, v, sentinel);
            Ok(builder.ins().sextend(I64, cmp))
        }
        IsNotNone(inner) => {
            let v = emit_expr(builder, inner, params, shape)?;
            let sentinel = builder.ins().iconst(I64, NONE_SENTINEL);
            let cmp = builder.ins().icmp(IntCC::NotEqual, v, sentinel);
            Ok(builder.ins().sextend(I64, cmp))
        }

        // --- Overlaps: max(s1, s2) < min(e1, e2) ---
        Overlaps {
            start1,
            end1,
            start2,
            end2,
        } => {
            let s1 = emit_expr(builder, start1, params, shape)?;
            let e1 = emit_expr(builder, end1, params, shape)?;
            let s2 = emit_expr(builder, start2, params, shape)?;
            let e2 = emit_expr(builder, end2, params, shape)?;
            // max(s1, s2)
            let cmp_s = builder.ins().icmp(IntCC::SignedGreaterThan, s1, s2);
            let max_start = builder.ins().select(cmp_s, s1, s2);
            // min(e1, e2)
            let cmp_e = builder.ins().icmp(IntCC::SignedLessThan, e1, e2);
            let min_end = builder.ins().select(cmp_e, e1, e2);
            // overlaps = max_start < min_end
            let cmp = builder
                .ins()
                .icmp(IntCC::SignedLessThan, max_start, min_end);
            Ok(builder.ins().sextend(I64, cmp))
        }

        // --- OverlapMinutes: max(0, min(e1,e2) - max(s1,s2)) / 60000 ---
        OverlapMinutes {
            start1,
            end1,
            start2,
            end2,
        } => {
            let s1 = emit_expr(builder, start1, params, shape)?;
            let e1 = emit_expr(builder, end1, params, shape)?;
            let s2 = emit_expr(builder, start2, params, shape)?;
            let e2 = emit_expr(builder, end2, params, shape)?;
            let cmp_s = builder.ins().icmp(IntCC::SignedGreaterThan, s1, s2);
            let max_start = builder.ins().select(cmp_s, s1, s2);
            let cmp_e = builder.ins().icmp(IntCC::SignedLessThan, e1, e2);
            let min_end = builder.ins().select(cmp_e, e1, e2);
            let diff = builder.ins().isub(min_end, max_start);
            let zero = builder.ins().iconst(I64, 0);
            let is_pos = builder.ins().icmp(IntCC::SignedGreaterThan, diff, zero);
            let clamped = builder.ins().select(is_pos, diff, zero);
            // milliseconds to minutes
            let ms_per_min = builder.ins().iconst(I64, 60_000);
            Ok(builder.ins().sdiv(clamped, ms_per_min))
        }

        // --- If/then/else ---
        If {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = emit_expr(builder, cond, params, shape)?;
            let t = emit_expr(builder, then_expr, params, shape)?;
            let e = emit_expr(builder, else_expr, params, shape)?;
            let zero = builder.ins().iconst(I64, 0);
            let is_true = builder.ins().icmp(IntCC::NotEqual, c, zero);
            Ok(builder.ins().select(is_true, t, e))
        }

        // --- Unsupported: fall back to interpreter ---
        Literal(_) => Err(JitError::Unsupported(
            "Non-i64/bool/None literal".to_string(),
        )),
        Param(_) => Err(JitError::Unsupported("Param reference".to_string())),
        Div(..) => Err(JitError::Unsupported(
            "Division (requires zero-check)".to_string(),
        )),
        Mod(..) => Err(JitError::Unsupported(
            "Modulo (requires zero-check)".to_string(),
        )),
        Contains(..) => Err(JitError::Unsupported("Contains (list op)".to_string())),
        RefField { .. } => Err(JitError::Unsupported(
            "RefField (indirect lookup)".to_string(),
        )),
        SetContains { .. } => Err(JitError::Unsupported("SetContains".to_string())),
        DateOf(_) => Err(JitError::Unsupported("DateOf".to_string())),
        OverlapsDate { .. } => Err(JitError::Unsupported("OverlapsDate".to_string())),
        OverlapDateMinutes { .. } => Err(JitError::Unsupported("OverlapDateMinutes".to_string())),
        FlattenedValue => Err(JitError::Unsupported("FlattenedValue".to_string())),
    }
}

/// Resolve a param_idx to the corresponding entity pointer from function params.
fn get_param_ptr(
    params: &[cranelift_codegen::ir::Value],
    param_idx: usize,
    shape: FnShape,
) -> Result<cranelift_codegen::ir::Value, JitError> {
    match shape {
        FnShape::UniToI64 => {
            if param_idx == 0 {
                Ok(params[0])
            } else {
                Err(JitError::Unsupported(format!(
                    "Uni function cannot reference param_idx={}",
                    param_idx
                )))
            }
        }
        FnShape::BiToBool | FnShape::BiToI64 => {
            if param_idx < 2 {
                Ok(params[param_idx])
            } else {
                Err(JitError::Unsupported(format!(
                    "Bi function cannot reference param_idx={}",
                    param_idx
                )))
            }
        }
    }
}
