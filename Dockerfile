FROM gramineproject/gramine:1.6-jammy as builder

RUN apt-get update && apt-get install -y jq build-essential libclang-dev

WORKDIR /workdir

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup toolchain install 1.75.0

RUN gramine-sgx-gen-private-key

RUN apt-get install -y pkg-config libssl-dev

# Build just the dependencies (shorcut)
COPY Cargo.lock Cargo.toml ./
RUN mkdir src && touch src/lib.rs
RUN cargo build --release
RUN rm -r src

# Now add our actual source
COPY teleport.env Makefile README.md ./
COPY src ./src
COPY abi ./abi

# Build with rust
RUN cargo build --release

# Make and sign the gramine manifest
COPY exex.manifest.template ./
RUN make SGX=1 RA_TYPE=dcap

CMD [ "gramine-sgx-sigstruct-view exex.sig" ]

# FROM gramineproject/gramine:1.6-jammy as runner

# RUN apt-get update && apt-get install -y libssl-dev

# COPY --from=builder /workdir/exex.sig /
# COPY --from=builder /workdir/exex.manifest.sgx /
# COPY --from=builder /workdir/target/release/teleport /target/release/
# COPY --from=builder /workdir/teleport.env /

# CMD [ "gramine-sgx-sigstruct-view exex.sig" ]