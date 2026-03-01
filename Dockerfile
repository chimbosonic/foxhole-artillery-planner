# -- Stage 1: Build backend + frontend WASM --
FROM rust:1.93-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli@0.7.3 --no-confirm --locked

WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY crates crates

# Build backend (release)
RUN cargo build -p foxhole-backend --release

# Build frontend WASM (release)
RUN cd crates/frontend && dx build --release --platform web

# -- Stage 2: Minimal runtime --
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Backend binary
COPY --from=builder /app/target/release/foxhole-backend .

# Game data assets (maps JSON, weapons JSON, map images)
COPY assets assets

# Frontend build output → dist/
COPY --from=builder /app/target/dx/foxhole-frontend/release/web/public dist

# Database volume
RUN mkdir -p data

ENV PORT=3000
ENV CORS_ORIGIN=https://arty.dp42.dev
ENV RUST_LOG=foxhole_backend=info
EXPOSE 3000

CMD ["./foxhole-backend"]
