ARG USER=eden
ARG RUST_BUILD_PROFILE=release
ARG BUILD_DIR=/usr/build/eden

# Install the required Rust components from 1.94.0
FROM rust:1.67-bullseye AS prepare
ARG BUILD_DIR
WORKDIR ${BUILD_DIR}

RUN cargo init --vcs none
COPY rust-toolchain.toml .

RUN cargo build
RUN rm -rf src Cargo.toml

FROM prepare AS cache

# Copy the stub-crates to the crates directory along with the required files
COPY ./stub-crates ./crates
COPY ./Cargo.lock .
COPY ./Cargo.toml .

# xtask is not necessary but it is required since we included it
# as a member in the local Cargo workspace.
RUN cargo new xtask

# Compile as usual
RUN cargo build --release -p eden

# Clean up mess
RUN rm -rf ./crates

FROM cache AS compile

ARG RUST_BUILD_PROFILE
ARG BUILD_DIR

COPY . .
RUN cargo build --profile ${RUST_BUILD_PROFILE} -p eden

# This is on purpose for docker to treat as an error because it is WIP
RUN exit 3
