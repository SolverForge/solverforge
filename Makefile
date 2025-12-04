# SolverForge Makefile
# Manages building, testing, and running the project

JAVA_HOME := /usr/lib64/jvm/java-24-openjdk-24
PATH := $(JAVA_HOME)/bin:$(PATH)
export JAVA_HOME PATH

# Submodule paths
JAVA_SERVICE := timefold-wasm-service

.PHONY: all build test test-verbose test-rust test-java clean fmt clippy help

# Default target
all: build test

# ============== Build ==============

build: build-rust build-java

build-rust:
	cargo build --workspace

build-java:
	cd $(JAVA_SERVICE) && mvn package -DskipTests -q

build-release:
	cargo build --workspace --release

# ============== Test ==============

test: test-rust test-java

# Run all Rust tests with output
test-rust:
	RUST_LOG=info cargo test --workspace -- --nocapture

# Run Rust tests quietly (no output unless failure)
test-rust-quiet:
	cargo test --workspace

# Run specific Rust test with output
# Usage: make test-one TEST=test_name
test-one:
	RUST_LOG=info cargo test $(TEST) -- --nocapture

# Run integration tests with full output
test-integration:
	RUST_LOG=info cargo test -p solverforge-service --test solve_integration -- --nocapture
	RUST_LOG=info cargo test -p solverforge-service --test employee_scheduling_integration -- --nocapture

# Run Java tests
test-java:
	cd $(JAVA_SERVICE) && mvn test

# Run Java tests with output
test-java-verbose:
	cd $(JAVA_SERVICE) && mvn test -X

# ============== Lint & Format ==============

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace

lint: fmt-check clippy

# ============== Clean ==============

clean: clean-rust clean-java

clean-rust:
	cargo clean

clean-java:
	cd $(JAVA_SERVICE) && mvn clean -q

# ============== Development ==============

# Watch and rebuild on changes (requires cargo-watch)
watch:
	cargo watch -x build

# Run a specific integration test with debug logging
# Usage: make debug-test TEST=test_solve_simple_problem
debug-test:
	RUST_LOG=debug cargo test -p solverforge-service $(TEST) -- --nocapture

# Show solver output during tests
# Usage: make solve-test TEST=test_employee_scheduling_solve
solve-test:
	RUST_LOG=info cargo test -p solverforge-service --test employee_scheduling_integration $(TEST) -- --nocapture 2>&1 | grep -E "(Solving|score|Solution|Test)"

# Benchmark solver performance (no assertions, more moves)
# Usage: make bench
# Usage: make bench SOLVER_MODE=FULL_ASSERT  (for debug mode)
# Usage: make bench EMPLOYEE_COUNT=50 SHIFT_COUNT=500
SOLVER_MODE ?= REPRODUCIBLE
MOVE_LIMIT ?= 1000000
EMPLOYEE_COUNT ?= 5
SHIFT_COUNT ?= 10
bench:
	SOLVER_MODE=$(SOLVER_MODE) MOVE_LIMIT=$(MOVE_LIMIT) EMPLOYEE_COUNT=$(EMPLOYEE_COUNT) SHIFT_COUNT=$(SHIFT_COUNT) \
		RUST_LOG=info cargo test -p solverforge-service test_employee_scheduling_solve -- --nocapture 2>&1 \
		| grep -E "(Problem Scale|speed|Performance|Summary)"

# Benchmark presets
bench-small:
	$(MAKE) bench EMPLOYEE_COUNT=15 SHIFT_COUNT=150

bench-large:
	$(MAKE) bench EMPLOYEE_COUNT=50 SHIFT_COUNT=500

bench-xlarge:
	$(MAKE) bench EMPLOYEE_COUNT=100 SHIFT_COUNT=1000

# ============== Submodule ==============

submodule-update:
	git submodule update --init --recursive

submodule-status:
	git -C $(JAVA_SERVICE) status

submodule-diff:
	git -C $(JAVA_SERVICE) diff

submodule-push:
	git -C $(JAVA_SERVICE) push solverforge main

# ============== Help ==============

help:
	@echo "SolverForge Makefile Commands:"
	@echo ""
	@echo "Build:"
	@echo "  make build          - Build both Rust and Java"
	@echo "  make build-rust     - Build Rust workspace"
	@echo "  make build-java     - Build Java service"
	@echo "  make build-release  - Build Rust in release mode"
	@echo ""
	@echo "Test:"
	@echo "  make test           - Run all tests (Rust + Java)"
	@echo "  make test-rust      - Run Rust tests with output"
	@echo "  make test-rust-quiet- Run Rust tests quietly"
	@echo "  make test-java      - Run Java tests"
	@echo "  make test-integration - Run integration tests with output"
	@echo "  make test-one TEST=name - Run specific test with output"
	@echo "  make debug-test TEST=name - Run test with debug logging"
	@echo "  make solve-test TEST=name - Run test showing solver output"
	@echo ""
	@echo "Lint:"
	@echo "  make fmt            - Format code"
	@echo "  make clippy         - Run clippy"
	@echo "  make lint           - Run fmt-check and clippy"
	@echo ""
	@echo "Clean:"
	@echo "  make clean          - Clean all build artifacts"
	@echo ""
	@echo "Submodule:"
	@echo "  make submodule-update - Update git submodules"
	@echo "  make submodule-status - Show submodule status"
	@echo "  make submodule-push   - Push submodule to remote"
