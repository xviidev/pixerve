# Architecture

[Documentation index](README.md)

## Request lifecycle

```mermaid
flowchart TD
    A[Image request] --> B[Origin token and request admission]
    B --> C[Parse, validate, and authenticate]
    C --> D{Canonical URL?}
    D -->|No| E[308 no-store redirect]
    D -->|Yes| F[Rate and transform policy]
    F --> G{RAM result?}
    G -->|Hit| R[Conditional response and cache policy]
    G -->|Miss| H[Coalesce identical variant requests]
    H --> I[Bounded queue and worker slot]
    I --> J{Disk result?}
    J -->|Hit| R
    J -->|Miss| K[Bounded local or S3 source read]
    K --> L[Verify content hash when present]
    L --> M[Native transform or raw passthrough]
    M --> N[Persist result and populate RAM cache]
    N --> R
```

The async HTTP runtime uses one Tokio runtime thread. Blocking image operations
run outside that thread, with native image objects confined to the processing
task. libvips concurrency defaults to one; its operation cache is disabled.
Thumbnail loading reduces dimensions during decoding where supported.

## Scheduling

`MAX_REQUESTS` bounds image handlers, including waiters for identical results.
`MAX_CONCURRENT` bounds jobs through source reads, native processing, disk writes,
and result construction. `MAX_QUEUE` bounds additional admitted jobs. Queue
descriptions do not contain downloaded source buffers.

Moka coalesces requests for the same variant. A worker checks the disk cache
before storage; another source download is unnecessary for an existing result.
Native-task permits stay held until work finishes even if an HTTP client goes
away. A full request/job limit returns `503` instead of admitting unbounded work.

## Cache identity and lifetime

Result keys are SHA-256 digests of namespace, source key, normalized transforms,
actual output format, and transform/OG/static route identity. Namespace includes
pipeline version, source identity, canonical static root, and watermark hash.
Equivalent omitted/explicit render defaults share an origin cache identity.
Authentication runs separately before lookup, so cached output is not an auth bypass.

RAM weighting includes bytes and estimated entry overhead. Idle expiry can remove
RAM results before their maximum TTL. Disk writes use temporary files and rename;
an in-memory index tracks size and approximate recency. Expired entries are
removed on startup/access and capacity is enforced at startup/write. There is
no periodic filesystem sweep or source polling loop. Disk index entries are
bounded at approximately 8192; restart does not preserve previous access ordering.

Each disk directory belongs to one process. Cross-process cache coordination,
locking, and distributed invalidation are not implemented. A disk hit promoted
to RAM keeps its remaining TTL instead of starting another full TTL.

## Native pipeline

Transforms inspect input dimensions, apply EXIF orientation, resize, crop/pad,
check effective output limits, blur if requested, apply an OG watermark if needed,
convert to sRGB, and encode. JPEG flattens alpha; other outputs can preserve it.
Smart crop materializes an already reduced image to avoid rereading a sequential
decoder backwards. Encoders strip metadata; raw/static return unchanged bytes.

## Resource boundaries

Encoded source bytes, decoded input pixels, output pixels, jobs, request count,
and cache retention are bounded independently. They are not a process RSS cap.
Codecs, native pipelines, active source/output buffers, allocator retention, and
OS file cache still consume memory. RAM retention can be disabled independently
from disk caching. There is no claim of zero idle CPU or immediate RSS shrinkage.

The HTTP server provides neither TLS termination nor an upload API. CDN caching,
application authentication, source publication, and frontend variant selection
are separate integration responsibilities.
