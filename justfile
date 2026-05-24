# RustCash task runner — mirrors what CI runs.
# Install just: cargo install just  OR  brew install just
#
# RULE: if you change a recipe here, keep .github/workflows/ci.yml in sync, and vice versa.

# Default: show available recipes
default:
    @just --list

# ── Formatting ─────────────────────────────────────────────────────────────────

# Format all code in place
fmt:
    cargo fmt --all

# Check formatting without modifying (what CI runs)
fmt-check:
    cargo fmt --all -- --check

# ── Linting ───────────────────────────────────────────────────────────────────

# Clippy with workspace lints — matches CI
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# ── Type checking & building ──────────────────────────────────────────────────

# Fast check (no codegen)
check:
    cargo check --workspace --all-targets

# Full build
build:
    cargo build --workspace

# Release build
build-release:
    cargo build --workspace --release

# ── Testing ───────────────────────────────────────────────────────────────────

# Run all tests
test:
    cargo test --workspace

# Run tests for a specific crate  e.g. `just test-crate core`
test-crate crate:
    cargo test -p rustcash-{{crate}}

# Run property-based tests only (proptest)
test-prop:
    cargo test --workspace prop

# ── Coverage ──────────────────────────────────────────────────────────────────

# Generate HTML coverage report (requires cargo-llvm-cov)
# Install: cargo install cargo-llvm-cov
coverage:
    cargo llvm-cov --workspace --html
    @echo "Coverage report: target/llvm-cov/html/index.html"

# Print coverage summary to stdout
coverage-summary:
    cargo llvm-cov --workspace

# ── Security & audit ─────────────────────────────────────────────────────────

# Audit dependencies for known vulnerabilities (requires cargo-audit)
# Install: cargo install cargo-audit
audit:
    cargo audit

# ── Combined ──────────────────────────────────────────────────────────────────

# Lint (what `just lint` means — fmt check + clippy)
lint: fmt-check clippy

# Full CI suite — run this before pushing
ci: lint test coverage audit

# ── Utilities ─────────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# Start the API server in development mode
serve:
    RUST_LOG=rustcash_api=debug,info cargo run -p rustcash-api

# Open generated coverage report in browser (macOS)
open-coverage: coverage
    open target/llvm-cov/html/index.html
