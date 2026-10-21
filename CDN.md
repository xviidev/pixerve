# Pixerve: CDN and frontend delivery

[Documentation index](docs/README.md)

Cloudflare is the primary deployment target for the included templates.
[Cloudflare Cache Rules](deploy/cloudflare-cache-rules.json), Fastly snippets,
and a Prometheus example are configuration templates, not applied account changes.
Replace `img.example.com` and merge into your existing configuration.

## Publish a public image

1. Finish editing/compressing the source, then compute its content-based key:

   ```sh
   ./target/release/pixerve version ./photo.jpg photos
   ```

   The JSON contains `sha256` and `key`, such as
   `photos/photo-<full-64-character-sha256>.jpg`. The command only suggests a key.
   Upload the exact bytes to that S3 key, or copy them under the equivalent
   relative path in `SOURCE_DIR`. Re-versioning a hashed filename is idempotent.

2. With a server-side signing key, generate and store fixed-format variants:

   ```sh
   ./target/release/pixerve asset 'photos/photo-<full-sha256>.jpg' 'p=card,f=webp'
   ./target/release/pixerve asset 'photos/photo-<full-sha256>.jpg' 'p=card,f=avif'
   ./target/release/pixerve asset 'photos/photo-<full-sha256>.jpg' 'p=card,f=jpeg'
   ```

   Replace the hash placeholder. A result has this shape:

   ```text
   /tr:f-webp,w-640,h-360,q-75,fit-cover/photos/photo-<hash>.jpg?pv=1&asig=<mac>
   ```

   Prepend your image hostname. Generate URLs once at publication and reuse them;
   do not create a different expiry/signature on every page render.

3. `asig` is a non-expiring HMAC for public assets, bound to source key, transform,
   and pipeline version. It requires a full source hash and fixed output.
   Automatic output and private mode reject it. A cold source read verifies the
   hash; a mismatch returns `409` with `no-store`.

4. With an empty signing key, use `pixerve url KEY 'p=card,f=webp'` instead.
   Open-mode limits still apply. When replacing an image, publish a new hashed
   key and new URLs rather than overwriting a referenced source.

See [CLI and signing](docs/signing.md) for protocol details. A public signature
restricts transformation generation; it does not make the source confidential.

## Cache policy

| Response | Default Cache-Control | Vary |
|---|---|---|
| Public, full source hash + matching pv + fixed output | `public, max-age=31536000, s-maxage=31536000, immutable` | None |
| Other public image | `public, max-age=<up to 300>, s-maxage=<up to 86400>, stale-while-revalidate=60, stale-if-error=86400` | Accept only for auto transforms |
| Private image | `private, no-store` | Accept only for auto transforms |
| Application error, canonical redirect, health, metrics | `no-store` | Not needed |

Immutable output includes explicit encoded formats, raw/static bytes, and the
dedicated OG JPEG. It additionally requires public visibility, a complete
verified source hash, and `pv` matching `PIPELINE_VERSION`. A hashed source
without `pv`, or with automatic output, remains mutable.

Mutable freshness is capped by remaining origin result lifetime. Serving an old
RAM/disk result does not grant another full day of CDN freshness. Disk promotion
keeps its remaining TTL. Backend failures use 502/504 so stale-if-error can help
where the CDN supports it; a missing object remains 404. Application errors are
not stored. CDN stale behavior depends on provider settings.

## Canonical URLs and cache identity

With `CANONICAL_REDIRECTS=true`, query transforms, presets, and parameter ordering
redirect to canonical URLs using uncached 308 responses. Authentication is checked
before redirect. Tracking/unknown query fields are stripped; duplicates are
rejected. A CMS should directly use CLI-generated URLs.

Origin result keys normalize render defaults and include actual output format,
source/route identity, and pipeline namespace. Authentication is checked before
origin lookup. At the CDN, preserve the **complete URL and query**, including
`sig`, `exp`, `asig`, and `pv`. Do not use Ignore Query String or remove auth
fields to increase hit ratio.

Automatic output includes `Vary: Accept`. Configure the CDN to support that
variation; the header alone does not prove correct edge cache separation.
Explicit formats avoid browser Accept fragmentation entirely.

## Pipeline upgrades and retention

Raise `PIPELINE_VERSION` when changing native encoder behavior, watermark,
or preset/render semantics, then generate new application URLs. The process only
serves its configured pipeline. Authenticated requests for an unavailable version
return 410; public asset signatures with mismatched versions fail with 403.

Keep old deployments/routes serving old versions while their URLs remain in use,
or migrate every application reference before retiring them. Source objects and
signing keys need matching retention. Public signing key rotation currently
requires an old verifier/deployment or regenerated stored URLs; there is no
built-in keyring or multi-pipeline router.

The origin namespace includes pipeline, source identity, static root, and
watermark hash. Changing it avoids reusing old origin output, but does not change
a CDN/browser key by itself. A new `pv` or source URL is still required.

During migration from blanket immutable headers, purge affected CDN entries.
A CDN purge cannot evict browser copies; new versioned URLs cause fresh requests.
Changing public content to private cannot revoke previously downloaded copies.

## Presets and responsive images

See the [preset reference](docs/api.md#presets). Use thumbnail/card as 320w/640w
variants for a cropped card; use article/wide as 960w/1280w for original-aspect
article images. Do not mix differing crop ratios within one srcset. Only advertise
actual output widths; ordinary transformations do not enlarge small sources.

`PRESETS_ONLY=true` limits normalized transform options to presets, with alternate
formats allowed. Enable it after migrating application URLs. Raw/static remain
available. `p=og` supplies geometry only; for a watermarked OG image use:

```sh
./target/release/pixerve asset 'og/photos/photo-<full-sha256>.jpg' ''
```

Example article markup; replace placeholders with generated fixed-format URLs:

```html
<picture>
  <source type="image/avif" srcset="{{avif_960}} 960w, {{avif_1280}} 1280w"
          sizes="(max-width: 960px) 100vw, 960px">
  <source type="image/webp" srcset="{{webp_960}} 960w, {{webp_1280}} 1280w"
          sizes="(max-width: 960px) 100vw, 960px">
  <img src="{{jpeg_960}}" srcset="{{jpeg_960}} 960w, {{jpeg_1280}} 1280w"
       sizes="(max-width: 960px) 100vw, 960px" width="{{width}}" height="{{height}}"
       alt="{{description}}" decoding="async" fetchpriority="high">
</picture>
```

Set sizes to your layout and dimensions to the real aspect ratio. Do not lazy-load
the principal hero image. For images below the fold, use `loading="lazy"` and omit
`fetchpriority="high"`. Avoid another CDN image re-encoding layer when Pixerve
already produces the desired format.

## Cloudflare

The [Cache Rules template](deploy/cloudflare-cache-rules.json) contains two ordered
rules for the image hostname:

1. Allow GET/HEAD caching using explicit origin cache policy, bypass when no
   cache permission exists, respect browser TTL, and normalize AVIF/WebP/JPEG
   Accept variations. Unknown Vary fields default to bypass.
2. Bypass `/health`, `/metrics`, and requests carrying Authorization.

Replace the placeholder hostname and merge with the zone's existing rules.
Review order and old overrides instead of replacing the whole zone ruleset.
Do not add a TTL override that defeats private/no-store.
See [Cloudflare Cache Rules settings](https://developers.cloudflare.com/cache/how-to/cache-rules/settings/).

The template uses modern configurable Vary behavior, not just legacy Vary for
Images. Accept normalization preserves q=0; the origin also skips explicitly
rejected AVIF/WebP types. See [Cloudflare Vary](https://developers.cloudflare.com/cache/concepts/vary/).

Use a cookie-free image hostname and preserve ETag. Enable Tiered Cache according
to your account/deployment. To restrict image origin requests, configure
`ORIGIN_TOKEN` and inject the same `X-Img-Origin-Token` through trusted origin
request settings. Never put that secret into HTML or browser requests.
Restrict the origin network and replace IP headers before enabling TRUST_PROXY.

Test the merged rules against public mutable, public immutable, private, and
automatic-format cases before switching production traffic. No live Cloudflare
configuration or hit-ratio validation has been performed in this checkout.

## Private content

`CACHE_VISIBILITY=private` requires a signing key and verifies `sig`/`exp` for
transforms, raw, static, and OG, including origin cache hits. It rejects `asig`
and returns `private, no-store`. Use a separate source/hostname where useful.

```sh
./target/release/pixerve sign photos/private.jpg 'p=article,f=webp' 900
```

Shared private caching needs access verification before every edge hit, which
these templates do not implement. Public legacy expiry limits origin access,
not the lifetime of an already public cached copy. Use stable assets for
long-lived public delivery and private mode for expiring confidential access.

## Verify delivery

From `pixerve/` after deployment:

```sh
node scripts/check-cdn.mjs 'https://img.example.com/your-canonical-image-url'
```

The read-only script makes three GETs and reports status, bytes, MIME type,
policy, Age, provider cache status, and timing. Canonical signature query values
are redacted. Check separate automatic-format requests for AVIF, WebP, JPEG,
and q=0 cases. Repeated edge HITs are desirable but not guaranteed after eviction
or across different points of presence.

Use Cloudflare Cache Analytics for edge hit/byte ratios. Origin metrics never
see responses finished at the edge. Cached `X-Img-Cache` and `Server-Timing`
describe the original origin response; inspect `CF-Cache-Status` for the latest
edge request. See [operations](docs/operations.md) for metrics and PromQL.

## Fastly reference

Use the separate [recv](deploy/fastly-recv.vcl) and
[fetch](deploy/fastly-fetch.vcl) snippets in their corresponding VCL stages.
Recv normalizes Accept with `accept.media_lookup`, preserves the URL/query, and
passes administrative/authenticated requests. Fetch passes private/no-store,
Set-Cookie, redirects, and errors instead of forcing a public TTL.

Remove older snippets that force Surrogate-Control or alter cache-key fields.
Configure shielding through backend settings. Keep the fetch snippet out of
recv. See [Fastly Accept](https://www.fastly.com/documentation/reference/http/http-headers/Accept/).
