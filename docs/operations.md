# Operations, troubleshooting, and benchmarks

[Documentation index](README.md)

## Health and metrics

`GET /health` returns `{"ok":true}` once initialization finishes. It is a liveness
check, not an S3 availability or codec readiness probe. Health and metrics use
`Cache-Control: no-store` and do not count themselves in image request metrics.

Set `METRICS_TOKEN` to a secret of at least 16 characters, then request:

```sh
curl -H "Authorization: Bearer $METRICS_TOKEN" http://127.0.0.1:3000/metrics
```

Disabled or unauthorized metrics return `404`. The token is independent of the
URL signing key and origin-access token. The sample
[Prometheus configuration](../deploy/prometheus.yml) reads credentials from a
separate mounted file. Replace its hostname and token-file path for your system.

## Metric reference

All names below are prefixed with `img_`. Counters have `_total` suffixes and use
fixed names without source-key, user, IP, or URL labels.

| Metric suffix | Type | Meaning |
|---|---|---|
| `requests_total` | counter | Image handler responses, including errors and redirects |
| `redirects_total` | counter | Canonical `308` responses |
| `client_errors_total`, `server_errors_total` | counter | 4xx / 5xx responses |
| `response_bytes_total` | counter | Declared response body bytes |
| `ram_hits_total`, `disk_hits_total` | counter | Origin result cache hits |
| `shared_responses_total` | counter | Requests sharing another result lookup/job |
| `cache_misses_total` | counter | Jobs needing a source read after disk lookup |
| `ram_hit_bytes_total`, `disk_hit_bytes_total` | counter | Bytes attributed to cache paths |
| `shared_bytes_total`, `miss_bytes_total` | counter | Bytes attributed to shared/miss responses |
| `source_reads_total`, `source_bytes_total` | counter | Source read attempts / successful input bytes |
| `source_errors_total` | counter | Read, timeout, or source-hash verification errors |
| `transforms_total`, `transform_errors_total` | counter | Native transforms attempted / failed |
| `ram_cache_bytes`, `ram_cache_entries` | gauge | Weighted RAM cache size / entries |
| `active_jobs`, `queued_jobs` | gauge | Current work and admitted waiters |
| `request_duration_seconds` | histogram | Image handler latency |
| `process_resident_memory_bytes` | gauge | Linux RSS, when `/proc/self/status` is readable |

Histogram buckets are 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10,
and +Inf seconds, with standard `_bucket`, `_count`, and `_sum` series.
Duration measures the handler, not complete client transfer time. Byte metrics
use `Content-Length`, not actual socket delivery; HEAD/304 count zero body bytes.
Some framework/error bodies may not yet have a declared length at observation.

RSS is sampled only when metrics are requested; there is no application idle
polling loop. CPU/cgroup usage and Windows RSS require OS/container monitoring.
Missing platform metrics are omitted, not reported as zero.

### PromQL examples

Aggregate handler p95:

```promql
histogram_quantile(0.95, sum by (le) (rate(img_request_duration_seconds_bucket[5m])))
```

Aggregate 5xx fraction:

```promql
sum(rate(img_server_errors_total[5m]))
/ clamp_min(sum(rate(img_requests_total[5m])), 0.000001)
```

Origin result reuse ratio, including coalesced requests:

```promql
(
  sum(rate(img_ram_hits_total[5m]))
  + sum(rate(img_disk_hits_total[5m]))
  + sum(rate(img_shared_responses_total[5m]))
)
/
clamp_min(
  sum(rate(img_ram_hits_total[5m]))
  + sum(rate(img_disk_hits_total[5m]))
  + sum(rate(img_shared_responses_total[5m]))
  + sum(rate(img_cache_misses_total[5m])),
  0.000001
)
```

Use corresponding byte counters for a byte reuse ratio. Origin reuse excludes
redirects/auth failures and is different from CDN hit ratio. Observe the latter
in CDN analytics because edge hits never reach the origin. Counter snapshots
are approximate during concurrent requests and reset on process restart.

## Inspect delivery

Run from the repository root against a deployed public canonical image URL:

```sh
node scripts/check-cdn.mjs 'https://img.example.com/your-canonical-image-url'
```

The script makes three GETs and prints status, bytes, type, cache policy, Age,
Cloudflare/Fastly status, origin cache status, and timing. Canonical `sig` and
`asig` query values are redacted. It does not change CDN settings or declare that
every repeated request must be a HIT. Test automatic format Accept cases
separately. Browser/edge-cached `X-Img-Cache` and `Server-Timing` describe the
original origin response, not the latest edge request.

## Troubleshooting

| Symptom | Check |
|---|---|
| Native library/DLL load error | Matching SDK architecture, `VIPS_LIB_DIR` during build, DLL/runtime library path |
| Startup fails on assets | Working directory, `STATIC_DIR`, required watermark file, service-user permissions |
| Local source unavailable | Correct key/root, existing file, root containment, permissions |
| `403` | Correct key, exact normalized params, clock/expiry, matching pipeline, correct origin header |
| Signed raw/static/OG still public | Public mode intentionally allows these; use private mode for restricted content |
| `308` before the image | Expected canonicalization; use CLI output directly or follow Location |
| `409` | Uploaded bytes differ from the complete filename hash; rehash and publish a new key |
| `410` | Deployment does not serve that `pv`; restore its route or migrate references |
| `413` | Encoded source size, decoded input size, effective output size, open policy, parser bounds |
| Smoke test loses HEIF/AVIF support after loading Sharp | Preserve the caller environment before importing Sharp; its bundled libvips sets `VIPSHOME`, which must not redirect Pixerve to Sharp's build prefix |
| AVIF returns `422` / unknown `.avif` buffer format | Use the explicit AV1 encoder build; on Ubuntu install libheif-plugin-aomenc and libheif-plugin-dav1d, then run the codec preflight |
| JPEG logs ignored trellis/deringing/scan options | Use the portable encoder options; these flags require mozjpeg and are not supported by standard distro JPEG builds |
| `422` or empty/failed output | Corrupt source, unsupported codec, native encoder options, watermark dimensions |
| `429` | Open limit is requests per `OPEN_WINDOW`, not requests per second; check trusted IP forwarding |
| `502` / `504` | Source credentials, endpoint, region, permissions, network, fixed source timeout |
| `503` under bursts | Request/job limits and queue saturation; inspect source/encode latency before raising limits |
| Unexpected CDN MISS/BYPASS | Canonical URL, complete cache key, explicit public policy, CDN rules, Accept variants |
| High idle RSS after traffic | Native allocator/codec retention and OS cache; RAM cache size is not RSS |
| Disk cache write warning | Volume permissions/free space and one-process ownership; result can still be served |

## Reproducible performance comparisons

No comparative benchmark result is currently claimed. Use equal CPU/memory
limits, source fixtures, native codec versions, output dimensions, and quality
when comparing deployments. Record source/output sizes and image quality along
with throughput; different encoders may not emit identical bytes.

Measure these workloads separately:

1. Idle after startup, then idle after traffic and RAM idle expiry.
2. Cold result: new key/variant with source read and encoding.
3. Warm RAM hit, then disk hit after restarting the process.
4. JPEG/WebP/AVIF, cover/smart crop, alpha, and OG watermark work.
5. Many concurrent requests for one variant versus many unique variants.
6. Source failures and saturation, including 429/503 frequency.

Collect CPU time per request, RSS/working set, throughput, p50/p95/p99 latency,
storage reads, and output bytes. Measure the origin separately from the CDN;
an edge HIT is not evidence of faster native image encoding.
