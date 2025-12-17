package solverforge

/*
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -lsolverforge_go
#include "solverforge.h"
#include <stdlib.h>
*/
import "C"
import (
	"sync"
)

// cBridge is a low-level CGO wrapper around the Rust GoBridge
type cBridge struct {
	ptr *C.GoBridge
	mu  sync.RWMutex
}

// newCBridge creates a new cBridge
func newCBridge() (*cBridge, error) {
	ptr := C.solverforge_bridge_new()
	if ptr == nil {
		return nil, NewError(ErrorCodeBridge, "failed to create bridge")
	}
	return &cBridge{ptr: ptr}, nil
}

// Free releases the bridge resources
func (b *cBridge) Free() {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.ptr != nil {
		C.solverforge_bridge_free(b.ptr)
		b.ptr = nil
	}
}

// RegisterObject registers a Go object and returns its handle ID
func (b *cBridge) RegisterObject(goRefID uint64) (uint64, error) {
	b.mu.RLock()
	defer b.mu.RUnlock()

	if b.ptr == nil {
		return 0, NewError(ErrorCodeBridge, "bridge is closed")
	}

	var handle C.uint64_t
	var err *C.CError

	success := C.solverforge_register_object(b.ptr, C.uint64_t(goRefID), &handle, &err)
	if !success {
		if err != nil {
			defer C.solverforge_error_free(err)
			return 0, cErrorToGo(err)
		}
		return 0, NewError(ErrorCodeBridge, "register_object failed")
	}

	return uint64(handle), nil
}

// ObjectCount returns the number of registered objects (for testing)
func (b *cBridge) ObjectCount() int {
	b.mu.RLock()
	defer b.mu.RUnlock()

	if b.ptr == nil {
		return 0
	}

	count := C.solverforge_bridge_object_count(b.ptr)
	return int(count)
}

// FunctionCount returns the number of registered functions (for testing)
func (b *cBridge) FunctionCount() int {
	b.mu.RLock()
	defer b.mu.RUnlock()

	if b.ptr == nil {
		return 0
	}

	count := C.solverforge_bridge_function_count(b.ptr)
	return int(count)
}

// cErrorToGo converts a C error to a Go error
func cErrorToGo(cerr *C.CError) error {
	if cerr == nil {
		return nil
	}

	code := ErrorCode(cerr.code)
	message := C.GoString(cerr.message)

	return &SolverForgeError{
		Code:    code,
		Message: message,
	}
}
