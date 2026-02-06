//! Cranelift JIT compiler for Expr trees — Generalized N-ary codegen.
//!
//! Compiles constraint expressions into native function pointers operating on
//! flat `*const i64` entity buffers. Each entity field occupies one i64 slot.
//!
//! # Calling Convention
//!
//! All compiled functions have a **single parameter**: a pointer to an array of
//! entity base pointers (`*const *const i64`). Field access compiles to:
//!
//! ```text
//! entity_ptr = load(base + param_idx * 8)      // indirect: load entity pointer from array
//! value      = load(entity_ptr + field_idx * 8) // direct: load field from flat buffer
//! ```
//!
//! Two loads per field access. No HashMap. No bounds checks. The pointer array
//! is built on the stack at each call site.
//!
//! # Zero-Fallback Policy
//!
//! If compilation fails for ANY reason, the process panics. There is no
//! interpreter fallback, no `Result` return, no `Unsupported` variant.
//! Every `Expr` that reaches this compiler must be compilable.

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::types::I64;
use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, MemFlags, Signature, UserFuncName};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

use crate::expr::Expr;
use crate::NONE_SENTINEL;

/// A JIT-compiled function. Owns the Cranelift module (code memory) and holds
/// the raw function pointer.
///
/// All compiled functions use the indirect pointer-array calling convention:
///   `fn(*const *const i64) -> i64`
///
/// Convenience methods `call_1`, `call_2` build a stack array and delegate to `call_n`.
pub struct JitFn {
    _module: JITModule,
    ptr: *const u8,
    arity: u8,
}

// SAFETY: JITModule owns the code memory. ptr is valid for the module's lifetime.
unsafe impl Send for JitFn {}
unsafe impl Sync for JitFn {}

impl JitFn {
    /// Generalized N-ary call. `ptrs[i]` is `flat_entity_ptr` for param `i`.
    ///
    /// The compiled function receives `ptrs.as_ptr()` — a `*const *const i64`.
    /// Inside the native code, field access does:
    ///   `entity_ptr = load(base + param_idx * 8)`
    ///   `value = load(entity_ptr + field_idx * 8)`
    #[inline]
    pub fn call_n(&self, ptrs: &[*const i64]) -> i64 {
        debug_assert!(
            ptrs.len() >= self.arity as usize,
            "call_n: expected {} ptrs, got {}",
            self.arity,
            ptrs.len()
        );
        let f: unsafe fn(*const *const i64) -> i64 = unsafe { std::mem::transmute(self.ptr) };
        unsafe { f(ptrs.as_ptr()) }
    }

    /// Convenience: 1-param call (uni key/filter/weight).
    #[inline]
    pub fn call_1(&self, a: *const i64) -> i64 {
        debug_assert_eq!(self.arity, 1);
        let ptrs = [a];
        let f: unsafe fn(*const *const i64) -> i64 = unsafe { std::mem::transmute(self.ptr) };
        unsafe { f(ptrs.as_ptr()) }
    }

    /// Convenience: 2-param call (bi key/filter/weight).
    #[inline]
    pub fn call_2(&self, a: *const i64, b: *const i64) -> i64 {
        debug_assert_eq!(self.arity, 2);
        let ptrs = [a, b];
        let f: unsafe fn(*const *const i64) -> i64 = unsafe { std::mem::transmute(self.ptr) };
        unsafe { f(ptrs.as_ptr()) }
    }

    pub fn arity(&self) -> u8 {
        self.arity
    }
}

// ---------------------------------------------------------------------------
// Public API — zero fallback, panics on failure
// ---------------------------------------------------------------------------

/// Compile an expression for `arity` entity parameters.
///
/// The generated native function takes a single `*const *const i64` (pointer to
/// array of entity base pointers) and returns `i64`.
///
/// # Panics
///
/// Panics if the expression contains unsupported variants or if Cranelift
/// codegen fails. Zero fallback — there is no interpreter path.
pub fn compile_n(expr: &Expr, arity: usize) -> JitFn {
    let (module, ptr) = compile_function(expr, arity)
        .unwrap_or_else(|e| panic!("JIT compile_n(arity={arity}) failed: {e}"));
    JitFn {
        _module: module,
        ptr,
        arity: arity as u8,
    }
}

/// Compile with 1 entity param. Thin wrapper around `compile_n`.
#[inline]
pub fn compile_1(expr: &Expr) -> JitFn {
    compile_n(expr, 1)
}

/// Compile with 2 entity params. Thin wrapper around `compile_n`.
#[inline]
pub fn compile_2(expr: &Expr) -> JitFn {
    compile_n(expr, 2)
}

// ---------------------------------------------------------------------------
// Internal codegen
// ---------------------------------------------------------------------------

fn make_jit_module() -> JITModule {
    let mut flag_builder = settings::builder();
    flag_builder
        .set("use_colocated_libcalls", "false")
        .expect("cranelift setting");
    flag_builder
        .set("is_pic", "false")
        .expect("cranelift setting");
    let isa_builder =
        cranelift_native::builder().unwrap_or_else(|e| panic!("cranelift ISA builder: {e}"));
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap_or_else(|e| panic!("cranelift ISA finish: {e}"));
    let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    JITModule::new(builder)
}

/// Internal error type — only used within compile_function, then unwrapped with panic.
#[derive(Debug)]
enum CodegenError {
    Module(cranelift_module::ModuleError),
    Codegen(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::Module(e) => write!(f, "module: {e}"),
            CodegenError::Codegen(s) => write!(f, "codegen: {s}"),
        }
    }
}

impl From<cranelift_module::ModuleError> for CodegenError {
    fn from(e: cranelift_module::ModuleError) -> Self {
        CodegenError::Module(e)
    }
}

#[allow(clippy::result_large_err)]
fn compile_function(expr: &Expr, arity: usize) -> Result<(JITModule, *const u8), CodegenError> {
    let mut module = make_jit_module();
    let ptr_type = module.target_config().pointer_type();

    // Single parameter: *const *const i64 (pointer to array of entity base pointers)
    let mut sig = Signature::new(CallConv::SystemV);
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(I64));

    let func_id = module.declare_function("jit_expr", Linkage::Local, &sig)?;
    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig);
    let mut func_ctx = FunctionBuilderContext::new();

    {
        let mut builder = FunctionBuilder::new(&mut func, &mut func_ctx);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        // v0 = base pointer (*const *const i64)
        let base_ptr = builder.block_params(entry)[0];

        // Pre-load all entity pointers from the base array.
        // entity_ptrs[i] = load(base_ptr + i * 8)
        let mut entity_ptrs = Vec::with_capacity(arity);
        let flags = MemFlags::trusted();
        for i in 0..arity {
            let offset = (i as i32) * 8;
            let entity_ptr = builder.ins().load(ptr_type, flags, base_ptr, offset);
            entity_ptrs.push(entity_ptr);
        }

        let result = emit_expr(&mut builder, expr, &entity_ptrs, arity);
        builder.ins().return_(&[result]);
        builder.finalize();
    }

    let mut ctx = Context::for_function(func);
    module
        .define_function(func_id, &mut ctx)
        .map_err(|e| CodegenError::Codegen(e.to_string()))?;
    module.clear_context(&mut ctx);
    module
        .finalize_definitions()
        .map_err(|e| CodegenError::Codegen(e.to_string()))?;

    let ptr = module.get_finalized_function(func_id);
    Ok((module, ptr))
}

/// Emit Cranelift IR for an expression. All values are i64 internally.
/// Booleans are 0 or 1 as i64.
///
/// `entity_ptrs` contains pre-loaded entity base pointers (one per param_idx).
/// Field access is a single load from the entity pointer:
///   `value = load(entity_ptrs[param_idx] + field_idx * 8)`
///
/// # Panics
///
/// Panics on unsupported Expr variants. Zero fallback.
fn emit_expr(
    builder: &mut FunctionBuilder,
    expr: &Expr,
    entity_ptrs: &[cranelift_codegen::ir::Value],
    arity: usize,
) -> cranelift_codegen::ir::Value {
    use Expr::*;
    match expr {
        Literal(crate::solution::DynamicValue::I64(n)) => builder.ins().iconst(I64, *n),
        Literal(crate::solution::DynamicValue::Bool(b)) => builder.ins().iconst(I64, *b as i64),
        Literal(crate::solution::DynamicValue::None) => builder.ins().iconst(I64, NONE_SENTINEL),

        Field {
            param_idx,
            field_idx,
        } => {
            assert!(
                (*param_idx) < arity,
                "JIT Field: param_idx={} but arity={}",
                param_idx,
                arity
            );
            // entity_ptrs[param_idx] was pre-loaded from base_ptr + param_idx * 8.
            // Now load the field: load(entity_ptr + field_idx * 8)
            let entity_ptr = entity_ptrs[*param_idx];
            let offset = (*field_idx as i32) * 8;
            builder
                .ins()
                .load(I64, MemFlags::trusted(), entity_ptr, offset)
        }

        Eq(l, r) => icmp_op(builder, IntCC::Equal, l, r, entity_ptrs, arity),
        Ne(l, r) => icmp_op(builder, IntCC::NotEqual, l, r, entity_ptrs, arity),
        Lt(l, r) => icmp_op(builder, IntCC::SignedLessThan, l, r, entity_ptrs, arity),
        Le(l, r) => icmp_op(
            builder,
            IntCC::SignedLessThanOrEqual,
            l,
            r,
            entity_ptrs,
            arity,
        ),
        Gt(l, r) => icmp_op(builder, IntCC::SignedGreaterThan, l, r, entity_ptrs, arity),
        Ge(l, r) => icmp_op(
            builder,
            IntCC::SignedGreaterThanOrEqual,
            l,
            r,
            entity_ptrs,
            arity,
        ),

        And(l, r) => bin_op(
            builder,
            |b, a, c| b.ins().band(a, c),
            l,
            r,
            entity_ptrs,
            arity,
        ),
        Or(l, r) => bin_op(
            builder,
            |b, a, c| b.ins().bor(a, c),
            l,
            r,
            entity_ptrs,
            arity,
        ),
        Not(inner) => {
            let v = emit_expr(builder, inner, entity_ptrs, arity);
            let one = builder.ins().iconst(I64, 1);
            builder.ins().bxor(v, one)
        }

        Add(l, r) => bin_op(
            builder,
            |b, a, c| b.ins().iadd(a, c),
            l,
            r,
            entity_ptrs,
            arity,
        ),
        Sub(l, r) => bin_op(
            builder,
            |b, a, c| b.ins().isub(a, c),
            l,
            r,
            entity_ptrs,
            arity,
        ),
        Mul(l, r) => bin_op(
            builder,
            |b, a, c| b.ins().imul(a, c),
            l,
            r,
            entity_ptrs,
            arity,
        ),
        Neg(inner) => {
            let v = emit_expr(builder, inner, entity_ptrs, arity);
            builder.ins().ineg(v)
        }
        Abs(inner) => {
            let v = emit_expr(builder, inner, entity_ptrs, arity);
            let neg = builder.ins().ineg(v);
            let zero = builder.ins().iconst(I64, 0);
            let is_neg = builder.ins().icmp(IntCC::SignedLessThan, v, zero);
            builder.ins().select(is_neg, neg, v)
        }

        Min(l, r) => select_op(builder, IntCC::SignedLessThan, l, r, entity_ptrs, arity),
        Max(l, r) => select_op(builder, IntCC::SignedGreaterThan, l, r, entity_ptrs, arity),

        IsNone(inner) => {
            let v = emit_expr(builder, inner, entity_ptrs, arity);
            let sentinel = builder.ins().iconst(I64, NONE_SENTINEL);
            let cmp = builder.ins().icmp(IntCC::Equal, v, sentinel);
            builder.ins().sextend(I64, cmp)
        }
        IsNotNone(inner) => {
            let v = emit_expr(builder, inner, entity_ptrs, arity);
            let sentinel = builder.ins().iconst(I64, NONE_SENTINEL);
            let cmp = builder.ins().icmp(IntCC::NotEqual, v, sentinel);
            builder.ins().sextend(I64, cmp)
        }

        Overlaps {
            start1,
            end1,
            start2,
            end2,
        } => {
            let s1 = emit_expr(builder, start1, entity_ptrs, arity);
            let e1 = emit_expr(builder, end1, entity_ptrs, arity);
            let s2 = emit_expr(builder, start2, entity_ptrs, arity);
            let e2 = emit_expr(builder, end2, entity_ptrs, arity);
            let cmp_s = builder.ins().icmp(IntCC::SignedGreaterThan, s1, s2);
            let max_start = builder.ins().select(cmp_s, s1, s2);
            let cmp_e = builder.ins().icmp(IntCC::SignedLessThan, e1, e2);
            let min_end = builder.ins().select(cmp_e, e1, e2);
            let cmp = builder
                .ins()
                .icmp(IntCC::SignedLessThan, max_start, min_end);
            builder.ins().sextend(I64, cmp)
        }

        OverlapMinutes {
            start1,
            end1,
            start2,
            end2,
        } => {
            let s1 = emit_expr(builder, start1, entity_ptrs, arity);
            let e1 = emit_expr(builder, end1, entity_ptrs, arity);
            let s2 = emit_expr(builder, start2, entity_ptrs, arity);
            let e2 = emit_expr(builder, end2, entity_ptrs, arity);
            let cmp_s = builder.ins().icmp(IntCC::SignedGreaterThan, s1, s2);
            let max_start = builder.ins().select(cmp_s, s1, s2);
            let cmp_e = builder.ins().icmp(IntCC::SignedLessThan, e1, e2);
            let min_end = builder.ins().select(cmp_e, e1, e2);
            let diff = builder.ins().isub(min_end, max_start);
            let zero = builder.ins().iconst(I64, 0);
            let is_pos = builder.ins().icmp(IntCC::SignedGreaterThan, diff, zero);
            let clamped = builder.ins().select(is_pos, diff, zero);
            let ms_per_min = builder.ins().iconst(I64, 60_000);
            builder.ins().sdiv(clamped, ms_per_min)
        }

        If {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = emit_expr(builder, cond, entity_ptrs, arity);
            let t = emit_expr(builder, then_expr, entity_ptrs, arity);
            let e = emit_expr(builder, else_expr, entity_ptrs, arity);
            let zero = builder.ins().iconst(I64, 0);
            let is_true = builder.ins().icmp(IntCC::NotEqual, c, zero);
            builder.ins().select(is_true, t, e)
        }

        // Zero fallback: unsupported variants panic immediately.
        Literal(v) => panic!("JIT: unsupported literal variant: {v:?}"),
        Param(p) => panic!("JIT: Param({p}) not supported in compiled expressions"),
        Div(..) => panic!("JIT: Div not yet implemented"),
        Mod(..) => panic!("JIT: Mod not yet implemented"),
        Contains(..) => panic!("JIT: Contains requires runtime set lookup — not compilable"),
        RefField { .. } => panic!("JIT: RefField requires runtime indirection — not compilable"),
        SetContains { .. } => panic!("JIT: SetContains requires runtime set — not compilable"),
        DateOf(_) => panic!("JIT: DateOf requires runtime date conversion — not compilable"),
        OverlapsDate { .. } => panic!("JIT: OverlapsDate requires runtime date — not compilable"),
        OverlapDateMinutes { .. } => {
            panic!("JIT: OverlapDateMinutes requires runtime date — not compilable")
        }
        FlattenedValue => panic!("JIT: FlattenedValue not yet implemented"),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn icmp_op(
    builder: &mut FunctionBuilder,
    cc: IntCC,
    left: &Expr,
    right: &Expr,
    entity_ptrs: &[cranelift_codegen::ir::Value],
    arity: usize,
) -> cranelift_codegen::ir::Value {
    let l = emit_expr(builder, left, entity_ptrs, arity);
    let r = emit_expr(builder, right, entity_ptrs, arity);
    let cmp = builder.ins().icmp(cc, l, r);
    builder.ins().sextend(I64, cmp)
}

fn bin_op(
    builder: &mut FunctionBuilder,
    op: impl FnOnce(
        &mut FunctionBuilder,
        cranelift_codegen::ir::Value,
        cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value,
    left: &Expr,
    right: &Expr,
    entity_ptrs: &[cranelift_codegen::ir::Value],
    arity: usize,
) -> cranelift_codegen::ir::Value {
    let l = emit_expr(builder, left, entity_ptrs, arity);
    let r = emit_expr(builder, right, entity_ptrs, arity);
    op(builder, l, r)
}

fn select_op(
    builder: &mut FunctionBuilder,
    cc: IntCC,
    left: &Expr,
    right: &Expr,
    entity_ptrs: &[cranelift_codegen::ir::Value],
    arity: usize,
) -> cranelift_codegen::ir::Value {
    let l = emit_expr(builder, left, entity_ptrs, arity);
    let r = emit_expr(builder, right, entity_ptrs, arity);
    let cmp = builder.ins().icmp(cc, l, r);
    builder.ins().select(cmp, l, r)
}
