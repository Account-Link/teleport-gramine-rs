FROM gramineproject/gramine:1.7-jammy AS builder

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
COPY teleport.env Makefile ./
COPY src ./src
COPY abi ./abi
COPY templates ./templates

# Build with rust
RUN cargo build --release

# Make and sign the gramine manifest
COPY exex.manifest.template ./
RUN make SGX=1 RA_TYPE=dcap

CMD [ "gramine-sgx-sigstruct-view exex.sig" ]
