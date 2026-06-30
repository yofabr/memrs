# syntax=docker/dockerfile:1
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# ---- memrs-core (server) dependency cache ----
COPY memrs-core/Cargo.toml memrs-core/Cargo.lock memrs-core/
RUN mkdir -p memrs-core/src && echo "fn main() {}" > memrs-core/src/main.rs
RUN cargo build --release --manifest-path memrs-core/Cargo.toml

# ---- memrs-cli dependency cache ----
COPY memrs-cli/Cargo.toml memrs-cli/
RUN mkdir -p memrs-cli/src && echo "fn main() {}" > memrs-cli/src/main.rs
RUN cargo build --release --manifest-path memrs-cli/Cargo.toml

# ---- Build with real source ----
COPY memrs-core/src memrs-core/src
COPY memrs-cli/src memrs-cli/src
RUN cargo build --release --manifest-path memrs-core/Cargo.toml
RUN cargo build --release --manifest-path memrs-cli/Cargo.toml

# ---- Runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/memrs-core/target/release/memrs /usr/local/bin/memrs
COPY --from=builder /app/memrs-cli/target/release/memrs-cli /usr/local/bin/memrs-cli
WORKDIR /data
EXPOSE 7898
CMD ["memrs"]
