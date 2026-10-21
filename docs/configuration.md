# Configuration reference

[Documentation index](README.md)

Settings load at startup from the working directory's `.env` and process
environment; existing environment values win. Parent `.env` files are not loaded.
Restart after changes. Tables show code defaults; `.env.example` uses the same
`static` and `assets/og_wm.png` paths relative to the repository root.

MB settings use 1,048,576 bytes. Pixel limits use decimal pixels. Booleans use
lowercase `true` or `false`. Relative paths use the process working directory.

## Server and storage

| Variable | Default | Description |
|---|---|---|
| `PORT` | `3000` | Listen port on `0.0.0.0` |
| `SOURCE_DIR` | unset | Nonempty local root selects local mode instead of S3 |
| `S3_ENDPOINT` | required in S3 mode | Endpoint; `http://` explicitly permits HTTP |
| `S3_REGION` | `auto` | Provider region |
| `S3_BUCKET` | required in S3 mode | Single source bucket |
| `S3_ACCESS_KEY_ID` | required in S3 mode | Access key |
| `S3_ACCESS_KEY_SECRET` | required in S3 mode | Secret key |
| `STATIC_DIR` | `static` | Local `/static/` root; must exist |
| `OG_WATERMARK` | `assets/og_wm.png` | Readable watermark required at startup |
| `RUST_LOG` | `pixerve=info` | Tracing filter, e.g. `pixerve=warn` |

S3 requests use path-style addressing. The server reads objects; your application
uploads them. No arbitrary remote URL source or upload HTTP endpoint is exposed.

## Processing limits

| Variable | Default | Description |
|---|---|---|
| `MAX_CONCURRENT` | `1` | Positive active-job limit, including source reads |
| `MAX_QUEUE` | `16` | Additional waiting unique jobs; zero disables waiting |
| `MAX_REQUESTS` | `128` | Positive concurrent image request limit, including shared waiters |
| `VIPS_CONCURRENCY` | `1` | Native concurrency, clamped to 1–4 |
| `MAX_SOURCE_MB` | `20` | Positive source byte limit, including raw/static |
| `MAX_INPUT_PIXELS` | `40000000` | Positive decoded transform input pixel limit |
| `MP_LIMIT` | `12` | Positive finite output megapixel limit |

Requested width/height also have a parser ceiling of 2400 each. Raw/static do not
decode images, so only the source byte limit applies to them. The source timeout
is fixed at 30 seconds.

## Origin result caches

| Variable | Default | Description |
|---|---|---|
| `MEM_CACHE_MB` | `8` | Weighted RAM capacity; zero disables retention |
| `DISK_CACHE_MB` | `256` | Disk result capacity; zero disables disk cache |
| `DISK_CACHE_DIR` | `.cache/images` | Dedicated writable directory for one process |
| `CACHE_TTL` | `86400` | Positive result lifetime in seconds |
| `CACHE_IDLE_SECS` | `300` | Positive RAM idle expiration in seconds |

These are not RSS or total-filesystem caps. Disk entries also use a bounded memory
index. Do not share a writable cache among instances. Disk-to-RAM promotion keeps
the remaining result lifetime.

## Signing and open mode

| Variable | Default | Description |
|---|---|---|
| `SIGNING_KEY` | empty | HMAC key; empty selects unsigned open mode |
| `OPEN_MAX_W` | `1280` | Open output width ceiling |
| `OPEN_MAX_H` | `1200` | Open output height ceiling |
| `OPEN_MAX_MP` | `3` | Positive finite open output megapixel ceiling |
| `OPEN_MAX_BLUR` | `10` | Finite open blur ceiling, at least 0.3 |
| `OPEN_ALLOWED_FORMATS` | `avif,webp,jpeg` | Comma-separated open transform formats |
| `OPEN_RPS` | `5` | Positive request count per window, despite the legacy name |
| `OPEN_WINDOW` | `60` | Positive IP window length in seconds |
| `TRUST_PROXY` | `false` | Use `CF-Connecting-IP`, then `X-Real-IP`, for the client IP |

Open IP limits apply to transforms, including OG, before cache lookup. OG is
exempt from open geometry/format policy, while global image/source/job limits
remain active. Raw/static do not consume the transform IP bucket. Signed mode
does not use this IP limiter, but retains capacity/global image limits.

Trust proxy headers only after restricting origin access and having a trusted
proxy replace client-supplied values. `X-Forwarded-For` is not used for this.

## CDN and visibility

| Variable | Default | Description |
|---|---|---|
| `PIPELINE_VERSION` | `1` | Nonempty, at most 32 ASCII letters/digits or `-_.` |
| `CACHE_VISIBILITY` | `public` | `public` or `private`; private requires signing key |
| `BROWSER_CACHE_TTL` | `300` | Mutable browser freshness ceiling, seconds |
| `CDN_CACHE_TTL` | `86400` | Mutable shared-cache freshness ceiling, seconds |
| `IMMUTABLE_CACHE_TTL` | `31536000` | Public versioned fixed-output freshness, seconds |
| `STALE_WHILE_REVALIDATE` | `60` | Mutable stale revalidation allowance, seconds |
| `STALE_IF_ERROR` | `86400` | Mutable stale error allowance, seconds |
| `CANONICAL_REDIRECTS` | `true` | Uncached `308` redirects to normalized URLs |
| `PRESETS_ONLY` | `false` | Restrict ordinary transforms to normalized preset options |
| `METRICS_TOKEN` | empty | Empty disables metrics; otherwise at least 16 characters |
| `ORIGIN_TOKEN` | empty | Optional origin header secret; otherwise at least 16 characters |

The five CDN durations accept 0–31,536,000 seconds. Mutable freshness is capped
by remaining origin lifetime; browser freshness is capped by edge freshness.
Private responses always use `private, no-store`. Origin and metrics tokens are
independent of URL signing. See [signing](signing.md) and [CDN delivery](../CDN.md).

## Build environment

`VIPS_LIB_DIR` adds a native library search path through `build.rs`. On Windows,
the SDK DLL directory must also be on runtime `PATH`. See [installation](installation.md).
