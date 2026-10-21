# HTTP API

[Documentation index](README.md)

## Routes

Image routes support `GET` and `HEAD`. Examples omit authentication parameters;
see [signing](signing.md) for when they are required.

| Route | Behavior |
|---|---|
| `/health` | JSON liveness; does not check storage or codecs |
| `/metrics` | Prometheus text; disabled by default, Bearer token when enabled |
| `/<key>` | Original bytes from the main source |
| `/<key>?w=640&f=webp` | Query transformation |
| `/<key>?p=card&f=webp` | Preset transformation |
| `/tr:f-webp,w-640,q-75/<key>` | Path-token transformation |
| `/og/<key>` | Fixed social JPEG with watermark from the main source |
| `/static/<key>` | Original bytes from local `STATIC_DIR` |

`og/`, `static/`, and `tr:` are reserved prefixes. On `/og/` and `/static/`, query
transform options do not override fixed route behavior. On `/tr:` routes,
transform settings come from path tokens; query transform fields do not override
them. Authentication and `pv` can still come from the query. Keys are relative paths.
Percent-encode special characters in URLs. Empty path components, `.` / `..`,
backslashes, colons, and control characters are rejected.

## Transform parameters

| Parameter | Accepted values | Default |
|---|---|---|
| `w` | Integer 1–2400 | Derived from aspect ratio if only `h` is given |
| `h` | Integer 1–2400 | Derived from aspect ratio if only `w` is given |
| `q` | Integer 1–100 | 95; presets override this |
| `f` | `auto`, `jpeg`, `png`, `webp`, `avif` | Automatic for transforms |
| `fit` | `cover`, `contain`, `fill`, `inside`, `outside` | `cover` |
| `fo` | Positions below, `entropy`, `attention` | `center` |
| `bl` | Finite sigma 0.3–100 | No blur |
| `bg` | 3/4/6/8 hex digits, without `#` | Transparent contain padding; white JPEG flattening |
| `p` | `thumbnail`, `card`, `article`, `wide`, `og` | No preset |

Global/open-mode limits can reject syntactically valid values. PNG uses compression
level 9; `q` does not tune PNG compression. Parameters and named values are
case-sensitive. Unknown query fields are ignored and stripped by canonicalization.
Duplicate query keys and duplicate token keys return `400`.

```text
/photos/example.jpg?w=640&h=360&fit=cover&fo=attention&f=webp&q=75
/tr:f-png,w-600,h-600,bg-ffffff,fit-contain/photos/example.jpg
/photos/example.jpg?w=960&bl=1.5&f=jpeg
```

### Resize modes

| Mode | Behavior with both dimensions |
|---|---|
| `cover` | Preserve aspect ratio, cover the box, crop |
| `contain` | Preserve aspect ratio, fit inside, pad to canvas |
| `fill` | Resize axes independently; may distort |
| `inside` | Fit inside without padding |
| `outside` | Cover the box without final cropping |

Ordinary transforms do not upscale source pixels. Cover may return a smaller
result for small sources; contain can add a larger canvas around an un-enlarged
image. A single dimension preserves aspect ratio, including in fill mode.
The dedicated OG route can upscale. EXIF orientation is applied before effective
output geometry is checked.

### Crop focus and color

Positions: `center`, `top`, `bottom`, `left`, `right`, `top_left`, `top_right`,
`bottom_left`, `bottom_right`. `entropy` and `attention` use libvips smart cropping.
Focus affects cover cropping, not general watermark placement or contain padding.

`bg=fff` is white; `bg=0000` is transparent black. Four/eight-digit forms include
alpha. JPEG flattens alpha onto the RGB background, defaulting to white.
Transforms convert to sRGB and strip metadata. Raw/static preserve original bytes
and metadata, with content type inferred from the filename.

## Presets

| Preset | Width | Height | Quality | Format | Fit / focus |
|---|---|---|---|---|---|
| `thumbnail` | 320 | 180 | 75 | WebP | cover / center |
| `card` | 640 | 360 | 75 | WebP | cover / center |
| `article` | 960 | derived | 80 | WebP | inside / center |
| `wide` | 1280 | derived | 80 | WebP | inside / center |
| `og` | 1200 | 630 | 70 | JPEG | cover / entropy |

Explicit options override preset values. `p=og` is an ordinary transform preset:
it adds no watermark and does not enable upscaling. Use `/og/<key>` for both.
`PRESETS_ONLY=true` accepts normalized preset options with a different format
allowed. It does not disable raw/static routes.

## Social images

`/og/<key>` produces a 1200 × 630, quality-70 JPEG with entropy crop and watermark.
The watermark is composited at the top-left origin at its existing dimensions;
it is not resized or repositioned. It must fit the output. A transparent
1200 × 630 overlay can position artwork within that canvas.

## Automatic formats

Missing `f` or `f=auto` prefers explicitly accepted AVIF, then WebP, then JPEG
fallback. An explicit media type with `q=0` is skipped. This is a format preference
policy, not full HTTP quality ranking or a `406` negotiator; wildcards alone do
not enable AVIF/WebP. Automatic responses include `Vary: Accept`; explicit formats,
social JPEGs, and raw/static do not. See [CDN delivery](../CDN.md).

## Canonical URLs and versions

By default, query/preset/alternative parameter order redirects with uncached
`308` to a canonical URL. Authentication is checked first. Use CLI-generated
URLs to avoid that round trip.

`pv` selects the configured pipeline. URLs without it remain mutable. A different
`pv` returns `410` after applicable authentication. Public asset signatures with
missing/wrong `pv` return `403`.

## Response headers

| Header | Meaning |
|---|---|
| `Content-Type` | Encoded format, or filename-derived raw type |
| `ETag` | Weak FNV-1a validator for output bytes |
| `Cache-Control` | Public mutable/immutable or private no-store |
| `Vary: Accept` | Automatic format transforms only |
| `X-Img-Cache` | Origin path: `RAM`, `DISK`, `MISS`, `SHARED` |
| `Server-Timing` | Origin handler duration, such as `img;dur=2.350` |
| `X-Content-Type-Options` | `nosniff` on successful images |
| `Retry-After` | `1` on application `429` / `503` |

`If-None-Match` supports lists, weak/strong comparisons, and `*`. A match returns
`304` with ETag/cache policy and no body. `HEAD` has no body but may require source
loading/encoding on a cold cache. The weak ETag is not a cryptographic fingerprint.

## Status codes

| Status | Meaning |
|---|---|
| `200` | Image, health, or authorized metrics |
| `304` | Conditional image match |
| `308` | Follow canonical `Location` |
| `400` | Invalid path/transform/query, duplicate fields, preset mismatch, or mixed signature types |
| `403` | Required signature/origin token missing, invalid, expired, or disallowed |
| `404` | Source absent, or metrics disabled/unauthorized |
| `405` | Unsupported method |
| `409` | Source does not match hash in its filename |
| `410` | Pipeline version unavailable |
| `413` | Source bytes, pixels, or transform policy exceeded |
| `422` | Corrupt/unsupported image, missing codec, or another native transform error |
| `429` | Open-mode IP window exceeded |
| `500` | Processing task failed |
| `502` | Source backend error other than absence/size rejection |
| `503` | Request/job capacity exhausted or shutdown |
| `504` | Source read exceeded 30 seconds |

Application image errors have short plain-text bodies and `no-store`. Framework
responses such as `405` may lack that header; bypass responses without explicit
cache permission at the CDN.
