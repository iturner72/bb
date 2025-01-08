# Get started with a build env with Rust nightly
FROM rustlang/rust:nightly-bullseye as builder

# If you’re using stable, use this instead
# FROM rust:1.74-bullseye as builder

# Install cargo-binstall, which makes it easier to install other
# cargo extensions like cargo-leptos
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN cp cargo-binstall /usr/local/cargo/bin

# install specific version of wasm-bindgen cli
RUN cargo install wasm-bindgen-cli --version 0.2.99

# Install cargo-leptos
RUN cargo binstall cargo-leptos --version 0.2.24 -y

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

# Make an /app dir, which everything will eventually live in
RUN mkdir -p /app
WORKDIR /app
COPY . .

# Build the app
ENV LEPTOS_ENV="PROD"
RUN cargo leptos build --release -vv

FROM debian:bullseye-slim as runtime

RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates libssl1.1 pkg-config \
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# -- NB: update binary name from "leptos_start" to match your app name in Cargo.toml --
# Copy the server binary to the /app directory
COPY --from=builder /app/target/release/bb /app/bb

# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site

# Copy Cargo.toml if it’s needed at runtime
COPY --from=builder /app/Cargo.toml /app/

RUN chmod +x /app/bb

# Set any required env variables and
ENV RUST_LOG="info" \
    LEPTOS_SITE_ADDR="0.0.0.0:8080" \
    LEPTOS_SITE_ROOT="site" \
    LEPTOS_OUTPUT_NAME="bb" \
    LEPTOS_ENV="PROD"

EXPOSE 8080

# -- NB: update binary name from "leptos_start" to match your app name in Cargo.toml --
# Run the server
CMD ["/app/bb"]

