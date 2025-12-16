package solverforge

import (
	"reflect"
	"testing"
)

func TestToValue(t *testing.T) {
	t.Run("nil", func(t *testing.T) {
		val, err := ToValue(nil)
		if err != nil {
			t.Fatalf("ToValue(nil) error = %v", err)
		}
		if val.Type() != ValueTypeNull {
			t.Errorf("ToValue(nil) type = %v, want %v", val.Type(), ValueTypeNull)
		}
	})

	t.Run("bool", func(t *testing.T) {
		val, err := ToValue(true)
		if err != nil {
			t.Fatalf("ToValue(true) error = %v", err)
		}
		bv, ok := val.(BoolValue)
		if !ok || !bv.Value {
			t.Errorf("ToValue(true) = %v, want BoolValue{true}", val)
		}
	})

	t.Run("integers", func(t *testing.T) {
		tests := []struct {
			name  string
			input interface{}
			want  int64
		}{
			{"int", int(42), 42},
			{"int8", int8(42), 42},
			{"int16", int16(42), 42},
			{"int32", int32(42), 42},
			{"int64", int64(42), 42},
			{"uint", uint(42), 42},
			{"uint8", uint8(42), 42},
			{"uint16", uint16(42), 42},
			{"uint32", uint32(42), 42},
			{"uint64", uint64(42), 42},
		}

		for _, tt := range tests {
			t.Run(tt.name, func(t *testing.T) {
				val, err := ToValue(tt.input)
				if err != nil {
					t.Fatalf("ToValue() error = %v", err)
				}
				iv, ok := val.(IntValue)
				if !ok || iv.Value != tt.want {
					t.Errorf("ToValue(%v) = %v, want IntValue{%d}", tt.input, val, tt.want)
				}
			})
		}
	})

	t.Run("floats", func(t *testing.T) {
		tests := []struct {
			name  string
			input interface{}
			want  float64
		}{
			{"float32", float32(3.14), 3.14},
			{"float64", float64(3.14), 3.14},
		}

		for _, tt := range tests {
			t.Run(tt.name, func(t *testing.T) {
				val, err := ToValue(tt.input)
				if err != nil {
					t.Fatalf("ToValue() error = %v", err)
				}
				fv, ok := val.(FloatValue)
				if !ok {
					t.Fatalf("ToValue() = %T, want FloatValue", val)
				}
				// Use approximate comparison for floats
				if diff := fv.Value - tt.want; diff < -0.01 || diff > 0.01 {
					t.Errorf("ToValue(%v) = FloatValue{%f}, want FloatValue{%f}", tt.input, fv.Value, tt.want)
				}
			})
		}
	})

	t.Run("string", func(t *testing.T) {
		val, err := ToValue("hello")
		if err != nil {
			t.Fatalf("ToValue(\"hello\") error = %v", err)
		}
		sv, ok := val.(StringValue)
		if !ok || sv.Value != "hello" {
			t.Errorf("ToValue(\"hello\") = %v, want StringValue{\"hello\"}", val)
		}
	})

	t.Run("slice", func(t *testing.T) {
		input := []int{1, 2, 3}
		val, err := ToValue(input)
		if err != nil {
			t.Fatalf("ToValue() error = %v", err)
		}
		av, ok := val.(ArrayValue)
		if !ok {
			t.Fatalf("ToValue() = %T, want ArrayValue", val)
		}
		if len(av.Values) != 3 {
			t.Errorf("ArrayValue length = %d, want 3", len(av.Values))
		}
	})

	t.Run("array", func(t *testing.T) {
		input := [3]int{1, 2, 3}
		val, err := ToValue(input)
		if err != nil {
			t.Fatalf("ToValue() error = %v", err)
		}
		av, ok := val.(ArrayValue)
		if !ok {
			t.Fatalf("ToValue() = %T, want ArrayValue", val)
		}
		if len(av.Values) != 3 {
			t.Errorf("ArrayValue length = %d, want 3", len(av.Values))
		}
	})

	t.Run("map", func(t *testing.T) {
		input := map[string]int{"a": 1, "b": 2}
		val, err := ToValue(input)
		if err != nil {
			t.Fatalf("ToValue() error = %v", err)
		}
		ov, ok := val.(ObjectValue)
		if !ok {
			t.Fatalf("ToValue() = %T, want ObjectValue", val)
		}
		if len(ov.Fields) != 2 {
			t.Errorf("ObjectValue field count = %d, want 2", len(ov.Fields))
		}
	})

	t.Run("struct", func(t *testing.T) {
		type Person struct {
			Name string
			Age  int
		}
		input := Person{Name: "Alice", Age: 30}
		val, err := ToValue(input)
		if err != nil {
			t.Fatalf("ToValue() error = %v", err)
		}
		ov, ok := val.(ObjectValue)
		if !ok {
			t.Fatalf("ToValue() = %T, want ObjectValue", val)
		}
		if len(ov.Fields) != 2 {
			t.Errorf("ObjectValue field count = %d, want 2", len(ov.Fields))
		}
		if nameVal, ok := ov.Fields["Name"]; !ok {
			t.Error("ObjectValue missing 'Name' field")
		} else if sv, ok := nameVal.(StringValue); !ok || sv.Value != "Alice" {
			t.Errorf("Name field = %v, want StringValue{\"Alice\"}", nameVal)
		}
	})

	t.Run("struct with json tags", func(t *testing.T) {
		type Person struct {
			Name string `json:"name"`
			Age  int    `json:"age"`
		}
		input := Person{Name: "Bob", Age: 25}
		val, err := ToValue(input)
		if err != nil {
			t.Fatalf("ToValue() error = %v", err)
		}
		ov, ok := val.(ObjectValue)
		if !ok {
			t.Fatalf("ToValue() = %T, want ObjectValue", val)
		}
		if _, ok := ov.Fields["name"]; !ok {
			t.Error("ObjectValue missing 'name' field (should use json tag)")
		}
		if _, ok := ov.Fields["Name"]; ok {
			t.Error("ObjectValue has 'Name' field (should use json tag 'name' instead)")
		}
	})

	t.Run("pointer", func(t *testing.T) {
		i := 42
		val, err := ToValue(&i)
		if err != nil {
			t.Fatalf("ToValue() error = %v", err)
		}
		iv, ok := val.(IntValue)
		if !ok || iv.Value != 42 {
			t.Errorf("ToValue(&42) = %v, want IntValue{42}", val)
		}
	})

	t.Run("nil pointer", func(t *testing.T) {
		var p *int = nil
		val, err := ToValue(p)
		if err != nil {
			t.Fatalf("ToValue(nil pointer) error = %v", err)
		}
		if val.Type() != ValueTypeNull {
			t.Errorf("ToValue(nil pointer) type = %v, want %v", val.Type(), ValueTypeNull)
		}
	})

	t.Run("nested struct", func(t *testing.T) {
		type Address struct {
			City string
		}
		type Person struct {
			Name    string
			Address Address
		}
		input := Person{
			Name:    "Charlie",
			Address: Address{City: "NYC"},
		}
		val, err := ToValue(input)
		if err != nil {
			t.Fatalf("ToValue() error = %v", err)
		}
		ov, ok := val.(ObjectValue)
		if !ok {
			t.Fatalf("ToValue() = %T, want ObjectValue", val)
		}
		addrVal, ok := ov.Fields["Address"]
		if !ok {
			t.Fatal("ObjectValue missing 'Address' field")
		}
		addrObj, ok := addrVal.(ObjectValue)
		if !ok {
			t.Fatalf("Address field = %T, want ObjectValue", addrVal)
		}
		if cityVal, ok := addrObj.Fields["City"]; !ok {
			t.Error("Address missing 'City' field")
		} else if sv, ok := cityVal.(StringValue); !ok || sv.Value != "NYC" {
			t.Errorf("City field = %v, want StringValue{\"NYC\"}", cityVal)
		}
	})
}

func TestFromValue(t *testing.T) {
	t.Run("null to int", func(t *testing.T) {
		var i int
		err := FromValue(NullValue{}, &i)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		if i != 0 {
			t.Errorf("FromValue(null) = %d, want 0", i)
		}
	})

	t.Run("bool", func(t *testing.T) {
		var b bool
		err := FromValue(BoolValue{Value: true}, &b)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		if !b {
			t.Error("FromValue(BoolValue{true}) = false, want true")
		}
	})

	t.Run("int", func(t *testing.T) {
		var i int64
		err := FromValue(IntValue{Value: 42}, &i)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		if i != 42 {
			t.Errorf("FromValue(IntValue{42}) = %d, want 42", i)
		}
	})

	t.Run("float", func(t *testing.T) {
		var f float64
		err := FromValue(FloatValue{Value: 3.14}, &f)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		if f != 3.14 {
			t.Errorf("FromValue(FloatValue{3.14}) = %f, want 3.14", f)
		}
	})

	t.Run("int to float", func(t *testing.T) {
		var f float64
		err := FromValue(IntValue{Value: 42}, &f)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		if f != 42.0 {
			t.Errorf("FromValue(IntValue{42}) = %f, want 42.0", f)
		}
	})

	t.Run("string", func(t *testing.T) {
		var s string
		err := FromValue(StringValue{Value: "hello"}, &s)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		if s != "hello" {
			t.Errorf("FromValue(StringValue{\"hello\"}) = %q, want \"hello\"", s)
		}
	})

	t.Run("slice", func(t *testing.T) {
		var slice []int
		arr := ArrayValue{Values: []Value{
			IntValue{Value: 1},
			IntValue{Value: 2},
			IntValue{Value: 3},
		}}
		err := FromValue(arr, &slice)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		if !reflect.DeepEqual(slice, []int{1, 2, 3}) {
			t.Errorf("FromValue() = %v, want [1 2 3]", slice)
		}
	})

	t.Run("map", func(t *testing.T) {
		var m map[string]int
		obj := ObjectValue{Fields: map[string]Value{
			"a": IntValue{Value: 1},
			"b": IntValue{Value: 2},
		}}
		err := FromValue(obj, &m)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		want := map[string]int{"a": 1, "b": 2}
		if !reflect.DeepEqual(m, want) {
			t.Errorf("FromValue() = %v, want %v", m, want)
		}
	})

	t.Run("struct", func(t *testing.T) {
		type Person struct {
			Name string
			Age  int
		}
		var p Person
		obj := ObjectValue{Fields: map[string]Value{
			"Name": StringValue{Value: "Alice"},
			"Age":  IntValue{Value: 30},
		}}
		err := FromValue(obj, &p)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		if p.Name != "Alice" || p.Age != 30 {
			t.Errorf("FromValue() = %+v, want {Name:Alice Age:30}", p)
		}
	})

	t.Run("struct with json tags", func(t *testing.T) {
		type Person struct {
			Name string `json:"name"`
			Age  int    `json:"age"`
		}
		var p Person
		obj := ObjectValue{Fields: map[string]Value{
			"name": StringValue{Value: "Bob"},
			"age":  IntValue{Value: 25},
		}}
		err := FromValue(obj, &p)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}
		if p.Name != "Bob" || p.Age != 25 {
			t.Errorf("FromValue() = %+v, want {Name:Bob Age:25}", p)
		}
	})

	t.Run("nil target", func(t *testing.T) {
		err := FromValue(IntValue{Value: 42}, nil)
		if err == nil {
			t.Error("FromValue(nil target) should return error")
		}
	})

	t.Run("non-pointer target", func(t *testing.T) {
		var i int
		err := FromValue(IntValue{Value: 42}, i)
		if err == nil {
			t.Error("FromValue(non-pointer) should return error")
		}
	})
}

func TestValueRoundtrip(t *testing.T) {
	t.Run("primitives", func(t *testing.T) {
		tests := []interface{}{
			true,
			int(42),
			int64(42),
			float64(3.14),
			"hello",
		}

		for _, original := range tests {
			t.Run(reflect.TypeOf(original).String(), func(t *testing.T) {
				val, err := ToValue(original)
				if err != nil {
					t.Fatalf("ToValue() error = %v", err)
				}

				// Create target of same type
				target := reflect.New(reflect.TypeOf(original))
				err = FromValue(val, target.Interface())
				if err != nil {
					t.Fatalf("FromValue() error = %v", err)
				}

				result := target.Elem().Interface()
				if !reflect.DeepEqual(result, original) {
					t.Errorf("roundtrip = %v, want %v", result, original)
				}
			})
		}
	})

	t.Run("collections", func(t *testing.T) {
		t.Run("slice", func(t *testing.T) {
			original := []int{1, 2, 3}
			val, err := ToValue(original)
			if err != nil {
				t.Fatalf("ToValue() error = %v", err)
			}

			var result []int
			err = FromValue(val, &result)
			if err != nil {
				t.Fatalf("FromValue() error = %v", err)
			}

			if !reflect.DeepEqual(result, original) {
				t.Errorf("roundtrip = %v, want %v", result, original)
			}
		})

		t.Run("map", func(t *testing.T) {
			original := map[string]int{"a": 1, "b": 2}
			val, err := ToValue(original)
			if err != nil {
				t.Fatalf("ToValue() error = %v", err)
			}

			var result map[string]int
			err = FromValue(val, &result)
			if err != nil {
				t.Fatalf("FromValue() error = %v", err)
			}

			if !reflect.DeepEqual(result, original) {
				t.Errorf("roundtrip = %v, want %v", result, original)
			}
		})
	})

	t.Run("struct", func(t *testing.T) {
		type Person struct {
			Name string
			Age  int
		}
		original := Person{Name: "Alice", Age: 30}
		val, err := ToValue(original)
		if err != nil {
			t.Fatalf("ToValue() error = %v", err)
		}

		var result Person
		err = FromValue(val, &result)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}

		if !reflect.DeepEqual(result, original) {
			t.Errorf("roundtrip = %+v, want %+v", result, original)
		}
	})

	t.Run("nested", func(t *testing.T) {
		type Address struct {
			City  string
			State string
		}
		type Person struct {
			Name    string
			Age     int
			Address Address
		}
		original := Person{
			Name: "Bob",
			Age:  25,
			Address: Address{
				City:  "NYC",
				State: "NY",
			},
		}
		val, err := ToValue(original)
		if err != nil {
			t.Fatalf("ToValue() error = %v", err)
		}

		var result Person
		err = FromValue(val, &result)
		if err != nil {
			t.Fatalf("FromValue() error = %v", err)
		}

		if !reflect.DeepEqual(result, original) {
			t.Errorf("roundtrip = %+v, want %+v", result, original)
		}
	})
}
