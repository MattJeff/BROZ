# ============================================================================
# LiveRelay SFU — Multi-stage Docker build
# ============================================================================
# Stage 1: Build the Rust binary
# Stage 2: Minimal runtime image
#
# Usage:
#   docker build -t liverelay .
#   docker run -p 8080:8080 -p 3478:3478/udp --env-file .env liverelay
# ============================================================================

# ── Stage 1: Builder ────────────────────────────────────────────────────────

FROM rust:1.91-bookworm AS builder

# Install build dependencies for webrtc-rs (needs OpenSSL headers).
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependency builds — copy only manifests first.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true

# Now copy the full source and build for real.
COPY . .
# Touch main.rs to force recompilation with real source.
RUN touch src/main.rs
RUN cargo build --release

# ── Stage 2: Runtime ────────────────────────────────────────────────────────

FROM debian:bookworm-slim AS runtime

# Install runtime dependencies.
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user.
RUN useradd -r -s /bin/false -m liverelay

WORKDIR /app

# Copy binary from builder.
COPY --from=builder /app/target/release/webrtc-sfu /app/liverelay

# Copy static files.
COPY --from=builder /app/static /app/static
COPY --from=builder /app/index.html /app/index.html

# Set ownership.
RUN chown -R liverelay:liverelay /app

USER liverelay

# ── Default environment ─────────────────────────────────────────────────────

ENV LIVERELAY_BIND_ADDR=0.0.0.0:8080
ENV LIVERELAY_LOG_LEVEL=info
ENV RUST_LOG=info

# ── Ports ────────────────────────────────────────────────────────────────────
# 8080  — HTTP/HTTPS API + signaling
# 3478  — TURN relay (UDP) — only needed if TURN_EMBEDDED=true
# 49152-65535 — TURN relay ports (UDP)

EXPOSE 8080
EXPOSE 3478/udp

# ── Health check ─────────────────────────────────────────────────────────────

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# ── Entrypoint ───────────────────────────────────────────────────────────────

ENTRYPOINT ["/app/liverelay"]
