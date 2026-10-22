---
title: Deployment
description: Run Pixerve behind a proxy with local or S3 storage.
---

Run the server with a fixed working directory and environment. It listens on
`0.0.0.0:$PORT` and expects HTTPS to terminate at a reverse proxy or CDN.

## Docker

Run the published image with your configuration:

```sh
docker run -d --name pixerve \
  -p 127.0.0.1:3000:3000 \
  --env-file .env \
  -v pixerve-cache:/app/.cache \
  ghcr.io/xviidev/pixerve:v0.1.2
```

The image defaults to bundled sources at `/app/static`. For S3, explicitly set
`-e SOURCE_DIR=` and supply bucket credentials. For local files, mount a source
directory read-only and set `SOURCE_DIR` to its container path.

The image runs as UID/GID 10001. Bind-mounted cache directories must be writable
by that user. Keep a separate writable cache for every instance.

Use a versioned tag so updates are intentional. See
[installation](../../start/installation/) for a sample setup and package downloads.

## Native process

For the Windows portable package, keep its native libraries beside the executable.
Use a process supervisor to start the installed executable, keeping source
directories and the watermark readable and the cache directory writable.
`Ctrl+C` and Unix `SIGTERM` initiate graceful shutdown. Allow active source reads
and transforms to finish before forcibly stopping the process.

## Reverse proxy and CDN

Preserve encoded paths, complete queries, `Accept`, and conditional request
headers. Apply the [delivery policy](../delivery/) to cache only public responses.

Set `ORIGIN_TOKEN` and have the proxy inject the matching `X-Img-Origin-Token`
header if direct image-origin requests should be rejected. Keep this token
between the proxy and origin. It does not authenticate `/health` or `/metrics`;
metrics uses its own Bearer token.

Enable `TRUST_PROXY=true` only if the proxy replaces client IP headers and clients
cannot bypass it. The server then uses `CF-Connecting-IP`, followed by `X-Real-IP`.
Otherwise it uses the peer IP.

## Capacity

Begin with the default single active worker. Measure queue depth, source latency,
encoding latency, and memory before raising concurrency. A CDN shares public
results across instances; RAM and disk caches remain local to each process.
The server has no distributed cache or background source polling.

For encoder, preset, watermark, or signing-key changes, follow
[pipeline retention](../delivery/#pipeline-upgrades-and-retention). A process
serves only its configured pipeline version.
