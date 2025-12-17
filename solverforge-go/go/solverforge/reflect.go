package solverforge

import (
	"fmt"
	"reflect"
)

// ToValue converts a Go value to a Value using reflection
// Supports: nil, bool, integers, floats, strings, slices, arrays, maps, structs, and pointers
func ToValue(v interface{}) (Value, error) {
	if v == nil {
		return NullValue{}, nil
	}

	return toValueReflect(reflect.ValueOf(v))
}

// toValueReflect is the internal reflection-based converter
func toValueReflect(rv reflect.Value) (Value, error) {
	// Handle invalid values
	if !rv.IsValid() {
		return NullValue{}, nil
	}

	// Dereference pointers
	for rv.Kind() == reflect.Ptr {
		if rv.IsNil() {
			return NullValue{}, nil
		}
		rv = rv.Elem()
	}

	// Handle interfaces
	if rv.Kind() == reflect.Interface {
		if rv.IsNil() {
			return NullValue{}, nil
		}
		rv = rv.Elem()
	}

	switch rv.Kind() {
	case reflect.Bool:
		return BoolValue{Value: rv.Bool()}, nil

	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return IntValue{Value: rv.Int()}, nil

	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		// Convert unsigned to signed (may lose precision for very large values)
		return IntValue{Value: int64(rv.Uint())}, nil

	case reflect.Float32, reflect.Float64:
		return FloatValue{Value: rv.Float()}, nil

	case reflect.String:
		return StringValue{Value: rv.String()}, nil

	case reflect.Slice, reflect.Array:
		return sliceToValue(rv)

	case reflect.Map:
		return mapToValue(rv)

	case reflect.Struct:
		return structToValue(rv)

	default:
		return nil, fmt.Errorf("unsupported type: %v", rv.Type())
	}
}

// sliceToValue converts a slice or array to an ArrayValue
func sliceToValue(rv reflect.Value) (Value, error) {
	length := rv.Len()
	values := make([]Value, length)

	for i := 0; i < length; i++ {
		elem := rv.Index(i)
		val, err := toValueReflect(elem)
		if err != nil {
			return nil, fmt.Errorf("failed to convert array element %d: %w", i, err)
		}
		values[i] = val
	}

	return ArrayValue{Values: values}, nil
}

// mapToValue converts a map to an ObjectValue
func mapToValue(rv reflect.Value) (Value, error) {
	// Only support maps with string keys
	if rv.Type().Key().Kind() != reflect.String {
		return nil, fmt.Errorf("only maps with string keys are supported, got: %v", rv.Type().Key())
	}

	fields := make(map[string]Value, rv.Len())
	iter := rv.MapRange()
	for iter.Next() {
		key := iter.Key().String()
		val, err := toValueReflect(iter.Value())
		if err != nil {
			return nil, fmt.Errorf("failed to convert map value for key %q: %w", key, err)
		}
		fields[key] = val
	}

	return ObjectValue{Fields: fields}, nil
}

// structToValue converts a struct to an ObjectValue
// It exports all public fields (fields starting with uppercase letter)
func structToValue(rv reflect.Value) (Value, error) {
	rt := rv.Type()
	fields := make(map[string]Value)

	for i := 0; i < rt.NumField(); i++ {
		field := rt.Field(i)
		fieldValue := rv.Field(i)

		// Skip unexported fields
		if !field.IsExported() {
			continue
		}

		// Use json tag if available, otherwise use field name
		fieldName := field.Name
		if jsonTag := field.Tag.Get("json"); jsonTag != "" && jsonTag != "-" {
			// Parse json tag (format: "name,omitempty,...")
			if comma := 0; comma < len(jsonTag) {
				for comma < len(jsonTag) && jsonTag[comma] != ',' {
					comma++
				}
				if comma > 0 {
					fieldName = jsonTag[:comma]
				}
			}
		}

		val, err := toValueReflect(fieldValue)
		if err != nil {
			return nil, fmt.Errorf("failed to convert struct field %q: %w", fieldName, err)
		}

		fields[fieldName] = val
	}

	return ObjectValue{Fields: fields}, nil
}

// FromValue converts a Value back to a Go value
// The target must be a pointer to the desired type
func FromValue(val Value, target interface{}) error {
	if target == nil {
		return fmt.Errorf("target cannot be nil")
	}

	rv := reflect.ValueOf(target)
	if rv.Kind() != reflect.Ptr {
		return fmt.Errorf("target must be a pointer, got %v", rv.Type())
	}

	if rv.IsNil() {
		return fmt.Errorf("target pointer is nil")
	}

	return fromValueReflect(val, rv.Elem())
}

// fromValueReflect is the internal reflection-based converter from Value to Go value
func fromValueReflect(val Value, rv reflect.Value) error {
	// Handle null values
	if _, isNull := val.(NullValue); isNull {
		// Set zero value for the target type
		rv.Set(reflect.Zero(rv.Type()))
		return nil
	}

	switch rv.Kind() {
	case reflect.Bool:
		if bv, ok := val.(BoolValue); ok {
			rv.SetBool(bv.Value)
			return nil
		}
		return fmt.Errorf("expected BoolValue, got %T", val)

	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		if iv, ok := val.(IntValue); ok {
			rv.SetInt(iv.Value)
			return nil
		}
		return fmt.Errorf("expected IntValue, got %T", val)

	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		if iv, ok := val.(IntValue); ok {
			rv.SetUint(uint64(iv.Value))
			return nil
		}
		return fmt.Errorf("expected IntValue, got %T", val)

	case reflect.Float32, reflect.Float64:
		if fv, ok := val.(FloatValue); ok {
			rv.SetFloat(fv.Value)
			return nil
		}
		// Also accept IntValue and convert to float
		if iv, ok := val.(IntValue); ok {
			rv.SetFloat(float64(iv.Value))
			return nil
		}
		return fmt.Errorf("expected FloatValue or IntValue, got %T", val)

	case reflect.String:
		if sv, ok := val.(StringValue); ok {
			rv.SetString(sv.Value)
			return nil
		}
		return fmt.Errorf("expected StringValue, got %T", val)

	case reflect.Slice:
		return arrayToSlice(val, rv)

	case reflect.Array:
		return arrayToArray(val, rv)

	case reflect.Map:
		return objectToMap(val, rv)

	case reflect.Struct:
		return objectToStruct(val, rv)

	case reflect.Ptr:
		if rv.IsNil() {
			// Allocate new value
			rv.Set(reflect.New(rv.Type().Elem()))
		}
		return fromValueReflect(val, rv.Elem())

	case reflect.Interface:
		// For interface{}, store the Value directly
		if rv.Type() == reflect.TypeOf((*interface{})(nil)).Elem() {
			rv.Set(reflect.ValueOf(val))
			return nil
		}
		return fmt.Errorf("unsupported interface type: %v", rv.Type())

	default:
		return fmt.Errorf("unsupported target type: %v", rv.Type())
	}
}

// arrayToSlice converts an ArrayValue to a Go slice
func arrayToSlice(val Value, rv reflect.Value) error {
	av, ok := val.(ArrayValue)
	if !ok {
		return fmt.Errorf("expected ArrayValue, got %T", val)
	}

	slice := reflect.MakeSlice(rv.Type(), len(av.Values), len(av.Values))
	for i, elem := range av.Values {
		if err := fromValueReflect(elem, slice.Index(i)); err != nil {
			return fmt.Errorf("failed to convert slice element %d: %w", i, err)
		}
	}

	rv.Set(slice)
	return nil
}

// arrayToArray converts an ArrayValue to a Go array
func arrayToArray(val Value, rv reflect.Value) error {
	av, ok := val.(ArrayValue)
	if !ok {
		return fmt.Errorf("expected ArrayValue, got %T", val)
	}

	if len(av.Values) != rv.Len() {
		return fmt.Errorf("array length mismatch: expected %d, got %d", rv.Len(), len(av.Values))
	}

	for i, elem := range av.Values {
		if err := fromValueReflect(elem, rv.Index(i)); err != nil {
			return fmt.Errorf("failed to convert array element %d: %w", i, err)
		}
	}

	return nil
}

// objectToMap converts an ObjectValue to a Go map
func objectToMap(val Value, rv reflect.Value) error {
	ov, ok := val.(ObjectValue)
	if !ok {
		return fmt.Errorf("expected ObjectValue, got %T", val)
	}

	// Only support maps with string keys
	if rv.Type().Key().Kind() != reflect.String {
		return fmt.Errorf("only maps with string keys are supported, got: %v", rv.Type().Key())
	}

	if rv.IsNil() {
		rv.Set(reflect.MakeMap(rv.Type()))
	}

	for key, elem := range ov.Fields {
		keyVal := reflect.ValueOf(key)
		elemVal := reflect.New(rv.Type().Elem()).Elem()
		if err := fromValueReflect(elem, elemVal); err != nil {
			return fmt.Errorf("failed to convert map value for key %q: %w", key, err)
		}
		rv.SetMapIndex(keyVal, elemVal)
	}

	return nil
}

// objectToStruct converts an ObjectValue to a Go struct
func objectToStruct(val Value, rv reflect.Value) error {
	ov, ok := val.(ObjectValue)
	if !ok {
		return fmt.Errorf("expected ObjectValue, got %T", val)
	}

	rt := rv.Type()
	for i := 0; i < rt.NumField(); i++ {
		field := rt.Field(i)
		if !field.IsExported() {
			continue
		}

		// Use json tag if available, otherwise use field name
		fieldName := field.Name
		if jsonTag := field.Tag.Get("json"); jsonTag != "" && jsonTag != "-" {
			// Parse json tag
			if comma := 0; comma < len(jsonTag) {
				for comma < len(jsonTag) && jsonTag[comma] != ',' {
					comma++
				}
				if comma > 0 {
					fieldName = jsonTag[:comma]
				}
			}
		}

		// Look up the value in the object
		if elemVal, exists := ov.Fields[fieldName]; exists {
			fieldVal := rv.Field(i)
			if err := fromValueReflect(elemVal, fieldVal); err != nil {
				return fmt.Errorf("failed to convert struct field %q: %w", fieldName, err)
			}
		}
		// If field doesn't exist in object, leave it as zero value
	}

	return nil
}
