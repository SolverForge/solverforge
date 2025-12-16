package solverforge

import (
	"sync"
	"testing"
)

func TestGoRegistry(t *testing.T) {
	t.Run("Register and Get", func(t *testing.T) {
		reg := NewGoRegistry()
		obj := "test object"

		id := reg.Register(obj)
		if id == 0 {
			t.Fatal("Register() returned 0 (invalid ID)")
		}

		retrieved, ok := reg.Get(id)
		if !ok {
			t.Fatal("Get() returned false")
		}

		if retrieved != obj {
			t.Errorf("Get() = %v, want %v", retrieved, obj)
		}
	})

	t.Run("Register nil", func(t *testing.T) {
		reg := NewGoRegistry()
		id := reg.Register(nil)
		if id != 0 {
			t.Errorf("Register(nil) = %d, want 0", id)
		}
	})

	t.Run("Get nonexistent", func(t *testing.T) {
		reg := NewGoRegistry()
		_, ok := reg.Get(999999)
		if ok {
			t.Error("Get(nonexistent) returned true")
		}
	})

	t.Run("Get with ID 0", func(t *testing.T) {
		reg := NewGoRegistry()
		_, ok := reg.Get(0)
		if ok {
			t.Error("Get(0) returned true")
		}
	})

	t.Run("Release", func(t *testing.T) {
		reg := NewGoRegistry()
		obj := "test object"

		id := reg.Register(obj)
		if !reg.Release(id) {
			t.Error("Release() returned false")
		}

		_, ok := reg.Get(id)
		if ok {
			t.Error("Get() after Release() returned true")
		}
	})

	t.Run("Release nonexistent", func(t *testing.T) {
		reg := NewGoRegistry()
		if reg.Release(999999) {
			t.Error("Release(nonexistent) returned true")
		}
	})

	t.Run("Release with ID 0", func(t *testing.T) {
		reg := NewGoRegistry()
		if reg.Release(0) {
			t.Error("Release(0) returned true")
		}
	})

	t.Run("Multiple objects", func(t *testing.T) {
		reg := NewGoRegistry()
		obj1 := "object1"
		obj2 := "object2"
		obj3 := "object3"

		id1 := reg.Register(obj1)
		id2 := reg.Register(obj2)
		id3 := reg.Register(obj3)

		if id1 == id2 || id2 == id3 || id1 == id3 {
			t.Error("Register() returned duplicate IDs")
		}

		if retrieved, ok := reg.Get(id2); !ok || retrieved != obj2 {
			t.Errorf("Get(id2) = %v, %v; want %v, true", retrieved, ok, obj2)
		}
	})

	t.Run("Count", func(t *testing.T) {
		reg := NewGoRegistry()

		if count := reg.Count(); count != 0 {
			t.Errorf("Count() = %d, want 0", count)
		}

		reg.Register("obj1")
		reg.Register("obj2")
		reg.Register("obj3")

		if count := reg.Count(); count != 3 {
			t.Errorf("Count() = %d, want 3", count)
		}
	})

	t.Run("Clear", func(t *testing.T) {
		reg := NewGoRegistry()
		id1 := reg.Register("obj1")
		id2 := reg.Register("obj2")

		reg.Clear()

		if count := reg.Count(); count != 0 {
			t.Errorf("Count() after Clear() = %d, want 0", count)
		}

		if _, ok := reg.Get(id1); ok {
			t.Error("Get(id1) after Clear() returned true")
		}
		if _, ok := reg.Get(id2); ok {
			t.Error("Get(id2) after Clear() returned true")
		}
	})

	t.Run("Concurrent access", func(t *testing.T) {
		reg := NewGoRegistry()
		var wg sync.WaitGroup
		numGoroutines := 100

		// Concurrent registrations
		wg.Add(numGoroutines)
		for i := 0; i < numGoroutines; i++ {
			go func(n int) {
				defer wg.Done()
				id := reg.Register(n)
				if id == 0 {
					t.Error("Register() returned 0")
				}
			}(i)
		}
		wg.Wait()

		if count := reg.Count(); count != numGoroutines {
			t.Errorf("Count() = %d, want %d", count, numGoroutines)
		}
	})
}

func TestFunctionRegistry(t *testing.T) {
	t.Run("Register and Get", func(t *testing.T) {
		reg := NewFunctionRegistry()
		fn := func(args []Value) (Value, error) {
			return IntValue{Value: 42}, nil
		}

		id := reg.Register(fn)
		if id == 0 {
			t.Fatal("Register() returned 0 (invalid ID)")
		}

		retrieved, ok := reg.Get(id)
		if !ok {
			t.Fatal("Get() returned false")
		}

		// Call the function to verify it works
		result, err := retrieved(nil)
		if err != nil {
			t.Fatalf("Function call error = %v", err)
		}

		if iv, ok := result.(IntValue); !ok || iv.Value != 42 {
			t.Errorf("Function result = %v, want IntValue{42}", result)
		}
	})

	t.Run("Register nil", func(t *testing.T) {
		reg := NewFunctionRegistry()
		id := reg.Register(nil)
		if id != 0 {
			t.Errorf("Register(nil) = %d, want 0", id)
		}
	})

	t.Run("Get nonexistent", func(t *testing.T) {
		reg := NewFunctionRegistry()
		_, ok := reg.Get(999999)
		if ok {
			t.Error("Get(nonexistent) returned true")
		}
	})

	t.Run("Release", func(t *testing.T) {
		reg := NewFunctionRegistry()
		fn := func(args []Value) (Value, error) {
			return NullValue{}, nil
		}

		id := reg.Register(fn)
		if !reg.Release(id) {
			t.Error("Release() returned false")
		}

		_, ok := reg.Get(id)
		if ok {
			t.Error("Get() after Release() returned true")
		}
	})

	t.Run("Multiple functions", func(t *testing.T) {
		reg := NewFunctionRegistry()

		fn1 := func(args []Value) (Value, error) { return IntValue{Value: 1}, nil }
		fn2 := func(args []Value) (Value, error) { return IntValue{Value: 2}, nil }
		fn3 := func(args []Value) (Value, error) { return IntValue{Value: 3}, nil }

		id1 := reg.Register(fn1)
		id2 := reg.Register(fn2)
		id3 := reg.Register(fn3)

		if id1 == id2 || id2 == id3 || id1 == id3 {
			t.Error("Register() returned duplicate IDs")
		}

		// Verify we can call each function
		if fn, ok := reg.Get(id2); ok {
			result, _ := fn(nil)
			if iv, ok := result.(IntValue); !ok || iv.Value != 2 {
				t.Errorf("Function result = %v, want IntValue{2}", result)
			}
		} else {
			t.Error("Get(id2) returned false")
		}
	})

	t.Run("Count", func(t *testing.T) {
		reg := NewFunctionRegistry()

		if count := reg.Count(); count != 0 {
			t.Errorf("Count() = %d, want 0", count)
		}

		fn := func(args []Value) (Value, error) { return NullValue{}, nil }
		reg.Register(fn)
		reg.Register(fn)
		reg.Register(fn)

		if count := reg.Count(); count != 3 {
			t.Errorf("Count() = %d, want 3", count)
		}
	})

	t.Run("Clear", func(t *testing.T) {
		reg := NewFunctionRegistry()
		fn := func(args []Value) (Value, error) { return NullValue{}, nil }

		id1 := reg.Register(fn)
		id2 := reg.Register(fn)

		reg.Clear()

		if count := reg.Count(); count != 0 {
			t.Errorf("Count() after Clear() = %d, want 0", count)
		}

		if _, ok := reg.Get(id1); ok {
			t.Error("Get(id1) after Clear() returned true")
		}
		if _, ok := reg.Get(id2); ok {
			t.Error("Get(id2) after Clear() returned true")
		}
	})
}

func TestGlobalRegistry(t *testing.T) {
	// Clean up before test
	globalRegistry.Clear()
	globalFunctionRegistry.Clear()

	t.Run("RegisterObject", func(t *testing.T) {
		obj := "global test"
		id := RegisterObject(obj)
		if id == 0 {
			t.Fatal("RegisterObject() returned 0")
		}

		retrieved, ok := GetObject(id)
		if !ok || retrieved != obj {
			t.Errorf("GetObject() = %v, %v; want %v, true", retrieved, ok, obj)
		}

		if !ReleaseObject(id) {
			t.Error("ReleaseObject() returned false")
		}
	})

	t.Run("RegisterFunction", func(t *testing.T) {
		fn := func(args []Value) (Value, error) {
			return BoolValue{Value: true}, nil
		}

		id := RegisterFunction(fn)
		if id == 0 {
			t.Fatal("RegisterFunction() returned 0")
		}

		retrieved, ok := GetFunction(id)
		if !ok {
			t.Fatal("GetFunction() returned false")
		}

		result, err := retrieved(nil)
		if err != nil {
			t.Fatalf("Function call error = %v", err)
		}

		if bv, ok := result.(BoolValue); !ok || !bv.Value {
			t.Errorf("Function result = %v, want BoolValue{true}", result)
		}

		if !ReleaseFunction(id) {
			t.Error("ReleaseFunction() returned false")
		}
	})
}
