# SolverForge Makefile
# Manages building, testing, and running the project

JAVA_HOME := /usr/lib64/jvm/java-24-openjdk-24
PATH := $(JAVA_HOME)/bin:$(PATH)
export JAVA_HOME PATH

# Submodule paths
JAVA_SERVICE := solverforge-wasm-service

.PHONY: all build test test-verbose test-rust test-java clean fmt clippy help \
        test-doc test-unit test-quick test-all test-match test-list test-count

# Default target
all: build test

# ============== Build ==============

build: build-rust build-java

build-rust:
	cargo build --workspace

build-java:
	cd $(JAVA_SERVICE) && mvn package -DskipTests -q

# Rebuild Java service and update cache (for development)
rebuild-java:
	@echo "Clearing cached JAR..."
	rm -f ~/.cache/solverforge/solverforge-wasm-service-*-runner.jar
	@echo "Building Java service..."
	cd $(JAVA_SERVICE) && mvn package -DskipTests -q
	@echo "Copying JAR to cache..."
	mkdir -p ~/.cache/solverforge
	cp $(JAVA_SERVICE)/target/solverforge-wasm-service-*-runner.jar ~/.cache/solverforge/
	@echo "Done. New JAR deployed to cache."

build-release:
	cargo build --workspace --release

# ============== Test ==============

test: test-rust test-python test-java

# Run Rust tests (excluding Python crate which needs special flags)
test-rust:
	RUST_LOG=info cargo test --workspace --exclude solverforge-python -- --nocapture

# Run Python binding tests (requires auto-initialize feature)
test-python:
	RUST_LOG=info cargo test -p solverforge-python --features auto-initialize --no-default-features -- --nocapture

# Run Rust tests quietly (no output unless failure)
test-rust-quiet:
	cargo test --workspace --exclude solverforge-python

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

# ============== Test Layers ==============
# Organized by scope: doc → unit → integration

# Doctests only - verifies documentation examples compile and work
test-doc:
	cargo test --workspace --doc --exclude solverforge-python

# Unit tests only (inline #[cfg(test)] modules, no integration tests)
test-unit:
	cargo test --workspace --lib --exclude solverforge-python

# Quick check: doctests + unit tests (fast feedback during development)
test-quick: test-doc test-unit

# Full test suite: all layers
test-all: test-doc test-unit test-integration

# Run tests matching a pattern
# Usage: make test-match PATTERN=score
PATTERN ?= ""
test-match:
	cargo test --workspace --exclude solverforge-python $(PATTERN)

# List all tests without running them
test-list:
	@cargo test --workspace --exclude solverforge-python -- --list 2>&1 | grep ': test$$' | sort

# Count tests per module
test-count:
	@cargo test --workspace --exclude solverforge-python -- --list 2>&1 | grep ': test$$' | \
		cut -d: -f1 | rev | cut -d: -f2- | rev | sort | uniq -c | sort -rn

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
	@SOLVER_MODE=$(SOLVER_MODE) MOVE_LIMIT=$(MOVE_LIMIT) EMPLOYEE_COUNT=$(EMPLOYEE_COUNT) SHIFT_COUNT=$(SHIFT_COUNT) \
		RUST_LOG=error cargo test -p solverforge-service test_employee_scheduling_solve -- --nocapture 2>&1 \
		| grep -E "^(=== (Problem Scale|Performance Stats|Summary)|Scale:|Hard Score:|Soft Score:|Time:|Skill mismatches:|Solution is|Test completed)"

# Benchmark presets
bench-small:
	$(MAKE) bench EMPLOYEE_COUNT=15 SHIFT_COUNT=150

bench-large:
	$(MAKE) bench EMPLOYEE_COUNT=50 SHIFT_COUNT=500

bench-xlarge:
	$(MAKE) bench EMPLOYEE_COUNT=100 SHIFT_COUNT=1000

# ============== Version & Release ==============

VERSION := $(shell grep -m1 '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

version:
	@echo $(VERSION)

# Bump version using commit-and-tag-version (requires npm install first)
bump-patch:
	npx commit-and-tag-version --release-as patch --no-verify

bump-minor:
	npx commit-and-tag-version --release-as minor --no-verify

bump-major:
	npx commit-and-tag-version --release-as major --no-verify

bump-dry:
	npx commit-and-tag-version --dry-run

# Pre-release validation
pre-release: fmt-check clippy test
	@echo "Pre-release checks passed for v$(VERSION)"

# ============== Publish ==============

# Dry run publishing to crates.io
publish-crates-dry:
	cargo publish -p solverforge-core --dry-run --allow-dirty
	@echo "Note: dependent crates will fail dry-run until solverforge-core is published"

# Publish to crates.io (run in order due to dependencies)
publish-crates:
	cargo publish -p solverforge-core
	@echo "Waiting for crates.io index..."
	@sleep 30
	cargo publish -p solverforge-derive
	@sleep 30
	cargo publish -p solverforge-service
	@sleep 30
	cargo publish -p solverforge

# Build Python wheels
build-wheels:
	cd solverforge-python && maturin build --release

# Publish to TestPyPI
publish-pypi-test:
	cd solverforge-python && maturin publish --repository testpypi

# Publish to PyPI
publish-pypi:
	cd solverforge-python && maturin publish

# Install Maven artifact locally
publish-maven-local:
	cd $(JAVA_SERVICE) && mvn clean install

# Deploy to Maven Central (requires credentials)
publish-maven:
	cd $(JAVA_SERVICE) && mvn clean deploy -P release

# Full release (after pre-release passes)
release: pre-release
	@echo ""
	@echo "Ready for release v$(VERSION)"
	@echo "Run these commands to publish:"
	@echo "  make publish-maven"
	@echo "  make publish-crates"
	@echo "  make publish-pypi"
	@echo ""
	@echo "Then push the tag:"
	@echo "  git push origin v$(VERSION)"

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
	@echo "  make rebuild-java   - Rebuild Java and update cache (dev)"
	@echo "  make build-release  - Build Rust in release mode"
	@echo ""
	@echo "Test:"
	@echo "  make test           - Run all tests (Rust + Python + Java)"
	@echo "  make test-rust      - Run Rust tests with output"
	@echo "  make test-python    - Run Python binding tests"
	@echo "  make test-rust-quiet- Run Rust tests quietly"
	@echo "  make test-java      - Run Java tests"
	@echo "  make test-integration - Run integration tests with output"
	@echo "  make test-one TEST=name - Run specific test with output"
	@echo "  make debug-test TEST=name - Run test with debug logging"
	@echo "  make solve-test TEST=name - Run test showing solver output"
	@echo ""
	@echo "Test Layers (fast feedback):"
	@echo "  make test-doc       - Run doctests only"
	@echo "  make test-unit      - Run unit tests only (no integration)"
	@echo "  make test-quick     - Run doctests + unit tests"
	@echo "  make test-all       - Run all test layers"
	@echo "  make test-match PATTERN=x - Run tests matching pattern"
	@echo "  make test-list      - List all tests"
	@echo "  make test-count     - Count tests per module"
	@echo ""
	@echo "Lint:"
	@echo "  make fmt            - Format code"
	@echo "  make clippy         - Run clippy"
	@echo "  make lint           - Run fmt-check and clippy"
	@echo ""
	@echo "Version & Release:"
	@echo "  make version        - Show current version"
	@echo "  make bump-patch     - Bump patch version (0.0.x)"
	@echo "  make bump-minor     - Bump minor version (0.x.0)"
	@echo "  make bump-major     - Bump major version (x.0.0)"
	@echo "  make bump-dry       - Preview version bump"
	@echo "  make pre-release    - Run all pre-release checks"
	@echo "  make release        - Validate and show publish commands"
	@echo ""
	@echo "Publish:"
	@echo "  make publish-crates-dry - Dry run crates.io publish"
	@echo "  make publish-crates - Publish to crates.io"
	@echo "  make build-wheels   - Build Python wheels"
	@echo "  make publish-pypi   - Publish to PyPI"
	@echo "  make publish-maven  - Deploy to Maven Central"
	@echo ""
	@echo "Clean:"
	@echo "  make clean          - Clean all build artifacts"
	@echo ""
	@echo "Submodule:"
	@echo "  make submodule-update - Update git submodules"
	@echo "  make submodule-status - Show submodule status"
	@echo "  make submodule-push   - Push submodule to remote"
