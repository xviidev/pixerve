---
title: Signing and access
description: Choose public or private access and generate the appropriate signed URL.
---

Pixerve supports public images with transformation authorization and private
images that require an expiring signature on every image route.

| Configuration | Ordinary transforms | Original, static, and OG routes | HTTP caching |
|---|---|---|---|
| Public, no signing key | Unsigned, subject to open limits | Public | Public |
| Public, signing key | Stable public or expiring signature | Public | Public |
| Private, signing key | Expiring signature | Expiring signature | `private, no-store` |

Setting `SIGNING_KEY` alone does not make source files private. To restrict all
image routes, set `CACHE_VISIBILITY=private` as well.

## Expiring signatures

Use the SDK's `sign()` method or the CLI:

```sh
pixerve sign photos/example.jpg 'p=card,f=webp' 3600
```

The result is a canonical URL with `pv`, `sig`, and `exp`. The signature covers
the source key, expanded transform parameters, and expiry in Unix seconds.
Generating a signature does not read or process the source image.

Public responses may remain in browser or CDN caches after a signature expires
at the origin. For restricted content, use private mode. Expiry cannot recall
copies already downloaded by a client.

## Stable public signatures

Use `asset()` or `pixerve asset` with a content-hashed source key:

```sh
pixerve version ./photo.jpg photos
# Upload the exact file bytes using the returned key.
pixerve asset 'photos/photo-<full-sha256>.jpg' 'p=card,f=webp'
```

The `asig` has no expiry and is bound to source key, transforms, and pipeline.
It requires public mode, a full 64-character lowercase SHA-256 filename suffix,
and fixed output. Pixerve verifies the hash when reading the source.

Generate stable URLs at publication, then reuse them. Keep source objects, keys,
and pipeline deployments available while their URLs remain in use. See
[delivery](../delivery/) for caching and upgrades.

## Where to sign

Use `@xviidev/pixerve/server` in a Node.js backend, or invoke the CLI from a trusted
server process. A REST signing endpoint is not part of the current API. The main
SDK entry point creates unsigned canonical URLs only.

See [CMS integration](../cms/) for TypeScript examples and
[CLI and signing protocol](../../reference/cli/) for other languages.
