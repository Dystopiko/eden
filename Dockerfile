ARG USER=eden
ARG RUST_BUILD_PROFILE=release
ARG BUILD_DIR=/usr/build/eden

FROM lukemathwalker/cargo-chef:0.1.77-rust-1.94.0-slim-trixie AS chef
ARG BUILD_DIR
WORKDIR ${BUILD_DIR}

FROM chef AS prepare

# Install required dependencies for compilation
# - libssl-dev: libcurl4 requires libssl
# - build-essential: `pkg-config` for openssl-sys crate
RUN apt-get update && \
    apt-get install -y \
        --no-install-recommends \
        build-essential \
        libssl-dev \
        pkg-config

FROM prepare AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM prepare AS compile

ARG RUST_BUILD_PROFILE
ARG BUILD_DIR

WORKDIR ${BUILD_DIR}

COPY --from=planner ${BUILD_DIR}/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=${BUILD_DIR}/target \
    cargo chef cook --profile ${RUST_BUILD_PROFILE} --recipe-path recipe.json

# Build eden binary
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=${BUILD_DIR}/target \
    cargo install --profile ${RUST_BUILD_PROFILE} --path crates/eden --locked --root /tmp

FROM debian:trixie-slim AS runner

ARG RUST_BUILD_PROFILE
ARG BUILD_DIR
ARG USER

# Setup unprivileged user
RUN useradd \
    --home "/dev/null" \
    --no-create-home \
    -s /bin/bash \
    ${USER}

WORKDIR /app

# Install required dependencies to run Eden
RUN apt update && apt install -y ca-certificates libcurl4

COPY --from=compile --chmod=0755 /tmp/bin/eden /app
COPY --from=compile ${BUILD_DIR}/crates/** /app

USER ${USER}

ENTRYPOINT [ "./eden" ]
STOPSIGNAL SIGTERM
