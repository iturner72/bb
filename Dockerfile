FROM debian:bullseye as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    wget \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"

# Verify the Rust version
RUN rustc --version && cargo --version

# Install cargo-binstall
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz \
    && tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz \
    && cp cargo-binstall /root/.cargo/bin

# Install wasm-bindgen-cli with specific version
RUN cargo install wasm-bindgen-cli --version 0.2.100

# Install cargo-leptos
RUN cargo binstall cargo-leptos --version 0.2.26 -y

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

# Make an /app dir, which everything will eventually live in
RUN mkdir -p /app
WORKDIR /app
COPY . .

# Build the app
ENV LEPTOS_ENV="PROD"
RUN cargo leptos build --release -vv

# Build the download_models binary separately with ssr features
RUN cargo build --bin download_models --release --features=ssr

FROM debian:bullseye-slim as runtime
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates libssl1.1 pkg-config \
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy the server binary to the /app directory
COPY --from=builder /app/target/release/bb /app/

# Copy the download_models binary
COPY --from=builder /app/target/release/download_models /app/

# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site

# Copy Cargo.toml if it's needed at runtime
COPY --from=builder /app/Cargo.toml /app/

# Create models directory
RUN mkdir -p /app/models
COPY ./scripts/start.sh /app/start.sh
RUN chmod +x /app/start.sh

# Set any required env variables and
ENV RUST_LOG="info" \
    LEPTOS_SITE_ADDR="0.0.0.0:8080" \
    LEPTOS_SITE_ROOT="site" \
    LEPTOS_OUTPUT_NAME="bb" \
    LEPTOS_ENV="PROD"
EXPOSE 8080

# Run the server
CMD ["/app/start.sh"]
