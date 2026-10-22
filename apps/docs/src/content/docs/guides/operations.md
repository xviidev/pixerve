---
title: "Operations"
description: "Inspect cache behavior, monitor the service, and diagnose image errors."
---

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
[Prometheus configuration](https://github.com/xviidev/pixerve/blob/main/deploy/prometheus.yml) reads credentials from a
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

RSS is sampled only when metrics are requested; there is no application idle
polling loop. CPU/cgroup usage and Windows RSS require OS/container monitoring.
Missing platform metrics are omitted, not reported as zero.

### PromQL examples

Aggregate handler p95:

```text
histogram_quantile(0.95, sum by (le) (rate(img_request_duration_seconds_bucket[5m])))
```

Aggregate 5xx fraction:

```text
sum(rate(img_server_errors_total[5m]))
/ clamp_min(sum(rate(img_requests_total[5m])), 0.000001)
```

Origin result reuse ratio, including coalesced requests:

```text
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

Inspect the headers of a deployed canonical image URL:

```sh
curl -I 'https://img.example.com/your-canonical-image-url'
```

Repeat the request and inspect `Age`, `Cache-Control`, `CF-Cache-Status` or
`X-Cache`, and `X-Img-Cache`. Test automatic-format requests with different
`Accept` headers separately. Cached `X-Img-Cache` and `Server-Timing` describe
the original origin response, not the latest edge request.

## Troubleshooting

| Symptom | Check |
|---|---|
| Native library/DLL load error | Extract the complete Windows package and keep DLLs beside the executable |
| Startup fails on assets | Working directory, `STATIC_DIR`, required watermark file, service-user permissions |
| Startup cannot render error images | Use the complete distribution; source installations also require libvips SVG/text support and an available font |
| Local source unavailable | Correct key/root, existing file, root containment, permissions |
| `403` | Correct key, exact normalized params, clock/expiry, matching pipeline, correct origin header |
| Signed raw/static/OG still public | Public mode intentionally allows these; use private mode for restricted content |
| `308` before the image | Expected canonicalization; use an SDK/CLI canonical URL or follow Location |
| `409` | Uploaded bytes differ from the complete filename hash; rehash and publish a new key |
| `410` | Deployment does not serve that `pv`; restore its route or migrate references |
| `413` | Encoded source size, decoded input size, effective output size, open policy, parser bounds |
| AVIF returns `422` | Use the current Docker image or full Windows package; check source validity and the server logs |
| `422` or empty/failed output | Corrupt source, unsupported codec, native encoder options, watermark dimensions |
| `429` | Open limit is requests per `OPEN_WINDOW`, not requests per second; check trusted IP forwarding |
| `502` / `504` | Source credentials, endpoint, region, permissions, network, fixed source timeout |
| `503` under bursts | Request/job limits and queue saturation; inspect source/encode latency before raising limits |
| Unexpected CDN MISS/BYPASS | Canonical URL, complete cache key, explicit public policy, CDN rules, Accept variants |
| High idle RSS after traffic | Native allocator/codec retention and OS cache; RAM cache size is not RSS |
| Disk cache write warning | Volume permissions/free space and one-process ownership; result can still be served |

## Measure capacity

Measure capacity with representative source images, output sizes, and formats.
Keep CPU and memory limits consistent across runs. Record output sizes and image
quality alongside throughput.

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
