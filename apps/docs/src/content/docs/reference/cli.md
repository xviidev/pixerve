---
title: "CLI and signing protocol"
description: "CLI commands and the HMAC protocol used by the JavaScript SDK."
---

The installed executable also provides URL and signing commands. Run `pixerve`
from a directory containing its `.env` configuration, or use `./pixerve.exe`
from the Windows package directory. Commands use the same `SIGNING_KEY`,
`PIPELINE_VERSION`, and visibility policy as the server.

With Docker, prefix commands with `docker exec pixerve`, for example:

```sh
docker exec pixerve pixerve url photos/example.jpg 'p=card,f=webp'
```

Signing creates a URL without reading or transforming the image. The `version`
command reads a file to calculate its hash; that file must be accessible where
the CLI runs. Run the executable without arguments to start the HTTP server.

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
pixerve url photos/example.jpg 'p=article,f=webp'
pixerve sign photos/example.jpg 'w=640,q=75,f=webp' 900
pixerve sign-until photos/example.jpg 'p=card,f=jpeg' 2000000000
pixerve sign static/logo.png '' 900
pixerve sign og/photos/example.jpg '' 900
```

The absolute expiry must still be in the future when the command is run. CLI
results are relative URLs; prepend your image service hostname in the CMS.
The `url` command is rejected in private mode. In public signed mode, an unsigned
ordinary transform URL from `url` will still be rejected by the server.

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

For TypeScript, use the [SDK signer](../sdk/#createsigneroptions) instead of
reimplementing the protocol. For another language, compare generated signatures
against CLI output. Keep signing keys on the server.

Legacy `s`/`e` aliases are accepted in query/path tokens. Query auth overrides
path-token auth; canonical output uses `sig`/`exp`. Combining `asig` with expiring
signature fields returns `400`. URL authentication occurs before redirects and
result-cache lookup.
