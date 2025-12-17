package solverforge

import (
	"fmt"
)

// ValueType represents the type of a Value
type ValueType int

const (
	ValueTypeNull ValueType = iota
	ValueTypeBool
	ValueTypeInt
	ValueTypeFloat
	ValueTypeString
	ValueTypeArray
	ValueTypeObject
	ValueTypeObjectRef
)

func (vt ValueType) String() string {
	switch vt {
	case ValueTypeNull:
		return "Null"
	case ValueTypeBool:
		return "Bool"
	case ValueTypeInt:
		return "Int"
	case ValueTypeFloat:
		return "Float"
	case ValueTypeString:
		return "String"
	case ValueTypeArray:
		return "Array"
	case ValueTypeObject:
		return "Object"
	case ValueTypeObjectRef:
		return "ObjectRef"
	default:
		return fmt.Sprintf("Unknown(%d)", vt)
	}
}

// Value is the interface for all value types
type Value interface {
	Type() ValueType
	String() string
}

// NullValue represents a null value
type NullValue struct{}

func (v NullValue) Type() ValueType { return ValueTypeNull }
func (v NullValue) String() string  { return "null" }

// BoolValue represents a boolean value
type BoolValue struct {
	Value bool
}

func (v BoolValue) Type() ValueType { return ValueTypeBool }
func (v BoolValue) String() string  { return fmt.Sprintf("%v", v.Value) }

// IntValue represents an integer value
type IntValue struct {
	Value int64
}

func (v IntValue) Type() ValueType { return ValueTypeInt }
func (v IntValue) String() string  { return fmt.Sprintf("%d", v.Value) }

// FloatValue represents a floating point value
type FloatValue struct {
	Value float64
}

func (v FloatValue) Type() ValueType { return ValueTypeFloat }
func (v FloatValue) String() string  { return fmt.Sprintf("%f", v.Value) }

// StringValue represents a string value
type StringValue struct {
	Value string
}

func (v StringValue) Type() ValueType { return ValueTypeString }
func (v StringValue) String() string  { return fmt.Sprintf("%q", v.Value) }

// ArrayValue represents an array of values
type ArrayValue struct {
	Values []Value
}

func (v ArrayValue) Type() ValueType { return ValueTypeArray }
func (v ArrayValue) String() string {
	result := "["
	for i, val := range v.Values {
		if i > 0 {
			result += ", "
		}
		result += val.String()
	}
	result += "]"
	return result
}

// ObjectValue represents a map of string keys to values
type ObjectValue struct {
	Fields map[string]Value
}

func (v ObjectValue) Type() ValueType { return ValueTypeObject }
func (v ObjectValue) String() string {
	result := "{"
	first := true
	for k, val := range v.Fields {
		if !first {
			result += ", "
		}
		first = false
		result += fmt.Sprintf("%q: %s", k, val.String())
	}
	result += "}"
	return result
}

// ObjectRefValue represents a reference to a host language object
type ObjectRefValue struct {
	Handle ObjectHandle
}

func (v ObjectRefValue) Type() ValueType { return ValueTypeObjectRef }
func (v ObjectRefValue) String() string  { return fmt.Sprintf("ObjectRef(%d)", v.Handle) }

// Note: C value conversion (cValueToGo/valueToc) will be implemented
// in the bridge layer when needed for FFI calls. The Value types above
// are pure Go types.

// Helper constructors for common cases

// NewNullValue creates a new null value
func NewNullValue() Value {
	return NullValue{}
}

// NewBoolValue creates a new boolean value
func NewBoolValue(b bool) Value {
	return BoolValue{Value: b}
}

// NewIntValue creates a new integer value
func NewIntValue(i int64) Value {
	return IntValue{Value: i}
}

// NewFloatValue creates a new float value
func NewFloatValue(f float64) Value {
	return FloatValue{Value: f}
}

// NewStringValue creates a new string value
func NewStringValue(s string) Value {
	return StringValue{Value: s}
}

// NewArrayValue creates a new array value
func NewArrayValue(values []Value) Value {
	return ArrayValue{Values: values}
}

// NewObjectValue creates a new object value
func NewObjectValue(fields map[string]Value) Value {
	return ObjectValue{Fields: fields}
}

// NewObjectRefValue creates a new object reference value
func NewObjectRefValue(handle ObjectHandle) Value {
	return ObjectRefValue{Handle: handle}
}
