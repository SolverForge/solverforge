package solverforge

// ObjectHandle is an opaque handle to a host language object
type ObjectHandle uint64

// NewObjectHandle creates a new ObjectHandle from an ID
func NewObjectHandle(id uint64) ObjectHandle {
	return ObjectHandle(id)
}

// ID returns the handle ID
func (h ObjectHandle) ID() uint64 {
	return uint64(h)
}

// IsValid returns true if the handle is non-zero
func (h ObjectHandle) IsValid() bool {
	return h != 0
}

// FunctionHandle is an opaque handle to a host language callable
type FunctionHandle uint64

// NewFunctionHandle creates a new FunctionHandle from an ID
func NewFunctionHandle(id uint64) FunctionHandle {
	return FunctionHandle(id)
}

// ID returns the handle ID
func (h FunctionHandle) ID() uint64 {
	return uint64(h)
}

// IsValid returns true if the handle is non-zero
func (h FunctionHandle) IsValid() bool {
	return h != 0
}
