# SolverForge Makefile v0.4.0
# Rust-only build system with colorized output

# ============== Colors & Symbols ==============
GREEN := \033[92m
CYAN := \033[96m
YELLOW := \033[93m
MAGENTA := \033[95m
RED := \033[91m
GRAY := \033[90m
BOLD := \033[1m
RESET := \033[0m

CHECK := ✓
CROSS := ✗
ARROW := ▸
PROGRESS := →

# ============== Project Metadata ==============
VERSION := $(shell grep -m1 '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
RUST_VERSION := 1.80+

# ============== Phony Targets ==============
.PHONY: banner help build build-release examples test test-quick test-doc test-unit test-one \
        lint fmt fmt-check clippy ci-local pre-release version bump-patch bump-minor bump-major \
        bump-dry publish-crates-dry publish-crates clean watch

# ============== Default Target ==============
.DEFAULT_GOAL := help

# ============== Banner ==============
banner:
	@printf "$(CYAN)$(BOLD)  ____        _                _____\n"
	@printf " / ___|  ___ | |_   _____ _ __|  ___|__  _ __ __ _  ___\n"
	@printf " \\___ \\\\ / _ \\\\| \\\\ \\\\ / / _ \\\\ '__| |_ / _ \\\\| '__/ _\` |/ _ \\\\\n"
	@printf "  ___) | (_) | |\\\\ V /  __/ |  |  _| (_) | | | (_| |  __/\n"
	@printf " |____/ \\\\___/|_| \\_/ \\___|_|  |_|  \\___/|_|  \\__, |\\___|\n"
	@printf "                                             |___/$(RESET)\n"
	@printf "  $(GRAY)v$(VERSION)$(RESET) $(CYAN)SolverForge Build System$(RESET)\n\n"

# ============== Build Targets ==============

build: banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║          Building Workspace          ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(ARROW) $(BOLD)Building all crates...$(RESET)\n"
	@cargo build --workspace && \
		printf "$(GREEN)$(CHECK) Build successful$(RESET)\n\n" || \
		(printf "$(RED)$(CROSS) Build failed$(RESET)\n\n" && exit 1)

build-release: banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║        Release Build                 ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(ARROW) $(BOLD)Building release binaries...$(RESET)\n"
	@cargo build --workspace --release && \
		printf "$(GREEN)$(CHECK) Release build successful$(RESET)\n\n" || \
		(printf "$(RED)$(CROSS) Release build failed$(RESET)\n\n" && exit 1)

examples: banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║        Building Examples             ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
	@for ex in nqueens employee-scheduling vehicle-routing; do \
		printf "$(PROGRESS) Building $$ex...\n"; \
		cargo build -p $$ex --quiet && \
			printf "$(GREEN)$(CHECK) Built $$ex$(RESET)\n" || \
			(printf "$(RED)$(CROSS) Failed to build $$ex$(RESET)\n" && exit 1); \
	done
	@printf "\n"

# ============== Test Targets ==============

test: banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║         Full Test Suite              ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(ARROW) $(BOLD)Running all tests...$(RESET)\n"
	@cargo test --workspace && \
		printf "\n$(GREEN)$(CHECK) All tests passed$(RESET)\n\n" || \
		(printf "\n$(RED)$(CROSS) Tests failed$(RESET)\n\n" && exit 1)

test-quick: banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║         Quick Test Suite             ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(PROGRESS) Running doctests...\n"
	@cargo test --workspace --doc --quiet && \
		printf "$(GREEN)$(CHECK) Doctests passed$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Doctests failed$(RESET)\n" && exit 1)
	@printf "$(PROGRESS) Running unit tests...\n"
	@cargo test --workspace --lib --quiet && \
		printf "$(GREEN)$(CHECK) Unit tests passed$(RESET)\n\n" || \
		(printf "$(RED)$(CROSS) Unit tests failed$(RESET)\n\n" && exit 1)

test-doc:
	@printf "$(PROGRESS) Running doctests...\n"
	@cargo test --workspace --doc && \
		printf "$(GREEN)$(CHECK) Doctests passed$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Doctests failed$(RESET)\n" && exit 1)

test-unit:
	@printf "$(PROGRESS) Running unit tests...\n"
	@cargo test --workspace --lib && \
		printf "$(GREEN)$(CHECK) Unit tests passed$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Unit tests failed$(RESET)\n" && exit 1)

test-one:
	@printf "$(PROGRESS) Running test: $(YELLOW)$(TEST)$(RESET)\n"
	@RUST_LOG=info cargo test $(TEST) -- --nocapture

# ============== Lint & Format ==============

lint: banner fmt-check clippy
	@printf "\n$(GREEN)$(BOLD)$(CHECK) All lint checks passed$(RESET)\n\n"

fmt:
	@printf "$(PROGRESS) Formatting code...\n"
	@cargo fmt --all
	@printf "$(GREEN)$(CHECK) Code formatted$(RESET)\n"

fmt-check:
	@printf "$(PROGRESS) Checking formatting...\n"
	@cargo fmt --all -- --check && \
		printf "$(GREEN)$(CHECK) Formatting valid$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Formatting issues found$(RESET)\n" && exit 1)

clippy:
	@printf "$(PROGRESS) Running clippy...\n"
	@cargo clippy --workspace -- -D warnings && \
		printf "$(GREEN)$(CHECK) Clippy passed$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Clippy warnings found$(RESET)\n" && exit 1)

# ============== CI Simulation ==============

ci-local: banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║              Local CI Simulation                         ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(ARROW) $(BOLD)Simulating GitHub Actions CI workflow locally...$(RESET)\n\n"
	@printf "$(PROGRESS) Step 1/5: Format check...\n"
	@$(MAKE) fmt-check --no-print-directory
	@printf "$(PROGRESS) Step 2/5: Build...\n"
	@cargo build --workspace --quiet && printf "$(GREEN)$(CHECK) Build passed$(RESET)\n"
	@printf "$(PROGRESS) Step 3/5: Clippy...\n"
	@$(MAKE) clippy --no-print-directory
	@printf "$(PROGRESS) Step 4/5: Doctests...\n"
	@cargo test --workspace --doc --quiet && printf "$(GREEN)$(CHECK) Doctests passed$(RESET)\n"
	@printf "$(PROGRESS) Step 5/5: Unit tests...\n"
	@cargo test --workspace --lib --quiet && printf "$(GREEN)$(CHECK) Unit tests passed$(RESET)\n"
	@printf "\n$(GREEN)$(BOLD)╔══════════════════════════════════════════════════════════╗$(RESET)\n"
	@printf "$(GREEN)$(BOLD)║              $(CHECK) CI SIMULATION PASSED                      ║$(RESET)\n"
	@printf "$(GREEN)$(BOLD)╚══════════════════════════════════════════════════════════╝$(RESET)\n\n"

# ============== Version Management ==============

version:
	@printf "$(CYAN)Current version:$(RESET) $(YELLOW)$(BOLD)$(VERSION)$(RESET)\n"

bump-patch: banner
	@printf "$(ARROW) Bumping patch version...\n"
	@npx commit-and-tag-version --release-as patch --no-verify
	@printf "$(GREEN)$(CHECK) Version bumped$(RESET)\n"

bump-minor: banner
	@printf "$(ARROW) Bumping minor version...\n"
	@npx commit-and-tag-version --release-as minor --no-verify
	@printf "$(GREEN)$(CHECK) Version bumped$(RESET)\n"

bump-major: banner
	@printf "$(ARROW) Bumping major version...\n"
	@npx commit-and-tag-version --release-as major --no-verify
	@printf "$(GREEN)$(CHECK) Version bumped$(RESET)\n"

bump-dry:
	@npx commit-and-tag-version --dry-run

# ============== Pre-Release Validation ==============

pre-release: banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║            Pre-Release Validation v$(VERSION)                  ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════════════════════════╝$(RESET)\n\n"
	@$(MAKE) fmt-check --no-print-directory
	@$(MAKE) clippy --no-print-directory
	@printf "$(PROGRESS) Running full test suite...\n"
	@cargo test --workspace --quiet && printf "$(GREEN)$(CHECK) All tests passed$(RESET)\n"
	@printf "\n$(GREEN)$(BOLD)$(CHECK) Ready for release v$(VERSION)$(RESET)\n\n"

# ============== Publishing ==============

publish-crates-dry: banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║              Dry-Run Publishing                          ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(ARROW) Publishing order: $(GRAY)core → macros → scoring → config → solver → benchmark → facade$(RESET)\n\n"
	@for crate in solverforge-core solverforge-macros solverforge-scoring solverforge-config solverforge-solver solverforge-benchmark solverforge; do \
		printf "$(PROGRESS) Dry-run: $$crate...\n"; \
		cargo publish -p $$crate --dry-run --allow-dirty || exit 1; \
	done
	@printf "\n$(GREEN)$(CHECK) All crates ready for publishing$(RESET)\n\n"

publish-crates: banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║              Publishing to crates.io                     ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(RED)$(BOLD)WARNING: This will publish version $(VERSION) to crates.io$(RESET)\n"
	@printf "$(YELLOW)Press Ctrl+C to abort, or Enter to continue...$(RESET)\n"
	@read dummy
	@printf "\n$(PROGRESS) Publishing crates in dependency order...\n\n"
	@printf "$(ARROW) [1/7] Publishing solverforge-core...\n"
	@cargo publish -p solverforge-core && printf "$(GREEN)$(CHECK) Published$(RESET)\n" || exit 1
	@printf "$(GRAY)Waiting 45s for crates.io index...$(RESET)\n" && sleep 45
	@printf "$(ARROW) [2/7] Publishing solverforge-macros...\n"
	@cargo publish -p solverforge-macros && printf "$(GREEN)$(CHECK) Published$(RESET)\n" || exit 1
	@printf "$(GRAY)Waiting 45s for crates.io index...$(RESET)\n" && sleep 45
	@printf "$(ARROW) [3/7] Publishing solverforge-scoring...\n"
	@cargo publish -p solverforge-scoring && printf "$(GREEN)$(CHECK) Published$(RESET)\n" || exit 1
	@printf "$(GRAY)Waiting 45s for crates.io index...$(RESET)\n" && sleep 45
	@printf "$(ARROW) [4/7] Publishing solverforge-config...\n"
	@cargo publish -p solverforge-config && printf "$(GREEN)$(CHECK) Published$(RESET)\n" || exit 1
	@printf "$(GRAY)Waiting 45s for crates.io index...$(RESET)\n" && sleep 45
	@printf "$(ARROW) [5/7] Publishing solverforge-solver...\n"
	@cargo publish -p solverforge-solver && printf "$(GREEN)$(CHECK) Published$(RESET)\n" || exit 1
	@printf "$(GRAY)Waiting 45s for crates.io index...$(RESET)\n" && sleep 45
	@printf "$(ARROW) [6/7] Publishing solverforge-benchmark...\n"
	@cargo publish -p solverforge-benchmark && printf "$(GREEN)$(CHECK) Published$(RESET)\n" || exit 1
	@printf "$(GRAY)Waiting 45s for crates.io index...$(RESET)\n" && sleep 45
	@printf "$(ARROW) [7/7] Publishing solverforge (facade)...\n"
	@cargo publish -p solverforge && printf "$(GREEN)$(CHECK) Published$(RESET)\n" || exit 1
	@printf "\n$(GREEN)$(BOLD)╔══════════════════════════════════════════════════════════╗$(RESET)\n"
	@printf "$(GREEN)$(BOLD)║          $(CHECK) All crates published successfully!            ║$(RESET)\n"
	@printf "$(GREEN)$(BOLD)╚══════════════════════════════════════════════════════════╝$(RESET)\n\n"

# ============== Clean ==============

clean:
	@printf "$(ARROW) Cleaning build artifacts...\n"
	@cargo clean
	@printf "$(GREEN)$(CHECK) Clean complete$(RESET)\n"

# ============== Development ==============

watch:
	@printf "$(ARROW) Watching for changes...\n"
	@cargo watch -x build

# ============== Help ==============

help: banner
	@/bin/echo -e "$(CYAN)$(BOLD)Build Commands:$(RESET)"
	@/bin/echo -e "  $(GREEN)make build$(RESET)          - Build all crates"
	@/bin/echo -e "  $(GREEN)make build-release$(RESET)  - Build in release mode"
	@/bin/echo -e "  $(GREEN)make examples$(RESET)       - Build all examples"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Test Commands:$(RESET)"
	@/bin/echo -e "  $(GREEN)make test$(RESET)           - Run all tests"
	@/bin/echo -e "  $(GREEN)make test-quick$(RESET)     - Run doctests + unit tests (fast)"
	@/bin/echo -e "  $(GREEN)make test-doc$(RESET)       - Run doctests only"
	@/bin/echo -e "  $(GREEN)make test-unit$(RESET)      - Run unit tests only"
	@/bin/echo -e "  $(GREEN)make test-one TEST=name$(RESET) - Run specific test with output"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Lint & Format:$(RESET)"
	@/bin/echo -e "  $(GREEN)make lint$(RESET)           - Run fmt-check + clippy"
	@/bin/echo -e "  $(GREEN)make fmt$(RESET)            - Format code"
	@/bin/echo -e "  $(GREEN)make fmt-check$(RESET)      - Check formatting"
	@/bin/echo -e "  $(GREEN)make clippy$(RESET)         - Run clippy lints"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)CI & Quality:$(RESET)"
	@/bin/echo -e "  $(GREEN)make ci-local$(RESET)       - $(YELLOW)$(BOLD)Simulate GitHub Actions CI locally$(RESET)"
	@/bin/echo -e "  $(GREEN)make pre-release$(RESET)    - Run all validation checks"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Version Management:$(RESET)"
	@/bin/echo -e "  $(GREEN)make version$(RESET)        - Show current version"
	@/bin/echo -e "  $(GREEN)make bump-patch$(RESET)     - Bump patch version (0.4.$(YELLOW)x$(RESET))"
	@/bin/echo -e "  $(GREEN)make bump-minor$(RESET)     - Bump minor version (0.$(YELLOW)x$(RESET).0)"
	@/bin/echo -e "  $(GREEN)make bump-major$(RESET)     - Bump major version ($(YELLOW)x$(RESET).0.0)"
	@/bin/echo -e "  $(GREEN)make bump-dry$(RESET)       - Preview version bump"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Publishing:$(RESET)"
	@/bin/echo -e "  $(GREEN)make publish-crates-dry$(RESET) - Dry-run publish to crates.io"
	@/bin/echo -e "  $(GREEN)make publish-crates$(RESET) - $(RED)$(BOLD)Publish to crates.io (all 7 crates)$(RESET)"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Other:$(RESET)"
	@/bin/echo -e "  $(GREEN)make clean$(RESET)          - Clean build artifacts"
	@/bin/echo -e "  $(GREEN)make watch$(RESET)          - Watch and rebuild on changes"
	@/bin/echo -e "  $(GREEN)make help$(RESET)           - Show this help message"
	@/bin/echo -e ""
	@/bin/echo -e "$(GRAY)Rust version required: $(RUST_VERSION)$(RESET)"
	@/bin/echo -e "$(GRAY)Current version: v$(VERSION)$(RESET)"
	@/bin/echo -e ""
