---
title: CMS integration
description: Generate canonical image URLs when publishing content from your CMS.
---

Store source keys in the CMS, then use Pixerve URLs for the variants your
templates need. A source key such as `photos/example.jpg` identifies the local
file or S3 object; your image hostname points to the Pixerve deployment.

## Use image URLs

Use an image URL directly in your template:

```html
<img src="https://img.example.com/photos/example.jpg?p=card&f=webp"
     width="640" height="360" alt="Describe the image">
```

This public example follows a redirect to the canonical URL. The installed CLI
can generate that URL in advance:

```sh
pixerve url photos/example.jpg 'p=card,f=webp'
```

With Docker, use `docker exec pixerve pixerve url` followed by the same arguments.
For signed deployments, use `sign` or `asset` instead; see [signing](../signing/).

## Install the SDK

Install the JavaScript and TypeScript SDK in the CMS project:

```sh
npm install @xviidev/pixerve
```

The SDK generates canonical URLs locally without making network requests.
Use `@xviidev/pixerve` in browser or backend code and
`@xviidev/pixerve/server` for signing in Node.js.

## Public URLs

```ts
import { createClient } from '@xviidev/pixerve';

const images = createClient({
  baseUrl: 'https://img.example.com',
  pipelineVersion: '1',
});

const url = images.url('photos/example.jpg', {
  preset: 'card',
  format: 'webp',
});
```

The result is:

```text
https://img.example.com/tr:f-webp,w-640,h-360,q-75,fit-cover/photos/example.jpg?pv=1
```

`createClient` works in browser and backend code. It does not sign URLs or make
network requests. If the server has a signing key, ordinary transforms still
need a valid signature.

## Signed URLs

Create signed URLs in backend code:

```ts
import { createSigner } from '@xviidev/pixerve/server';

const images = createSigner({
  baseUrl: 'https://img.example.com',
  signingKey: process.env.PIXERVE_SIGNING_KEY!,
  pipelineVersion: '1',
});

const url = images.sign('photos/example.jpg', {
  transforms: { preset: 'card', format: 'webp' },
  expiresIn: 3600,
});
```

The key must match the server's `SIGNING_KEY`. Keep it out of client bundles and
public environment variables. For private images, configure
`CACHE_VISIBILITY=private` on Pixerve and `visibility: 'private'` on the signer.

## Publish stable public variants

For public articles, use a source key containing the full SHA-256 of the file.
The CLI's `version` command calculates a key; upload the exact file bytes under
it. Then generate each variant once:

```ts
const variants = {
  card: images.asset(sourceKey, { preset: 'card', format: 'webp' }),
  article: images.asset(sourceKey, { preset: 'article', format: 'webp' }),
};
```

Store those URLs with the media record and reuse them in templates. `asset`
produces a non-expiring public signature bound to source, transforms, and pipeline.
It requires a content-hashed source and a fixed format.

When replacing an image, publish a new source key and regenerate its URLs.
The CMS pipeline version must match the Pixerve deployment. See
[pipeline upgrades](../delivery/#pipeline-upgrades-and-retention) when upgrading
the image service or changing its watermark.

## Responsive images

Use the same crop ratio for every URL in a `srcset`. The `article` and `wide`
presets preserve source aspect ratio; `thumbnail` and `card` crop to 16:9.
Ordinary transforms do not enlarge small images, so declare actual output widths.
See the [SDK reference](../../reference/sdk/) for options and defaults.
