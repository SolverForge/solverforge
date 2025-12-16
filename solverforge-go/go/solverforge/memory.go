package solverforge

/*
#include "solverforge.h"
#include <stdlib.h>
*/
import "C"
import (
	"runtime"
	"sync"
	"unsafe"
)

// Finalizer manages cleanup of resources when objects are garbage collected
type Finalizer struct {
	cleanup func()
	once    sync.Once
}

// NewFinalizer creates a new finalizer that will call cleanup when garbage collected
func NewFinalizer(cleanup func()) *Finalizer {
	f := &Finalizer{cleanup: cleanup}
	runtime.SetFinalizer(f, func(f *Finalizer) {
		f.once.Do(func() {
			if f.cleanup != nil {
				f.cleanup()
			}
		})
	})
	return f
}

// Release manually triggers the cleanup and disables the finalizer
func (f *Finalizer) Release() {
	f.once.Do(func() {
		if f.cleanup != nil {
			f.cleanup()
		}
		runtime.SetFinalizer(f, nil)
	})
}

// Note: CValue memory management is handled internally by the Rust FFI layer.
// The Go side doesn't need to directly manage CValue lifetimes as they are
// created and freed within individual FFI call boundaries.

// KeepAlive is a utility to ensure an object is not garbage collected
// until a certain point in the program
// This is useful when passing Go pointers to C code
func KeepAlive(obj interface{}) {
	runtime.KeepAlive(obj)
}

// Pin temporarily prevents an object from being garbage collected
// Returns a function that must be called to unpin the object
func Pin(obj interface{}) func() {
	// Create a channel that keeps a reference to the object
	ch := make(chan interface{}, 1)
	ch <- obj

	return func() {
		<-ch
		close(ch)
	}
}

// StringPool manages C string allocations for reuse
type StringPool struct {
	strings map[string]*C.char
	mu      sync.RWMutex
}

// NewStringPool creates a new string pool
func NewStringPool() *StringPool {
	return &StringPool{
		strings: make(map[string]*C.char),
	}
}

// Get returns a C string for the given Go string, reusing existing allocations
func (sp *StringPool) Get(s string) *C.char {
	sp.mu.RLock()
	if cstr, exists := sp.strings[s]; exists {
		sp.mu.RUnlock()
		return cstr
	}
	sp.mu.RUnlock()

	sp.mu.Lock()
	defer sp.mu.Unlock()

	// Double-check after acquiring write lock
	if cstr, exists := sp.strings[s]; exists {
		return cstr
	}

	cstr := C.CString(s)
	sp.strings[s] = cstr
	return cstr
}

// Release frees all pooled strings
func (sp *StringPool) Release() {
	sp.mu.Lock()
	defer sp.mu.Unlock()

	for _, cstr := range sp.strings {
		C.free(unsafe.Pointer(cstr))
	}
	sp.strings = make(map[string]*C.char)
}

// Size returns the number of strings in the pool
func (sp *StringPool) Size() int {
	sp.mu.RLock()
	defer sp.mu.RUnlock()
	return len(sp.strings)
}

// ArenaAllocator manages a batch of memory allocations that can be freed together
type ArenaAllocator struct {
	allocations []unsafe.Pointer
	mu          sync.Mutex
}

// NewArenaAllocator creates a new arena allocator
func NewArenaAllocator() *ArenaAllocator {
	return &ArenaAllocator{
		allocations: make([]unsafe.Pointer, 0),
	}
}

// Allocate allocates memory and tracks it for batch cleanup
func (a *ArenaAllocator) Allocate(size C.size_t) unsafe.Pointer {
	a.mu.Lock()
	defer a.mu.Unlock()

	ptr := C.malloc(size)
	if ptr != nil {
		a.allocations = append(a.allocations, ptr)
	}
	return ptr
}

// AllocateString allocates a C string and tracks it
func (a *ArenaAllocator) AllocateString(s string) *C.char {
	a.mu.Lock()
	defer a.mu.Unlock()

	cstr := C.CString(s)
	a.allocations = append(a.allocations, unsafe.Pointer(cstr))
	return cstr
}

// Release frees all tracked allocations
func (a *ArenaAllocator) Release() {
	a.mu.Lock()
	defer a.mu.Unlock()

	for _, ptr := range a.allocations {
		C.free(ptr)
	}
	a.allocations = a.allocations[:0]
}

// Count returns the number of tracked allocations
func (a *ArenaAllocator) Count() int {
	a.mu.Lock()
	defer a.mu.Unlock()
	return len(a.allocations)
}

// SafeCString creates a C string that will be automatically freed
// Returns the C string and a cleanup function
func SafeCString(s string) (*C.char, func()) {
	cstr := C.CString(s)
	return cstr, func() {
		C.free(unsafe.Pointer(cstr))
	}
}

// WithCString executes a function with a temporary C string
// The C string is automatically freed after the function returns
func WithCString(s string, fn func(*C.char) error) error {
	cstr := C.CString(s)
	defer C.free(unsafe.Pointer(cstr))
	return fn(cstr)
}

// WithCStrings executes a function with multiple temporary C strings
// All C strings are automatically freed after the function returns
func WithCStrings(strings []string, fn func([]*C.char) error) error {
	cstrs := make([]*C.char, len(strings))
	for i, s := range strings {
		cstrs[i] = C.CString(s)
	}
	defer func() {
		for _, cstr := range cstrs {
			C.free(unsafe.Pointer(cstr))
		}
	}()
	return fn(cstrs)
}

// CopyBytes copies a Go byte slice to C memory
// Returns a pointer to the C memory and the size
// Caller is responsible for freeing the memory with C.free()
func CopyBytes(data []byte) (unsafe.Pointer, C.size_t) {
	if len(data) == 0 {
		return nil, 0
	}

	size := C.size_t(len(data))
	ptr := C.malloc(size)
	if ptr == nil {
		return nil, 0
	}

	// Copy data to C memory
	cBytes := (*[1 << 30]byte)(ptr)[:len(data):len(data)]
	copy(cBytes, data)

	return ptr, size
}

// Note: CValue freeing is handled internally by the Rust FFI layer.
// Individual FFI calls manage their own CValue allocations.
