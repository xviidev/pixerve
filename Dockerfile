# Build from the Pixerve repository root:
# docker build -f Dockerfile -t pixerve .
FROM rust:1.92-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends libvips-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY assets ./assets
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends libvips42 libheif1 libaom3 libdav1d6 fonts-dejavu-core ca-certificates wget && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 pixerve && useradd --uid 10001 --gid pixerve --no-create-home pixerve
WORKDIR /app
COPY --from=build /build/target/release/pixerve /usr/local/bin/pixerve
COPY assets ./assets
COPY static ./static
RUN mkdir -p /app/.cache/images && chown -R pixerve:pixerve /app/.cache
ENV PORT=3000 SOURCE_DIR=/app/static STATIC_DIR=/app/static OG_WATERMARK=/app/assets/og_wm.png
USER pixerve
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD wget -q --spider http://127.0.0.1:${PORT}/health || exit 1
CMD ["pixerve"]
