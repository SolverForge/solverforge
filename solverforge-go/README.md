# SolverForge Go Bindings

Go language bindings for the SolverForge constraint solver library.

## Status

✅ **Phase 1 Complete** - Foundation layer implemented
- ✅ FFI layer with C-compatible exports
- ✅ Bridge pattern following LanguageBridge trait
- ✅ Handle-based object registry
- ✅ Basic type conversions (CValue ↔ Value)
- ✅ Comprehensive test coverage (17 Rust + 6 Go tests)

✅ **Phase 2 Complete** - Type system implemented
- ✅ Go Value types (NullValue, BoolValue, IntValue, FloatValue, StringValue, ArrayValue, ObjectValue, ObjectRefValue)
- ✅ Go object and function registries with thread-safe operations
- ✅ Reflection-based Go → Value conversion (ToValue)
- ✅ Value → Go conversion (FromValue)
- ✅ Memory management utilities (finalizers, string pools, arena allocators)
- ✅ Comprehensive test coverage (69 tests passing)

**Next Steps:**
- Phase 3: Domain model generation from struct tags
- Phase 4: Complete LanguageBridge implementation
- Phase 5: Score types
- Phase 6: Constraint streams API

## Architecture

```
Go Code → CGO → C FFI (Rust) → solverforge-core
```

### Components

**Rust FFI Layer** (`src/`):
- `ffi.rs` - C-compatible function exports
- `bridge.rs` - GoBridge implementing LanguageBridge
- `conversions.rs` - CValue struct for FFI-safe values
- `registry.rs` - Handle registry for Go objects
- `errors.rs` - FFI error handling

**Go Package** (`go/solverforge/`):
- `bridge_cgo.go` - CGO bindings to Rust
- `handle.go` - ObjectHandle and FunctionHandle types
- `errors.go` - Error types
- `value.go` - Value types and type system
- `registry.go` - Object and function registries
- `reflect.go` - Reflection-based type conversions
- `memory.go` - Memory management utilities
- `bridge_test.go`, `value_test.go`, `registry_test.go`, `reflect_test.go`, `memory_test.go` - Test suites

## Building

### Prerequisites

- Rust 1.70+ (for workspace features)
- Go 1.21+ (for generics and modern features)
- GCC or Clang (for CGO)

### Build Commands

```bash
# From repository root
make build-go       # Build both Rust FFI and Go package
make test-go        # Run all tests (Rust + Go)
make fmt-go         # Format Go code
make clean-go       # Clean build artifacts
```

### Manual Build

```bash
# Build Rust FFI layer
cd solverforge-go
cargo build --release

# Build Go package
cd go
go build ./...

# Run Go tests (requires LD_LIBRARY_PATH)
cd go
LD_LIBRARY_PATH=../../../target/release:$LD_LIBRARY_PATH go test -v ./...
```

## Usage (Planned)

```go
package main

import "github.com/solverforge/solverforge-go/solverforge"

type Lesson struct {
    ID       string     `solverforge:"planning_id" json:"id"`
    Subject  string     `json:"subject"`
    Timeslot *Timeslot  `solverforge:"planning_variable,value_range=timeslots"`
}

type Timeslot struct {
    ID   string `solverforge:"planning_id" json:"id"`
    Time string `json:"time"`
}

type Timetable struct {
    Timeslots []*Timeslot              `solverforge:"value_range_provider=timeslots"`
    Lessons   []*Lesson                `solverforge:"planning_entity_collection"`
    Score     *solverforge.HardSoftScore `solverforge:"planning_score"`
}

func init() {
    solverforge.RegisterPlanningEntity(&Lesson{})
    solverforge.RegisterPlanningSolution(&Timetable{})
}

func main() {
    dm, _ := solverforge.BuildDomainModel(&Timetable{})
    solver, _ := solverforge.NewSolver(dm)
    defer solver.Close()

    problem := &Timetable{
        Timeslots: []*Timeslot{{ID: "1", Time: "9am"}},
        Lessons:   []*Lesson{{ID: "1", Subject: "Math"}},
    }

    solution, _ := solver.Solve(problem)
    println(solution.Score.ToShortString())
}
```

## Design Principles

### Memory Safety

Go bindings follow strict memory safety rules:
- **ID-based registry**: Go objects stored in Go-side registry, only IDs passed to Rust
- **No direct pointer sharing**: Complies with CGO pointer rules
- **Explicit cleanup**: Bridge cleanup via `defer` pattern
- **Finalizers**: Automatic cleanup when GC collects unused handles

### CGO Pointer Rules Compliance

```go
// ✅ CORRECT: Pass numeric ID only
goRefID := uint64(123)
rustHandle := bridge.RegisterObject(goRefID)

// ❌ WRONG: Don't pass Go pointers containing Go pointers to C
// This violates CGO rules and would cause runtime panics
```

### Thread Safety

- All bridge operations are thread-safe for concurrent goroutines
- Rust uses `Arc<Mutex<...>>` for shared state
- Go uses `sync.RWMutex` and `sync.Map`
- Safe for use with `go test -race`

## Project Structure

```
solverforge-go/
├── Cargo.toml          # Rust FFI crate config (staticlib + cdylib)
├── build.rs            # Generates C headers via cbindgen
├── cbindgen.toml       # C header generation config
├── src/                # Rust FFI implementation
│   ├── lib.rs
│   ├── ffi.rs          # ~150 lines - FFI exports
│   ├── bridge.rs       # ~250 lines - GoBridge
│   ├── conversions.rs  # ~400 lines - CValue types
│   ├── registry.rs     # ~200 lines - Handle registry
│   └── errors.rs       # ~100 lines - Error handling
└── go/
    ├── go.mod          # Go module definition
    └── solverforge/    # Go package
        ├── bridge_cgo.go   # CGO wrapper
        ├── handle.go       # Handle types
        ├── errors.go       # Error types
        └── bridge_test.go  # Test suite
```

## Development

### Running Tests

```bash
# Run all tests
make test-go

# Run only Rust FFI tests
cd solverforge-go && cargo test --lib

# Run only Go tests
cd solverforge-go/go
LD_LIBRARY_PATH=../../../target/release:$LD_LIBRARY_PATH go test -v ./...

# Run with race detector
cd solverforge-go/go
LD_LIBRARY_PATH=../../../target/release:$LD_LIBRARY_PATH go test -race ./...
```

### Formatting

```bash
# Format Rust code
cargo fmt --all

# Format Go code
cd solverforge-go/go && go fmt ./...

# Or from root
make fmt-go
```

### Debugging

Enable debug logging:
```bash
RUST_LOG=debug cargo test -p solverforge-go
```

Check CGO compilation:
```bash
cd solverforge-go/go
CGO_CFLAGS="-g" go build -x ./...
```

## Testing

### Test Coverage

**Rust FFI Layer** (17 tests):
- Bridge lifecycle (creation, cleanup)
- Object registration and handle generation
- Error propagation across FFI boundary
- Null pointer safety
- Handle registry operations
- Value type conversions

**Go Package** (6 tests):
- Bridge creation and cleanup
- Object registration with unique handles
- Multiple object management
- Handle validation
- Error type functionality
- Post-cleanup error handling

### Test Execution

All tests use the release build of the Rust library for consistent behavior:
```bash
make test-go
# Builds release lib, then runs:
# - 17 Rust unit tests
# - 6 Go integration tests
```

## Implementation Phases

Based on the [implementation plan](/home/pvd/.claude/plans/wise-brewing-squid.md):

- [x] **Phase 1: Foundation** (~1 week) - COMPLETE
  - Workspace structure
  - Basic FFI exports (bridge_new/free)
  - CValue struct for primitives
  - Go CGO wrapper
  - Basic testing

- [ ] **Phase 2: Type System** (~1.5 weeks)
  - Full CValue implementation (arrays, objects)
  - Reflection-based Go → Value conversion
  - Value → Go conversion
  - Memory management with finalizers

- [ ] **Phase 3: Domain Model** (~1.5 weeks)
  - Struct tag parsing
  - FieldType extraction
  - Annotation parsing
  - DomainClass builder

- [ ] **Phase 4: LanguageBridge** (~1.5 weeks)
  - Complete GoBridge implementation
  - All LanguageBridge trait methods
  - Go callbacks

- [ ] **Phase 5: Score Types** (~0.5 weeks)
  - Score interface
  - SimpleScore, HardSoftScore implementations
  - Arithmetic operations

- [ ] **Phase 6: Constraint Streams** (~1.5 weeks)
  - ConstraintFactory
  - Stream operations
  - Joiners

- [ ] **Phase 7: Integration & Testing** (~1.5 weeks)
  - Example applications
  - Integration tests
  - Performance benchmarks

- [ ] **Phase 8: Polish & Release** (~1 week)
  - Documentation
  - CI/CD
  - Release preparation

**Estimated Total**: 8-10 weeks

## References

- [Implementation Plan](/home/pvd/.claude/plans/wise-brewing-squid.md)
- [Python Bindings](../solverforge-python/) - Similar pattern for reference
- [Core Library](../solverforge-core/) - LanguageBridge trait definition
- [MockBridge](../solverforge-core/src/bridge.rs) - Reference implementation

## License

Apache-2.0
