# Start from Gramine's Ubuntu-based image with SGX support
FROM gramineproject/gramine:1.7-jammy AS builder

# Install necessary build dependencies
RUN apt-get update && apt-get install -y jq build-essential libclang-dev

WORKDIR /workdir

# Install Rust and set up the environment
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup toolchain install 1.75.0

# Generate SGX private key
RUN gramine-sgx-gen-private-key

# Install additional dependencies
RUN apt-get install -y pkg-config libssl-dev

# Build dependencies first for better caching
COPY Cargo.lock Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -r src

# Copy project files
COPY teleport.env Makefile ./
COPY src ./src
COPY abi ./abi
COPY templates ./templates

# Build the project
RUN cargo build --release

# Prepare and sign the Gramine manifest
COPY exex.manifest.template ./
RUN make SGX=1 RA_TYPE=dcap

# Default command to view the SGX sigstructure
CMD [ "gramine-sgx-sigstruct-view exex.sig" ]
