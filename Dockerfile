FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin teleport

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
RUN apt update \
    && apt install -y openssl ca-certificates \
    && apt clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*
WORKDIR /app
COPY --from=builder /app/target/release/teleport /usr/local/bin
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/teleport"]
