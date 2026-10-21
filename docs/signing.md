# CLI and URL signing

[Documentation index](README.md)

CLI commands run from the repository root, load only that directory's `.env`, and use the
same `SIGNING_KEY`, `PIPELINE_VERSION`, and visibility policy as the server.
They do not initialize libvips processing, but the executable still needs its
native libraries to load. Run without arguments to start the HTTP server.

## Commands

| Command | Result |
|---|---|
| `pixerve version FILE [prefix]` | JSON containing SHA-256 and a suggested source key |
| `pixerve url PATH PARAMS` | Canonical unsigned URL with pipeline version |
| `pixerve asset PATH PARAMS` | Stable signed URL for a public content-hashed source |
| `pixerve sign PATH PARAMS [ttl]` | Signed URL expiring after TTL seconds; default 86400 |
| `pixerve sign-until PATH PARAMS EXPIRY` | Signed URL using an absolute future Unix timestamp |

`PARAMS` uses comma-separated `key=value` fields, such as `p=card,f=webp`, rather
than URL query syntax. Pass an empty string for raw/static/OG examples. Ordinary
nonempty transform parameters default to WebP in the CLI unless `f` is set;
HTTP transforms without an explicit format instead negotiate from `Accept`.
No command uploads, renames, or copies your source files.

```sh
./target/release/pixerve url photos/example.jpg 'p=article,f=webp'
./target/release/pixerve sign photos/example.jpg 'w=640,q=75,f=webp' 900
./target/release/pixerve sign-until photos/example.jpg 'p=card,f=jpeg' 2000000000
./target/release/pixerve sign static/logo.png '' 900
./target/release/pixerve sign og/photos/example.jpg '' 900
```

The absolute expiry must still be in the future when the command is run. CLI
results are relative URLs; prepend your image service hostname in the CMS.
The `url` command is rejected in private mode. In public signed mode, an unsigned
ordinary transform URL from `url` will still be rejected by the server.

## Choose the access policy

| Server configuration | Ordinary transforms | Raw/static/OG | Shared HTTP caching |
|---|---|---|---|
| Public, no signing key | Unsigned, open limits | Public; OG also rate-limited | Public |
| Public, signing key | Valid `asig` or expiring signature | Public without a signature | Public |
| Private, signing key | Valid expiring signature | Valid expiring signature | `private, no-store` |

Public raw/static/OG routes remain public even when `SIGNING_KEY` is set. For
confidential images, use private mode and a separate source/hostname as appropriate.
Private mode keeps origin result caching, but checks authentication before each
origin cache lookup and disallows public asset signatures.

## Stable public assets

```sh
./target/release/pixerve version ./photo.jpg photos
./target/release/pixerve asset 'photos/photo-<full-sha256>.jpg' 'p=card,f=webp'
```

Upload the exact bytes under the returned `key` before using the URL. The name
has the shape `<stem>-<64 lowercase SHA-256 hex characters>.<extension>`.
Running `version` on an already hashed name replaces the existing hash suffix
rather than appending another one.

`asset` requires a signing key, public visibility, a fully hashed source key,
and fixed output (explicit format, raw, static, or dedicated OG). It returns
`pv` and `asig` without expiry. A cold source read verifies the hash before
caching. The signature authorizes that source/transform/pipeline combination;
changing any of them invalidates it.

Generate URLs once at publication and reuse them. Store source keys and variant
URLs in the application. Do not generate a new timestamp during every render.
Keep sources, pipeline deployments, and keys available for the lifetime of URLs;
see [pipeline migration](../CDN.md#pipeline-upgrades-and-retention).

## Expiring URLs

`sign` and `sign-until` produce `sig` and `exp`. `exp` is integer Unix seconds;
verification rejects it once it is less than the server clock. Keep signing
and serving clocks synchronized.

Public caching may serve a cached public response after its signature expires
at the origin. Expiry is therefore not a confidentiality or edge revocation
mechanism in public mode. Use private mode for time-limited private access.
Revoking a URL cannot recall copies already downloaded by a client.

## Signature interoperability

All MACs are HMAC-SHA256, encoded as lowercase hexadecimal. Normalize only
present transform fields, in this exact order:

```text
w,h,q,bl,bg,f,fit,fo
```

Join fields as `key=value` with commas and no spaces. Expand presets and apply
explicit overrides first. Do not add omitted HTTP defaults for signing. The
cache key separately normalizes defaults, which is not the signing payload.

For expiring URLs, sign this UTF-8 payload:

```text
<source-key>|<normalized-transforms>|<expiry>
```

For stable public assets, sign:

```text
pixerve-public-asset-v1|<source-key>|<normalized-transforms>|<pipeline-version>
```

The `pixerve-public-asset-v1` domain separates public asset signatures from other
HMAC uses. Leading `/` and route prefixes `/og/` / `/static/` are not part of
the signed source key. OG uses the expanded OG preset; raw/static use
empty transform normalization. A card WebP preset normalizes to:

```text
w=640,h=360,q=75,f=webp,fit=cover
```

Example server-side JavaScript for that exact public card preset:

```js
import { createHmac } from 'node:crypto';

function signPublicCard(key, secret, pipeline = '1') {
  const normalized = 'w=640,h=360,q=75,f=webp,fit=cover';
  const payload = `pixerve-public-asset-v1|${key}|${normalized}|${pipeline}`;
  return createHmac('sha256', secret).update(payload, 'utf8').digest('hex');
}
```

Use the CLI as the reference implementation for numeric formatting and presets.
Keep signing secrets on the server. Signature and bearer/origin token checks use
MAC verification rather than direct secret string comparison.

When upgrading from pre-Pixerve builds, regenerate stored public asset URLs for
the Pixerve signing domain. The expiring `sig`/`exp` payload is unchanged.

Legacy `s`/`e` aliases are accepted in query/path tokens. Query auth overrides
path-token auth; canonical output uses `sig`/`exp`. Combining `asig` with expiring
signature fields returns `400`. URL authentication occurs before redirects and
result-cache lookup.
