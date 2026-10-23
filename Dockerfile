FROM ubuntu:24.04 AS development
ARG RUST_VERSION=1.90.0
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*
ENV CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup
ENV PATH="/opt/cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup.sh \
    && sh /tmp/rustup.sh -y --profile minimal --default-toolchain ${RUST_VERSION} \
    && rustup component add rustfmt clippy \
    && rm /tmp/rustup.sh
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
RUN cargo fetch --locked

FROM development AS builder
RUN cargo build --release --locked

FROM ubuntu:24.04 AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl jq \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 tutorial
WORKDIR /app
COPY --from=builder /app/target/release/http-402-rust /usr/local/bin/http-402-rust
COPY scripts/demo.sh /app/scripts/demo.sh
USER tutorial
ENV BIND_ADDR=0.0.0.0:3000
EXPOSE 3000
CMD ["http-402-rust"]
