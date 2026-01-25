# Multi-stage build for SolverForge Employee Scheduling (Rust)
#
# Build context: rust/employee-scheduling/
# Uses published solverforge crate from crates.io

FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY static/ ./static/
COPY solver.toml ./

# Build release binary with musl target for static linking
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage - minimal Alpine image
FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy binary from builder (musl static binary)
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/employee-scheduling ./employee-scheduling

# Copy static files
COPY --from=builder /build/static/ ./static/

# Copy solver config
COPY --from=builder /build/solver.toml ./solver.toml

# Expose port 7860 (HF Spaces default)
EXPOSE 7860

# Run the application
CMD ["./employee-scheduling"]
