FROM rust:1.96-trixie AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libpq-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install diesel_cli --no-default-features --features postgres

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs

RUN cargo build --release

RUN rm -rf src

COPY src ./src
COPY migrations ./migrations
COPY diesel.toml ./

RUN cargo build --release

FROM debian:trixie-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 1001 -g root appuser

COPY --from=builder /app/target/release/ButterflyAPI /app/butterfly-api

COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

RUN chown -R appuser:root /app && chmod -R g=u /app

USER appuser

EXPOSE 23888

ENTRYPOINT ["/app/butterfly-api"]
