# syntax=docker/dockerfile:1.3-labs
#
# References
# ----------
# https://docs.docker.com/reference/cli/docker/buildx/build/#allow
# https://docs.docker.com/reference/dockerfile/#run---security
FROM  nixpkgs/cachix-flakes AS nix-build
WORKDIR /usr/src/app
COPY flake.lock flake.nix Cargo.lock Cargo.toml rust-toolchain .
COPY src src
RUN rm src/bin/redeem.rs
COPY abi abi
COPY templates templates
RUN --security=insecure nix build --sandbox --show-trace

FROM scratch as nix-build-output
COPY --from=nix-build /usr/src/app/result/bin/teleport .


FROM rust:1.79.0 AS chef
RUN cargo install cargo-chef
WORKDIR /usr/src/app

FROM chef AS planner
RUN apt-get update && apt-get install -y \
            libssl-dev \
            pkg-config \
        && rm -rf /var/lib/apt/lists/*
COPY Cargo.lock Cargo.toml teleport.env .
COPY src src
RUN rm src/bin/redeem.rs
COPY abi abi
COPY templates templates
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS cargo-build
COPY --from=planner /usr/src/app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY Cargo.lock Cargo.toml teleport.env .
COPY src src
RUN rm src/bin/redeem.rs
COPY abi abi
COPY templates templates
RUN cargo build --release


FROM gramineproject/gramine:1.7-jammy AS builder

ARG nix=1
ARG sgx=1
ENV NIX ${nix}
ENV SGX ${sgx}
ENV RA_TYPE dcap

RUN apt-get update && apt-get install -y \
            build-essential \
            jq \
            libclang-dev \
        && rm -rf /var/lib/apt/lists/*

WORKDIR /workdir

COPY --from=nix-build /usr/src/app/result/bin/teleport result/bin/teleport
COPY --from=cargo-build /usr/src/app/target/release/teleport target/release/teleport

# Make and sign the gramine manifest
RUN gramine-sgx-gen-private-key
COPY exex.manifest.template teleport.env Makefile ./
RUN make

CMD [ "gramine-sgx-sigstruct-view exex.sig" ]
