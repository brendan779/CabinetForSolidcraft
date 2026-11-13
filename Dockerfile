# XCabinet engine service. Build: docker build -t xcabinet-engine .
# Run: docker run -p 127.0.0.1:8080:8080 -e ENGINE_TOKEN=... xcabinet-engine
FROM rust:1.92-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# cabinet-core compiles the hardware catalog in with include_str!.
COPY catalog ./catalog
RUN cargo build --release --locked -p engine-api

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 engine
WORKDIR /app
COPY --from=build /src/target/release/xcabinet-engine /usr/local/bin/xcabinet-engine
COPY catalog ./catalog
ENV PORT=8080 CATALOG_DIR=/app/catalog
USER engine
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD ["xcabinet-engine", "health"]
CMD ["xcabinet-engine", "serve"]
