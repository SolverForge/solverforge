package solverforge

import (
	"testing"
)

func TestValueTypes(t *testing.T) {
	tests := []struct {
		name     string
		value    Value
		wantType ValueType
	}{
		{"null", NullValue{}, ValueTypeNull},
		{"bool", BoolValue{Value: true}, ValueTypeBool},
		{"int", IntValue{Value: 42}, ValueTypeInt},
		{"float", FloatValue{Value: 3.14}, ValueTypeFloat},
		{"string", StringValue{Value: "hello"}, ValueTypeString},
		{"array", ArrayValue{Values: []Value{IntValue{Value: 1}}}, ValueTypeArray},
		{"object", ObjectValue{Fields: map[string]Value{"key": IntValue{Value: 1}}}, ValueTypeObject},
		{"objectref", ObjectRefValue{Handle: 123}, ValueTypeObjectRef},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := tt.value.Type(); got != tt.wantType {
				t.Errorf("Type() = %v, want %v", got, tt.wantType)
			}
		})
	}
}

func TestValueString(t *testing.T) {
	tests := []struct {
		name  string
		value Value
		want  string
	}{
		{"null", NullValue{}, "null"},
		{"bool_true", BoolValue{Value: true}, "true"},
		{"bool_false", BoolValue{Value: false}, "false"},
		{"int", IntValue{Value: 42}, "42"},
		{"string", StringValue{Value: "hello"}, `"hello"`},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := tt.value.String(); got != tt.want {
				t.Errorf("String() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestValueConstructors(t *testing.T) {
	t.Run("NewNullValue", func(t *testing.T) {
		v := NewNullValue()
		if v.Type() != ValueTypeNull {
			t.Errorf("NewNullValue() type = %v, want %v", v.Type(), ValueTypeNull)
		}
	})

	t.Run("NewBoolValue", func(t *testing.T) {
		v := NewBoolValue(true)
		if bv, ok := v.(BoolValue); !ok || !bv.Value {
			t.Errorf("NewBoolValue(true) = %v, want BoolValue{true}", v)
		}
	})

	t.Run("NewIntValue", func(t *testing.T) {
		v := NewIntValue(42)
		if iv, ok := v.(IntValue); !ok || iv.Value != 42 {
			t.Errorf("NewIntValue(42) = %v, want IntValue{42}", v)
		}
	})

	t.Run("NewFloatValue", func(t *testing.T) {
		v := NewFloatValue(3.14)
		if fv, ok := v.(FloatValue); !ok || fv.Value != 3.14 {
			t.Errorf("NewFloatValue(3.14) = %v, want FloatValue{3.14}", v)
		}
	})

	t.Run("NewStringValue", func(t *testing.T) {
		v := NewStringValue("hello")
		if sv, ok := v.(StringValue); !ok || sv.Value != "hello" {
			t.Errorf("NewStringValue(\"hello\") = %v, want StringValue{\"hello\"}", v)
		}
	})

	t.Run("NewArrayValue", func(t *testing.T) {
		values := []Value{IntValue{Value: 1}, IntValue{Value: 2}}
		v := NewArrayValue(values)
		if av, ok := v.(ArrayValue); !ok || len(av.Values) != 2 {
			t.Errorf("NewArrayValue() = %v, want ArrayValue with 2 elements", v)
		}
	})

	t.Run("NewObjectValue", func(t *testing.T) {
		fields := map[string]Value{"key": IntValue{Value: 42}}
		v := NewObjectValue(fields)
		if ov, ok := v.(ObjectValue); !ok || len(ov.Fields) != 1 {
			t.Errorf("NewObjectValue() = %v, want ObjectValue with 1 field", v)
		}
	})

	t.Run("NewObjectRefValue", func(t *testing.T) {
		v := NewObjectRefValue(123)
		if orv, ok := v.(ObjectRefValue); !ok || orv.Handle != 123 {
			t.Errorf("NewObjectRefValue(123) = %v, want ObjectRefValue{123}", v)
		}
	})
}

// Note: C value conversion tests will be added when implementing the bridge layer (Phase 4)
