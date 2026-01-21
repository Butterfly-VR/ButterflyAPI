# AI generated Dockerfile, use with caution
FROM rust:1.92-trixie AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies first (caching optimization)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY src ./src
COPY migrations ./migrations
COPY diesel.toml ./

# Touch main.rs to ensure it rebuilds with actual code
RUN touch src/main.rs

RUN cargo install diesel_cli --no-default-features --features postgres

RUN cargo build --release

# Stage 2: Create the runtime image
FROM debian:trixie-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 1001 -g root appuser

# Copy the compiled binary from builder
COPY --from=builder /app/target/release/ButterflyAPI /app/butterfly-api

# Copy migrations for diesel
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

RUN chown -R appuser:root /app && chmod -R g=u /app

# Switch to non-root user (security best practice)
USER appuser

EXPOSE 23888

ENTRYPOINT ["/app/butterfly-api"]
