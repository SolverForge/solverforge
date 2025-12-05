use crate::domain::{DomainClass, FieldType, PrimitiveType};
use indexmap::IndexMap;

#[derive(Debug, Clone)]
pub struct MemoryLayout {
    pub total_size: u32,
    pub alignment: u32,
    pub field_offsets: IndexMap<String, FieldLayout>,
}

#[derive(Debug, Clone)]
pub struct FieldLayout {
    pub offset: u32,
    pub size: u32,
    pub wasm_type: WasmMemoryType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmMemoryType {
    I32,
    I64,
    F32,
    F64,
    Pointer,
    ArrayPointer,
}

impl WasmMemoryType {
    pub fn size(&self) -> u32 {
        match self {
            WasmMemoryType::I32 => 4,
            WasmMemoryType::I64 => 8,
            WasmMemoryType::F32 => 4,
            WasmMemoryType::F64 => 8,
            WasmMemoryType::Pointer => 4,
            // Arrays are host-managed references (pointers) in WASM32 - 4 bytes
            WasmMemoryType::ArrayPointer => 4,
        }
    }

    pub fn alignment(&self) -> u32 {
        match self {
            WasmMemoryType::I32 => 4,
            WasmMemoryType::I64 => 8,
            WasmMemoryType::F32 => 4,
            WasmMemoryType::F64 => 8,
            WasmMemoryType::Pointer => 4,
            WasmMemoryType::ArrayPointer => 4,
        }
    }
}

#[derive(Debug, Default)]
pub struct LayoutCalculator {
    class_layouts: IndexMap<String, MemoryLayout>,
}

impl LayoutCalculator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calculate_layout(&mut self, class: &DomainClass) -> MemoryLayout {
        if let Some(layout) = self.class_layouts.get(&class.name) {
            return layout.clone();
        }

        let mut field_offsets = IndexMap::new();
        let mut current_offset: u32 = 0;
        let mut max_alignment: u32 = 4;

        for field in &class.fields {
            let wasm_type = field_type_to_wasm(&field.field_type);
            let field_alignment = wasm_type.alignment();
            let field_size = wasm_type.size();

            if !current_offset.is_multiple_of(field_alignment) {
                current_offset = (current_offset / field_alignment + 1) * field_alignment;
            }

            field_offsets.insert(
                field.name.clone(),
                FieldLayout {
                    offset: current_offset,
                    size: field_size,
                    wasm_type,
                },
            );

            current_offset += field_size;
            max_alignment = max_alignment.max(field_alignment);
        }

        if !current_offset.is_multiple_of(max_alignment) {
            current_offset = (current_offset / max_alignment + 1) * max_alignment;
        }

        let layout = MemoryLayout {
            total_size: current_offset.max(4),
            field_offsets,
            alignment: max_alignment,
        };

        self.class_layouts
            .insert(class.name.clone(), layout.clone());
        layout
    }

    pub fn get_layout(&self, class_name: &str) -> Option<&MemoryLayout> {
        self.class_layouts.get(class_name)
    }
}

fn field_type_to_wasm(field_type: &FieldType) -> WasmMemoryType {
    match field_type {
        FieldType::Primitive(prim) => match prim {
            PrimitiveType::Bool => WasmMemoryType::I32,
            PrimitiveType::Int => WasmMemoryType::I32,
            PrimitiveType::Long => WasmMemoryType::I64,
            PrimitiveType::Float => WasmMemoryType::F32,
            PrimitiveType::Double => WasmMemoryType::F64,
            PrimitiveType::String => WasmMemoryType::Pointer,
            PrimitiveType::Date => WasmMemoryType::I64, // Epoch day as i64
            PrimitiveType::DateTime => WasmMemoryType::I64, // Epoch second as i64
        },
        FieldType::Object { .. } => WasmMemoryType::Pointer,
        FieldType::Array { .. } | FieldType::List { .. } | FieldType::Set { .. } => {
            WasmMemoryType::ArrayPointer
        }
        FieldType::Map { .. } => WasmMemoryType::Pointer,
        FieldType::Score(_) => WasmMemoryType::Pointer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FieldDescriptor, PlanningAnnotation};

    fn create_test_class() -> DomainClass {
        DomainClass::new("Lesson")
            .with_annotation(PlanningAnnotation::PlanningEntity)
            .with_field(FieldDescriptor::new(
                "id",
                FieldType::Primitive(PrimitiveType::Long),
            ))
            .with_field(FieldDescriptor::new("room", FieldType::object("Room")))
            .with_field(FieldDescriptor::new(
                "timeslot",
                FieldType::object("Timeslot"),
            ))
    }

    #[test]
    fn test_layout_calculation() {
        let mut calc = LayoutCalculator::new();
        let class = create_test_class();
        let layout = calc.calculate_layout(&class);

        // id (i64) + room (pointer) + timeslot (pointer) = 8 + 4 + 4 = 16 bytes
        assert!(layout.total_size >= 16);
        assert!(layout.field_offsets.contains_key("id"));
        assert!(layout.field_offsets.contains_key("room"));
        assert!(layout.field_offsets.contains_key("timeslot"));
    }

    #[test]
    fn test_field_type_mapping() {
        assert_eq!(
            field_type_to_wasm(&FieldType::Primitive(PrimitiveType::Int)),
            WasmMemoryType::I32
        );
        assert_eq!(
            field_type_to_wasm(&FieldType::Primitive(PrimitiveType::Long)),
            WasmMemoryType::I64
        );
        assert_eq!(
            field_type_to_wasm(&FieldType::Primitive(PrimitiveType::Double)),
            WasmMemoryType::F64
        );
        assert_eq!(
            field_type_to_wasm(&FieldType::object("Room")),
            WasmMemoryType::Pointer
        );
        assert_eq!(
            field_type_to_wasm(&FieldType::list(FieldType::object("Room"))),
            WasmMemoryType::ArrayPointer
        );
    }

    #[test]
    fn test_alignment() {
        let mut calc = LayoutCalculator::new();
        let class = DomainClass::new("Test")
            .with_field(FieldDescriptor::new(
                "a",
                FieldType::Primitive(PrimitiveType::Int),
            ))
            .with_field(FieldDescriptor::new(
                "b",
                FieldType::Primitive(PrimitiveType::Long),
            ));

        let layout = calc.calculate_layout(&class);
        let b_layout = layout.field_offsets.get("b").unwrap();

        assert_eq!(b_layout.offset % 8, 0);
    }

    #[test]
    fn test_cached_layout() {
        let mut calc = LayoutCalculator::new();
        let class = create_test_class();

        let layout1 = calc.calculate_layout(&class);
        let layout2 = calc.calculate_layout(&class);

        assert_eq!(layout1.total_size, layout2.total_size);
        assert!(calc.get_layout("Lesson").is_some());
        assert!(calc.get_layout("Unknown").is_none());
    }

    #[test]
    fn test_wasm_memory_type_sizes() {
        assert_eq!(WasmMemoryType::I32.size(), 4);
        assert_eq!(WasmMemoryType::I64.size(), 8);
        assert_eq!(WasmMemoryType::F32.size(), 4);
        assert_eq!(WasmMemoryType::F64.size(), 8);
        assert_eq!(WasmMemoryType::Pointer.size(), 4);
        assert_eq!(WasmMemoryType::ArrayPointer.size(), 4);
    }

    #[test]
    fn test_wasm_memory_type_alignment() {
        assert_eq!(WasmMemoryType::I32.alignment(), 4);
        assert_eq!(WasmMemoryType::I64.alignment(), 8);
        assert_eq!(WasmMemoryType::F32.alignment(), 4);
        assert_eq!(WasmMemoryType::F64.alignment(), 8);
        assert_eq!(WasmMemoryType::Pointer.alignment(), 4);
        assert_eq!(WasmMemoryType::ArrayPointer.alignment(), 4);
    }

    #[test]
    fn test_empty_class_layout() {
        let mut calc = LayoutCalculator::new();
        let class = DomainClass::new("Empty");
        let layout = calc.calculate_layout(&class);

        assert_eq!(layout.total_size, 4); // Minimum 4 bytes
        assert_eq!(layout.alignment, 4);
        assert!(layout.field_offsets.is_empty());
    }

    #[test]
    fn test_bool_field() {
        let mut calc = LayoutCalculator::new();
        let class = DomainClass::new("Test").with_field(FieldDescriptor::new(
            "active",
            FieldType::Primitive(PrimitiveType::Bool),
        ));

        let layout = calc.calculate_layout(&class);
        let field = layout.field_offsets.get("active").unwrap();

        assert_eq!(field.wasm_type, WasmMemoryType::I32);
    }

    #[test]
    fn test_score_field() {
        use crate::domain::ScoreType;

        let mut calc = LayoutCalculator::new();
        let class = DomainClass::new("Test").with_field(FieldDescriptor::new(
            "score",
            FieldType::Score(ScoreType::HardSoft),
        ));

        let layout = calc.calculate_layout(&class);
        let field = layout.field_offsets.get("score").unwrap();

        assert_eq!(field.wasm_type, WasmMemoryType::Pointer);
    }

    #[test]
    fn test_map_field() {
        let mut calc = LayoutCalculator::new();
        let class = DomainClass::new("Test").with_field(FieldDescriptor::new(
            "data",
            FieldType::map(
                FieldType::Primitive(PrimitiveType::String),
                FieldType::object("Value"),
            ),
        ));

        let layout = calc.calculate_layout(&class);
        let field = layout.field_offsets.get("data").unwrap();

        assert_eq!(field.wasm_type, WasmMemoryType::Pointer);
    }
}
