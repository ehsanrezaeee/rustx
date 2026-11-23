# Use Docker's bundled BuildKit frontend without an extra Docker Hub fetch.
FROM rust:1.90.0-bookworm AS build
WORKDIR /src
RUN rustup target add wasm32-unknown-unknown \
    && cargo install wasm-bindgen-cli --version 0.2.100 --locked
COPY . .
# Cache compilation without depending on a host target/ or prebuilt bundle.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build -p rustx-cli --bin rustx --release --locked \
    && target/release/rustx build --target web --release --out dist \
    && cp target/release/rustx /usr/local/bin/rustx

FROM debian:bookworm-slim AS demo
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /usr/local/bin/rustx /usr/local/bin/rustx
COPY --from=build /src/dist ./dist
COPY LICENSE NOTICE.md ./
USER 10001:10001
EXPOSE 3000
HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:3000/runtime-config.json > /dev/null || exit 1
CMD ["rustx", "serve", "/app/dist", "3000", "--host", "0.0.0.0"]
