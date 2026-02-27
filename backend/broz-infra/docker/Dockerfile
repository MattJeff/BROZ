# Multi-stage Dockerfile for all BROZ microservices
# Usage: docker build --build-arg SERVICE=broz-auth -t broz-auth .

ARG SERVICE=broz-auth

# ---- Builder Stage ----
FROM rust:1.91-slim-bookworm AS builder

ARG SERVICE

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY broz-shared/ broz-shared/

# Copy all service Cargo.toml for dependency caching
COPY broz-auth/Cargo.toml broz-auth/Cargo.toml
COPY broz-user/Cargo.toml broz-user/Cargo.toml
COPY broz-gateway/Cargo.toml broz-gateway/Cargo.toml
COPY broz-matching/Cargo.toml broz-matching/Cargo.toml
COPY broz-messaging/Cargo.toml broz-messaging/Cargo.toml
COPY broz-notification/Cargo.toml broz-notification/Cargo.toml
COPY broz-moderation/Cargo.toml broz-moderation/Cargo.toml
COPY broz-analytics/Cargo.toml broz-analytics/Cargo.toml

# Create dummy source files for dependency caching
RUN for dir in broz-auth broz-user broz-gateway broz-matching broz-messaging broz-notification broz-moderation broz-analytics; do \
    mkdir -p $dir/src && echo "fn main() {}" > $dir/src/main.rs; \
    done

# Build dependencies (cached layer)
RUN cargo build --release --package ${SERVICE} 2>/dev/null || true

# Copy actual source code
COPY . .

# Touch source files to force rebuild
RUN find . -name "*.rs" -path "*/src/*" -exec touch {} +

# Build the target service
RUN cargo build --release --package ${SERVICE}

# Ensure migrations dir exists (even if empty) for COPY in runtime stage
RUN mkdir -p /app/${SERVICE}/migrations

# ---- Runtime Stage ----
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

ARG SERVICE

WORKDIR /app

COPY --from=builder /app/target/release/${SERVICE} /app/service

# Copy migrations (dir always exists, may be empty for services without migrations)
COPY --from=builder /app/${SERVICE}/migrations /app/migrations

ENV BROZ_ENV=production
ENV RUST_LOG=info

EXPOSE 3000

CMD ["/app/service"]
