package solverforge

import (
	"runtime"
	"sync"
	"testing"
	"time"
)

func TestFinalizer(t *testing.T) {
	t.Run("cleanup called on Release", func(t *testing.T) {
		called := false
		f := NewFinalizer(func() {
			called = true
		})

		f.Release()

		if !called {
			t.Error("cleanup function was not called")
		}
	})

	t.Run("cleanup called only once", func(t *testing.T) {
		count := 0
		f := NewFinalizer(func() {
			count++
		})

		f.Release()
		f.Release()
		f.Release()

		if count != 1 {
			t.Errorf("cleanup called %d times, want 1", count)
		}
	})

	t.Run("cleanup called by GC", func(t *testing.T) {
		called := false
		var mu sync.Mutex

		func() {
			_ = NewFinalizer(func() {
				mu.Lock()
				called = true
				mu.Unlock()
			})
		}()

		// Force GC multiple times
		for i := 0; i < 5; i++ {
			runtime.GC()
			time.Sleep(10 * time.Millisecond)

			mu.Lock()
			wasCalled := called
			mu.Unlock()

			if wasCalled {
				return // Success
			}
		}

		t.Skip("finalizer not called by GC (this is timing-dependent)")
	})
}

func TestStringPool(t *testing.T) {
	t.Run("Get and reuse", func(t *testing.T) {
		pool := NewStringPool()
		defer pool.Release()

		cstr1 := pool.Get("hello")
		if cstr1 == nil {
			t.Fatal("Get() returned nil")
		}

		cstr2 := pool.Get("hello")
		if cstr2 == nil {
			t.Fatal("Get() returned nil")
		}

		// Should return the same pointer for the same string
		if cstr1 != cstr2 {
			t.Error("Get() returned different pointers for same string")
		}
	})

	t.Run("different strings", func(t *testing.T) {
		pool := NewStringPool()
		defer pool.Release()

		cstr1 := pool.Get("hello")
		cstr2 := pool.Get("world")

		if cstr1 == cstr2 {
			t.Error("Get() returned same pointer for different strings")
		}
	})

	t.Run("Size", func(t *testing.T) {
		pool := NewStringPool()
		defer pool.Release()

		if size := pool.Size(); size != 0 {
			t.Errorf("Size() = %d, want 0", size)
		}

		pool.Get("hello")
		pool.Get("world")
		pool.Get("hello") // Duplicate

		if size := pool.Size(); size != 2 {
			t.Errorf("Size() = %d, want 2", size)
		}
	})

	t.Run("Release", func(t *testing.T) {
		pool := NewStringPool()

		pool.Get("hello")
		pool.Get("world")

		pool.Release()

		if size := pool.Size(); size != 0 {
			t.Errorf("Size() after Release() = %d, want 0", size)
		}
	})

	t.Run("concurrent access", func(t *testing.T) {
		pool := NewStringPool()
		defer pool.Release()

		var wg sync.WaitGroup
		numGoroutines := 100

		wg.Add(numGoroutines)
		for i := 0; i < numGoroutines; i++ {
			go func(n int) {
				defer wg.Done()
				_ = pool.Get("test")
			}(i)
		}
		wg.Wait()

		// Should only have one entry since all goroutines used the same string
		if size := pool.Size(); size != 1 {
			t.Errorf("Size() = %d, want 1", size)
		}
	})
}

func TestArenaAllocator(t *testing.T) {
	t.Run("Allocate and track", func(t *testing.T) {
		arena := NewArenaAllocator()
		defer arena.Release()

		ptr := arena.Allocate(100)
		if ptr == nil {
			t.Fatal("Allocate() returned nil")
		}

		if count := arena.Count(); count != 1 {
			t.Errorf("Count() = %d, want 1", count)
		}
	})

	t.Run("AllocateString", func(t *testing.T) {
		arena := NewArenaAllocator()
		defer arena.Release()

		cstr := arena.AllocateString("hello")
		if cstr == nil {
			t.Fatal("AllocateString() returned nil")
		}

		if count := arena.Count(); count != 1 {
			t.Errorf("Count() = %d, want 1", count)
		}
	})

	t.Run("multiple allocations", func(t *testing.T) {
		arena := NewArenaAllocator()
		defer arena.Release()

		arena.Allocate(100)
		arena.Allocate(200)
		arena.AllocateString("test")

		if count := arena.Count(); count != 3 {
			t.Errorf("Count() = %d, want 3", count)
		}
	})

	t.Run("Release", func(t *testing.T) {
		arena := NewArenaAllocator()

		arena.Allocate(100)
		arena.Allocate(200)

		arena.Release()

		if count := arena.Count(); count != 0 {
			t.Errorf("Count() after Release() = %d, want 0", count)
		}
	})

	t.Run("multiple releases", func(t *testing.T) {
		arena := NewArenaAllocator()

		arena.Allocate(100)
		arena.Release()
		arena.Release() // Should not crash

		if count := arena.Count(); count != 0 {
			t.Errorf("Count() = %d, want 0", count)
		}
	})
}

func TestSafeCString(t *testing.T) {
	t.Run("creates and cleans up", func(t *testing.T) {
		cstr, cleanup := SafeCString("hello")
		if cstr == nil {
			t.Fatal("SafeCString() returned nil")
		}

		cleanup()
		// If cleanup failed, this would leak memory
	})
}

// Note: WithCString, WithCStrings, and CopyBytes tests are skipped
// as they require CGO in test files which is not well-supported.
// These functions are tested indirectly through bridge integration tests.

func TestKeepAlive(t *testing.T) {
	t.Run("does not panic", func(t *testing.T) {
		obj := "test"
		KeepAlive(obj)
		// If it doesn't panic, test passes
	})
}

func TestPin(t *testing.T) {
	t.Run("pins and unpins", func(t *testing.T) {
		obj := "test"
		unpin := Pin(obj)
		if unpin == nil {
			t.Fatal("Pin() returned nil")
		}

		unpin()
		// If it doesn't panic, test passes
	})
}
