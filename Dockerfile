# ── Build stage ──
FROM rust:1-trixie AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev g++ && \
    rm -rf /var/lib/apt/lists/*

COPY app/Cargo.toml app/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY app/src ./src
COPY app/build.rs ./
COPY app/migrations ./migrations
COPY app/.sqlx ./.sqlx

ENV SQLX_OFFLINE=true
RUN touch src/main.rs && cargo build --release

# ── Runtime stage ──
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/otter ./otter
COPY --from=builder /app/migrations ./migrations

EXPOSE 3000

CMD ["./otter"]
