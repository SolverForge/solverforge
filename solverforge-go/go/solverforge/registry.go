package solverforge

import (
	"fmt"
	"sync"
	"sync/atomic"
)

// GoRegistry manages Go objects that need to be referenced from Rust
// It provides thread-safe storage using numeric IDs to comply with CGO pointer rules
type GoRegistry struct {
	objects sync.Map // map[uint64]interface{}
	nextID  uint64   // atomic counter for generating IDs
}

// globalRegistry is the default registry instance
var globalRegistry = NewGoRegistry()

// NewGoRegistry creates a new object registry
func NewGoRegistry() *GoRegistry {
	return &GoRegistry{
		nextID: 1, // Start at 1, reserve 0 for invalid/null
	}
}

// Register stores an object and returns a unique ID
// The ID can be safely passed across the FFI boundary
func (r *GoRegistry) Register(obj interface{}) uint64 {
	if obj == nil {
		return 0
	}
	id := atomic.AddUint64(&r.nextID, 1)
	r.objects.Store(id, obj)
	return id
}

// Get retrieves an object by ID
// Returns (object, true) if found, (nil, false) if not found
func (r *GoRegistry) Get(id uint64) (interface{}, bool) {
	if id == 0 {
		return nil, false
	}
	return r.objects.Load(id)
}

// Release removes an object from the registry
// Returns true if the object was found and removed, false otherwise
func (r *GoRegistry) Release(id uint64) bool {
	if id == 0 {
		return false
	}
	_, loaded := r.objects.LoadAndDelete(id)
	return loaded
}

// GetTyped retrieves an object and type-asserts it
// Returns error if not found or wrong type
func (r *GoRegistry) GetTyped(id uint64, target interface{}) error {
	obj, ok := r.Get(id)
	if !ok {
		return fmt.Errorf("object with ID %d not found in registry", id)
	}

	// Use type switch to handle common cases
	switch t := target.(type) {
	case *interface{}:
		*t = obj
		return nil
	default:
		return fmt.Errorf("GetTyped requires a pointer to the target type")
	}
}

// Count returns the approximate number of objects in the registry
// This is useful for debugging and testing
func (r *GoRegistry) Count() int {
	count := 0
	r.objects.Range(func(_, _ interface{}) bool {
		count++
		return true
	})
	return count
}

// Clear removes all objects from the registry
// This is mainly useful for testing
func (r *GoRegistry) Clear() {
	r.objects.Range(func(key, _ interface{}) bool {
		r.objects.Delete(key)
		return true
	})
}

// Global registry functions for convenience

// RegisterObject stores an object in the global registry
func RegisterObject(obj interface{}) uint64 {
	return globalRegistry.Register(obj)
}

// GetObject retrieves an object from the global registry
func GetObject(id uint64) (interface{}, bool) {
	return globalRegistry.Get(id)
}

// ReleaseObject removes an object from the global registry
func ReleaseObject(id uint64) bool {
	return globalRegistry.Release(id)
}

// FunctionRegistry manages Go functions that need to be called from Rust
type FunctionRegistry struct {
	functions sync.Map // map[uint64]func([]Value) (Value, error)
	nextID    uint64   // atomic counter
}

// globalFunctionRegistry is the default function registry
var globalFunctionRegistry = NewFunctionRegistry()

// NewFunctionRegistry creates a new function registry
func NewFunctionRegistry() *FunctionRegistry {
	return &FunctionRegistry{
		nextID: 1,
	}
}

// Register stores a function and returns a unique ID
func (r *FunctionRegistry) Register(fn func([]Value) (Value, error)) uint64 {
	if fn == nil {
		return 0
	}
	id := atomic.AddUint64(&r.nextID, 1)
	r.functions.Store(id, fn)
	return id
}

// Get retrieves a function by ID
func (r *FunctionRegistry) Get(id uint64) (func([]Value) (Value, error), bool) {
	if id == 0 {
		return nil, false
	}
	fn, ok := r.functions.Load(id)
	if !ok {
		return nil, false
	}
	return fn.(func([]Value) (Value, error)), true
}

// Release removes a function from the registry
func (r *FunctionRegistry) Release(id uint64) bool {
	if id == 0 {
		return false
	}
	_, loaded := r.functions.LoadAndDelete(id)
	return loaded
}

// Count returns the approximate number of functions in the registry
func (r *FunctionRegistry) Count() int {
	count := 0
	r.functions.Range(func(_, _ interface{}) bool {
		count++
		return true
	})
	return count
}

// Clear removes all functions from the registry
func (r *FunctionRegistry) Clear() {
	r.functions.Range(func(key, _ interface{}) bool {
		r.functions.Delete(key)
		return true
	})
}

// Global function registry functions

// RegisterFunction stores a function in the global registry
func RegisterFunction(fn func([]Value) (Value, error)) uint64 {
	return globalFunctionRegistry.Register(fn)
}

// GetFunction retrieves a function from the global registry
func GetFunction(id uint64) (func([]Value) (Value, error), bool) {
	return globalFunctionRegistry.Get(id)
}

// ReleaseFunction removes a function from the global registry
func ReleaseFunction(id uint64) bool {
	return globalFunctionRegistry.Release(id)
}
