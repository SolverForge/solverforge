mod codegen;
mod compiler;
mod predicate;
#[cfg(test)]
mod tests;

pub use predicate::PredicateDefinition;

use crate::domain::DomainModel;
use crate::error::{SolverForgeError, SolverForgeResult};
use crate::wasm::host_functions::{HostFunctionRegistry, WasmType};
use crate::wasm::memory::{LayoutCalculator, WasmMemoryType};
use base64::{engine::general_purpose::STANDARD, Engine};
use compiler::{ExpressionCompiler, LocalAllocator};
use indexmap::IndexMap;
use std::cell::RefCell;
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function, FunctionSection,
    GlobalSection, GlobalType, Instruction, MemorySection, MemoryType, Module, TypeSection,
    ValType,
};

/// Base offset for string constants in memory (64KB into memory)
const STRING_CONSTANTS_BASE: u32 = 65536;

pub struct WasmModuleBuilder {
    layout_calculator: LayoutCalculator,
    domain_model: Option<DomainModel>,
    predicates: IndexMap<String, PredicateDefinition>,
    function_types: Vec<FunctionSignature>,
    initial_memory_pages: u32,
    max_memory_pages: Option<u32>,
    host_functions: HostFunctionRegistry,
    host_function_indices: IndexMap<String, u32>,
    /// String constants collected during expression compilation.
    string_constants: RefCell<IndexMap<String, u32>>,
    /// Current offset for next string constant.
    string_constants_offset: RefCell<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FunctionSignature {
    params: Vec<ValType>,
    results: Vec<ValType>,
}

impl Default for WasmModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmModuleBuilder {
    pub fn new() -> Self {
        Self {
            layout_calculator: LayoutCalculator::new(),
            domain_model: None,
            predicates: IndexMap::new(),
            function_types: Vec::new(),
            initial_memory_pages: 16,
            max_memory_pages: Some(256),
            host_functions: HostFunctionRegistry::new(),
            host_function_indices: IndexMap::new(),
            string_constants: RefCell::new(IndexMap::new()),
            string_constants_offset: RefCell::new(0),
        }
    }

    pub fn with_host_functions(mut self, registry: HostFunctionRegistry) -> Self {
        self.host_functions = registry;
        self
    }

    pub fn with_domain_model(mut self, model: DomainModel) -> Self {
        for class in model.classes.values() {
            self.layout_calculator.calculate_layout(class);
        }
        self.domain_model = Some(model);
        self
    }

    pub fn with_initial_memory(mut self, pages: u32) -> Self {
        self.initial_memory_pages = pages;
        self
    }

    pub fn with_max_memory(mut self, pages: Option<u32>) -> Self {
        self.max_memory_pages = pages;
        self
    }

    pub fn add_predicate(mut self, predicate: PredicateDefinition) -> Self {
        self.predicates.insert(predicate.name.clone(), predicate);
        self
    }

    /// Get or allocate an offset for a string constant.
    fn get_string_constant_offset(&self, s: &str) -> u32 {
        let mut constants = self.string_constants.borrow_mut();
        if let Some(&offset) = constants.get(s) {
            return STRING_CONSTANTS_BASE + offset;
        }

        let mut current_offset = self.string_constants_offset.borrow_mut();
        let offset = *current_offset;

        let string_size = 4 + s.len() as u32;
        let aligned_size = (string_size + 3) & !3;

        constants.insert(s.to_string(), offset);
        *current_offset = offset + aligned_size;

        STRING_CONSTANTS_BASE + offset
    }

    /// Build the data section containing all string constants.
    fn build_string_data_section(&self) -> Option<DataSection> {
        let constants = self.string_constants.borrow();
        if constants.is_empty() {
            return None;
        }

        let total_size = *self.string_constants_offset.borrow();
        let mut data = vec![0u8; total_size as usize];

        for (s, &offset) in constants.iter() {
            let bytes = s.as_bytes();
            let len = bytes.len() as u32;

            let offset = offset as usize;
            data[offset..offset + 4].copy_from_slice(&len.to_le_bytes());
            data[offset + 4..offset + 4 + bytes.len()].copy_from_slice(bytes);
        }

        let mut data_section = DataSection::new();
        data_section.active(
            0,
            &ConstExpr::i32_const(STRING_CONSTANTS_BASE as i32),
            data.iter().copied(),
        );

        Some(data_section)
    }

    pub fn build(mut self) -> SolverForgeResult<Vec<u8>> {
        let model = self
            .domain_model
            .take()
            .ok_or_else(|| SolverForgeError::WasmGeneration("Domain model not set".to_string()))?;

        let mut module = Module::new();

        let mut type_section = TypeSection::new();
        let mut import_section = wasm_encoder::ImportSection::new();
        let mut function_section = FunctionSection::new();
        let mut code_section = CodeSection::new();
        let mut export_section = ExportSection::new();
        let mut memory_section = MemorySection::new();
        let mut global_section = GlobalSection::new();

        memory_section.memory(MemoryType {
            minimum: self.initial_memory_pages as u64,
            maximum: self.max_memory_pages.map(|p| p as u64),
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        global_section.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(1024),
        );

        // Generate host function imports
        let mut func_idx: u32 = 0;
        let func_names: Vec<String> = self
            .host_functions
            .function_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        for func_name in func_names {
            if let Some(host_func) = self.host_functions.lookup(&func_name).cloned() {
                let params: Vec<ValType> = host_func
                    .params
                    .iter()
                    .map(|t| wasm_type_to_val_type(*t))
                    .collect();
                let results: Vec<ValType> = if matches!(host_func.return_type, WasmType::Void) {
                    vec![]
                } else {
                    vec![wasm_type_to_val_type(host_func.return_type)]
                };

                let type_idx = self.add_function_type(&mut type_section, params, results);

                import_section.import(
                    "host",
                    &host_func.name,
                    wasm_encoder::EntityType::Function(type_idx),
                );

                self.host_function_indices
                    .insert(func_name.clone(), func_idx);
                func_idx += 1;
            }
        }

        // alloc(size: i32) -> i32
        let alloc_type_idx =
            self.add_function_type(&mut type_section, vec![ValType::I32], vec![ValType::I32]);
        function_section.function(alloc_type_idx);
        code_section.function(&codegen::generate_allocator());
        export_section.export("alloc", ExportKind::Func, func_idx);
        func_idx += 1;

        // dealloc(ptr: i32)
        let dealloc_type_idx =
            self.add_function_type(&mut type_section, vec![ValType::I32], vec![]);
        function_section.function(dealloc_type_idx);
        code_section.function(&codegen::generate_deallocator());
        export_section.export("dealloc", ExportKind::Func, func_idx);
        func_idx += 1;

        // Generate getter/setter functions for each class field
        for class in model.classes.values() {
            let layout = self.layout_calculator.calculate_layout(class);

            for field in &class.fields {
                if let Some(field_layout) = layout.field_offsets.get(&field.name) {
                    // Getter
                    let getter_name = format!("get_{}_{}", class.name, field.name);
                    let result_type = wasm_memory_type_to_val_type(field_layout.wasm_type);

                    let getter_type_idx = self.add_function_type(
                        &mut type_section,
                        vec![ValType::I32],
                        vec![result_type],
                    );
                    function_section.function(getter_type_idx);
                    code_section.function(&codegen::generate_getter(
                        field_layout.offset,
                        field_layout.wasm_type,
                    ));
                    export_section.export(&getter_name, ExportKind::Func, func_idx);
                    func_idx += 1;

                    // Setter
                    let setter_name = format!("set_{}_{}", class.name, field.name);
                    let setter_type_idx = self.add_function_type(
                        &mut type_section,
                        vec![ValType::I32, result_type],
                        vec![],
                    );
                    function_section.function(setter_type_idx);
                    code_section.function(&codegen::generate_setter(
                        field_layout.offset,
                        field_layout.wasm_type,
                    ));
                    export_section.export(&setter_name, ExportKind::Func, func_idx);
                    func_idx += 1;
                }
            }
        }

        // Generate predicate functions
        let predicates: Vec<_> = self
            .predicates
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (name, predicate) in &predicates {
            let params: Vec<ValType> = predicate
                .param_types
                .clone()
                .unwrap_or_else(|| (0..predicate.arity).map(|_| ValType::I32).collect());
            let pred_type_idx =
                self.add_function_type(&mut type_section, params, vec![ValType::I32]);
            function_section.function(pred_type_idx);
            code_section.function(&self.generate_predicate(predicate, &model)?);
            export_section.export(name, ExportKind::Func, func_idx);
            func_idx += 1;
        }

        // Solution mapper wrapper functions
        for gen_func in codegen::generate_solution_mappers(&self.host_function_indices) {
            let type_idx =
                self.add_function_type(&mut type_section, gen_func.params, gen_func.results);
            function_section.function(type_idx);
            code_section.function(&gen_func.body);
            export_section.export(&gen_func.name, ExportKind::Func, func_idx);
            func_idx += 1;
        }

        // List accessor functions
        for gen_func in codegen::generate_list_accessors(&self.host_function_indices) {
            let type_idx =
                self.add_function_type(&mut type_section, gen_func.params, gen_func.results);
            function_section.function(type_idx);
            code_section.function(&gen_func.body);
            export_section.export(&gen_func.name, ExportKind::Func, func_idx);
            func_idx += 1;
        }

        export_section.export("memory", ExportKind::Memory, 0);

        module.section(&type_section);
        if !self.host_function_indices.is_empty() {
            module.section(&import_section);
        }
        module.section(&function_section);
        module.section(&memory_section);
        module.section(&global_section);
        module.section(&export_section);
        module.section(&code_section);

        if let Some(data_section) = self.build_string_data_section() {
            module.section(&data_section);
        }

        Ok(module.finish())
    }

    pub fn build_base64(self) -> SolverForgeResult<String> {
        let bytes = self.build()?;
        Ok(STANDARD.encode(&bytes))
    }

    fn add_function_type(
        &mut self,
        type_section: &mut TypeSection,
        params: Vec<ValType>,
        results: Vec<ValType>,
    ) -> u32 {
        let sig = FunctionSignature {
            params: params.clone(),
            results: results.clone(),
        };

        for (idx, existing) in self.function_types.iter().enumerate() {
            if *existing == sig {
                return idx as u32;
            }
        }

        let idx = self.function_types.len() as u32;
        type_section.ty().function(params, results);
        self.function_types.push(sig);
        idx
    }

    fn generate_predicate(
        &self,
        predicate: &PredicateDefinition,
        model: &DomainModel,
    ) -> SolverForgeResult<Function> {
        let mut func = Function::new([(12, ValType::I32)]);

        let mut locals = LocalAllocator::new(predicate.arity);

        let compiler = ExpressionCompiler {
            layout_calculator: &self.layout_calculator,
            host_function_indices: &self.host_function_indices,
            get_string_constant_offset: &|s: &str| self.get_string_constant_offset(s),
        };

        compiler.compile_expression(
            &mut func,
            &predicate.body,
            model,
            u32::MAX,
            u32::MAX,
            &mut locals,
        )?;

        func.instruction(&Instruction::End);
        Ok(func)
    }
}

fn wasm_type_to_val_type(wasm_type: WasmType) -> ValType {
    match wasm_type {
        WasmType::I32 | WasmType::Ptr => ValType::I32,
        WasmType::I64 => ValType::I64,
        WasmType::F32 => ValType::F32,
        WasmType::F64 => ValType::F64,
        WasmType::Void => panic!("Void type cannot be converted to ValType"),
    }
}

fn wasm_memory_type_to_val_type(memory_type: WasmMemoryType) -> ValType {
    match memory_type {
        WasmMemoryType::I32 | WasmMemoryType::Pointer | WasmMemoryType::ArrayPointer => {
            ValType::I32
        }
        WasmMemoryType::I64 => ValType::I64,
        WasmMemoryType::F32 => ValType::F32,
        WasmMemoryType::F64 => ValType::F64,
    }
}
