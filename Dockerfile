# XCabinet engine service. Build: docker build -t xcabinet-engine .
# Run: docker run -p 8080:8080 xcabinet-engine   (then GET /health, POST /v1/build)
FROM rust:1.92-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p engine-api

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 engine
WORKDIR /app
COPY --from=build /src/target/release/xcabinet-engine /usr/local/bin/xcabinet-engine
COPY catalog ./catalog
ENV PORT=8080 CATALOG_DIR=/app/catalog
USER engine
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD ["/bin/sh", "-c", "exec 3<>/dev/tcp/127.0.0.1/$PORT && printf 'GET /health HTTP/1.0\\r\\n\\r\\n' >&3 && grep -q ok <&3"]
CMD ["xcabinet-engine", "serve"]
