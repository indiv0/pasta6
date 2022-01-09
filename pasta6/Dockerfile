FROM debian:bullseye AS rustup

WORKDIR /root

# Common packages
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
    ca-certificates curl file \
    build-essential \
    autoconf automake autotools-dev libtool xutils-dev && \
    rm -rf /var/lib/apt/lists/*

ENV SSL_VERSION=3.0.1

RUN curl https://www.openssl.org/source/openssl-$SSL_VERSION.tar.gz -O && \
    tar xvf openssl-$SSL_VERSION.tar.gz && \
    cd openssl-$SSL_VERSION && ./config && make depend && make install && \
    cd .. && rm -rf openssl-$SSL_VERSION*

ENV OPENSSL_LIB_DIR=/usr/local/ssl/lib \
    OPENSSL_INCLUDE_DIR=/usr/local/ssl/include \
    OPENSSL_STATIC=1

# Install toolchain
RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain stable -y

ENV PATH=/root/.cargo/bin:$PATH

FROM rustup AS builder

# Make a fake Rust app to keep a cached layer of compiled crates.
RUN USER=root cargo new app
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
# Needs at least a `bin.rs` file with a main function and a `lib.rs` file.
RUN mkdir src && echo "fn main() {}" > src/bin.rs && touch src/lib.rs
# Will build all dependent crates in release mode
RUN cargo build --release

# Copy the rest
COPY . .
# Build (install) the actual binaries
RUN cargo install --path .

# Runtime image
FROM debian:bullseye

# Run as "app" user
RUN useradd -ms /bin/bash app

USER app
WORKDIR /app

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /root/.cargo/bin/pasta6 /app/pasta6

# No CMD or ENTRYPOINT, see fly.toml with `cmd` override.
