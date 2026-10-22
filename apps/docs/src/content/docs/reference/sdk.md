---
title: JavaScript SDK
description: TypeScript interfaces for canonical URLs, expiring signatures, and public assets.
---

`@xviidev/pixerve` is an ESM package with TypeScript declarations and no runtime
dependencies. The main entry point uses browser APIs; `/server` uses Node.js
crypto and is exported only for Node.

```sh
npm install @xviidev/pixerve
```

See [CMS integration](../../guides/cms/) for usage examples.
Source, issues, and release history are maintained in
[pixerve-js](https://github.com/xviidev/pixerve-js).

## createClient(options)

```ts
import { createClient } from '@xviidev/pixerve';

const images = createClient({
  baseUrl: 'https://img.example.com',
  pipelineVersion: '1',
});
```

`baseUrl` is an HTTP(S) origin without a path, query, credentials, or fragment.
`pipelineVersion` defaults to `"1"` and must match the server's `PIPELINE_VERSION`.
It is independent of the Pixerve release number.

### url(source, transforms?)

Returns an absolute canonical URL containing `pv`. Without transforms, it serves
original bytes. Nonempty transforms default to WebP, matching the CLI. Choose
`format: 'auto'` explicitly for HTTP format negotiation.

```ts
images.url('photos/example.jpg');
images.url('photos/example.jpg', { width: 640, format: 'avif' });
images.url('static/logo.png');
images.url('og/photos/example.jpg');
```

Pass unencoded relative source keys. The SDK encodes spaces, Unicode, and URL
syntax characters. `og/`, `static/`, and `favicon.ico` take no transform options.
`preset: 'og'` is an ordinary transform without the dedicated route's watermark
or upscaling.

## Transform options

| Option | Values |
|---|---|
| `preset` | `thumbnail`, `card`, `article`, `wide`, `og` |
| `width`, `height` | Integer 1–2400 |
| `quality` | Integer 1–100 |
| `format` | `webp`, `avif`, `jpeg`, `png`, `auto` |
| `fit` | `cover`, `contain`, `fill`, `inside`, `outside` |
| `focus` | `center`, `top`, `bottom`, `left`, `right`, corner names such as `top_left`, `entropy`, `attention` |
| `blur` | Finite number 0.3–100 |
| `background` | 3, 4, 6, or 8 hex digits, without `#` |

Explicit options override presets. Unknown options and invalid values throw
`TypeError`. Deployment limits may reject otherwise valid values. The SDK does
not contact storage or inspect image dimensions.

## createSigner(options)

```ts
import { createSigner } from '@xviidev/pixerve/server';

const images = createSigner({
  baseUrl: 'https://img.example.com',
  signingKey: process.env.PIXERVE_SIGNING_KEY!,
  pipelineVersion: '1',
  visibility: 'public',
});
```

The signer requires a nonempty key matching the server. `visibility` defaults to
`public`; use `private` for a private deployment. This disables `asset()` but
does not change server configuration.

### sign(source, options?)

```ts
images.sign('photo.jpg', {
  transforms: { preset: 'card' },
  expiresIn: 3600,
});
```

Returns a canonical URL with `pv`, `sig`, and `exp`. Set either `expiresIn`
(positive integer seconds, default 86400) or `expiresAt` (future Unix seconds).
The SDK handles `og/` and `static/` route prefixes in the signing payload.

### asset(source, transforms?)

Returns a canonical URL with `pv` and stable `asig`. Requires public visibility,
a full source hash in the filename, and a fixed format. Raw/static bytes and
the dedicated OG route also qualify. Invalid combinations throw `TypeError`.
The image server verifies the source hash when it reads the file.

## Canonical and signing order

URL tokens use `f,w,h,q,bl,bg,fit,fo`. Signature normalization uses
`w,h,q,bl,bg,f,fit,fo`. Presets are expanded first. Omitted rendering defaults
are not added to the signature. These rules match the Pixerve CLI.
