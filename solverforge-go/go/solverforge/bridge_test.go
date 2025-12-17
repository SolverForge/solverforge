package solverforge

import (
	"testing"
)

func TestCBridgeCreation(t *testing.T) {
	bridge, err := newCBridge()
	if err != nil {
		t.Fatalf("failed to create bridge: %v", err)
	}
	defer bridge.Free()

	if bridge.ptr == nil {
		t.Fatal("bridge pointer is nil")
	}
}

func TestRegisterObject(t *testing.T) {
	bridge, err := newCBridge()
	if err != nil {
		t.Fatalf("failed to create bridge: %v", err)
	}
	defer bridge.Free()

	// Register an object with Go ref ID 100
	handle, err := bridge.RegisterObject(100)
	if err != nil {
		t.Fatalf("failed to register object: %v", err)
	}

	if handle == 0 {
		t.Fatal("invalid handle returned")
	}

	// Verify object count
	if bridge.ObjectCount() != 1 {
		t.Errorf("expected object count 1, got %d", bridge.ObjectCount())
	}
}

func TestMultipleObjects(t *testing.T) {
	bridge, err := newCBridge()
	if err != nil {
		t.Fatalf("failed to create bridge: %v", err)
	}
	defer bridge.Free()

	// Register multiple objects
	handle1, err := bridge.RegisterObject(100)
	if err != nil {
		t.Fatalf("failed to register object 1: %v", err)
	}

	handle2, err := bridge.RegisterObject(200)
	if err != nil {
		t.Fatalf("failed to register object 2: %v", err)
	}

	// Handles should be different
	if handle1 == handle2 {
		t.Error("handles should be unique")
	}

	// Verify object count
	if bridge.ObjectCount() != 2 {
		t.Errorf("expected object count 2, got %d", bridge.ObjectCount())
	}
}

func TestBridgeAfterFree(t *testing.T) {
	bridge, err := newCBridge()
	if err != nil {
		t.Fatalf("failed to create bridge: %v", err)
	}

	bridge.Free()

	// Operations after free should fail
	_, err = bridge.RegisterObject(100)
	if err == nil {
		t.Error("expected error after bridge is freed")
	}

	// Count operations should return 0
	if bridge.ObjectCount() != 0 {
		t.Error("object count should be 0 after free")
	}
}

func TestHandleTypes(t *testing.T) {
	h := NewObjectHandle(42)
	if !h.IsValid() {
		t.Error("handle should be valid")
	}
	if h.ID() != 42 {
		t.Errorf("expected ID 42, got %d", h.ID())
	}

	invalid := ObjectHandle(0)
	if invalid.IsValid() {
		t.Error("zero handle should be invalid")
	}
}

func TestErrorTypes(t *testing.T) {
	err := NewError(ErrorCodeBridge, "test error")
	if err.Code != ErrorCodeBridge {
		t.Errorf("expected error code Bridge, got %v", err.Code)
	}
	if err.Message != "test error" {
		t.Errorf("expected message 'test error', got %s", err.Message)
	}

	errStr := err.Error()
	if errStr != "[Bridge] test error" {
		t.Errorf("unexpected error string: %s", errStr)
	}
}
