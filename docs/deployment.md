# Deployment and migration

[Documentation index](README.md)

## Native process

Build with `cargo build --release --locked`, install compatible native runtime
libraries, and configure a process supervisor to run the executable with a fixed
working directory and environment. The process binds `0.0.0.0:$PORT`; the listen
address is not configurable. Put it behind a TLS-terminating reverse proxy and
restrict direct origin access as required by your deployment.

Make `SOURCE_DIR` / `STATIC_DIR` and the watermark readable. Give the process
write access to a dedicated disk cache directory. Run each instance with its own
cache directory. Keep credentials and `.env` out of the public repository.
`Ctrl+C` triggers graceful shutdown; Unix also handles `SIGTERM`. Allow shutdown
time for active source reads and blocking transforms to finish.

## Docker

Run from the standalone Pixerve repository root:

```sh
docker build -t pixerve:local .
docker run --rm --name pixerve \
  -p 127.0.0.1:3000:3000 \
  --env-file .env \
  -e SOURCE_DIR=/app/static \
  -e STATIC_DIR=/app/static \
  -e OG_WATERMARK=/app/assets/og_wm.png \
  -v pixerve-cache:/app/.cache \
  pixerve:local
```

This first example explicitly selects bundled local samples. For S3, replace
that SOURCE_DIR argument with `-e SOURCE_DIR=` and fill the S3 settings in `.env`;
leaving it absent does not override the image's local-source default. For your
own local storage, mount it read-only and override the path:

```sh
docker run --rm --name pixerve \
  -p 127.0.0.1:3000:3000 \
  --env-file .env \
  -e SOURCE_DIR=/data/images \
  -e STATIC_DIR=/app/static \
  -e OG_WATERMARK=/app/assets/og_wm.png \
  -v /absolute/path/to/images:/data/images:ro \
  -v pixerve-cache:/app/.cache \
  pixerve:local
```

The runtime uses UID/GID 10001; bind-mounted cache directories must be writable
by that user. The health check polls `/health`; unlike the application, this
configured external check does generate periodic idle requests. The image runs
the `pixerve` executable. No prebuilt registry image is currently documented as published.
Docker/Linux validation remains pending. The [distribution guide](distribution.md)
describes the supplied CI workflow and Windows portable packager.

## Proxy and CDN

1. Route the image hostname to the proxy and terminate HTTPS there.
2. Preserve the complete encoded path/query and conditional request headers.
3. Configure cache behavior from [CDN delivery](../CDN.md), including automatic
   format variants if you expose them.
4. If `ORIGIN_TOKEN` is set, inject the matching `X-Img-Origin-Token` at the trusted
   proxy/CDN. Do not send that secret to the browser.
5. Enable `TRUST_PROXY=true` only when client IP headers are replaced by a trusted
   proxy and clients cannot bypass it. Otherwise retain the peer-IP default.
6. Configure an independent metrics Bearer token and keep `/metrics` uncached.

`ORIGIN_TOKEN` protects image handlers, not `/health` or the independently
authenticated `/metrics`. Network policy can restrict all origin endpoints.

## Scaling

Start with one active job and inspect latency, memory, queue depth, and backend
reads before raising concurrency. AVIF work and source sizes can change resource
needs substantially. Prefer a shared CDN ahead of instances; origin caches are
local, not distributed. Do not mount one writable cache volume into replicas.

The service has no automatic S3 output-variant persistence, cache purge endpoint,
or background job queue. A restart preserves usable disk results, not in-flight jobs.

## Migration from the previous Bun/Sharp service

- Keep the old origin available during validation. Test a representative set of
  source keys, formats, fits, crop positions, transparency, EXIF, and OG output.
- Existing HMAC ordering and the weak FNV ETag scheme are retained. Invalid and
  duplicate transform input now fails instead of being silently accepted.
- Canonical redirects are enabled by default. Use `CANONICAL_REDIRECTS=false`
  temporarily for clients that cannot follow redirects, or update generated URLs.
- Mutable URLs no longer receive blanket immutable caching. Purge stale CDN
  entries from the old policy and publish new hashed/pipeline URLs for browsers.
- Review signed/public route behavior: a signing key alone does not privatize
  raw/static/OG. Private mode changes all image routes to signed no-store access.
- Migrate frontend URLs to presets before enabling `PRESETS_ONLY=true`.
- Native encoder versions can produce different bytes and sizes. Validate visual
  quality and dimensions rather than requiring byte-identical Sharp output.
- Benchmark comparable workloads before claiming CPU/RAM gains. See [operations](operations.md).

For encoder, watermark, preset, key, or pipeline changes, follow the
[retention procedure](../CDN.md#pipeline-upgrades-and-retention). A single process
serves only its configured pipeline version; it has no built-in multi-version router.

For upgrades from pre-Pixerve builds, regenerate public asset URLs with the
`pixerve-public-asset-v1` signing domain. Expiring signature payloads are unchanged.
