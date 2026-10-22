---
title: Use Pixerve
description: Resize, crop, convert, and display images using Pixerve URLs.
---

After [installation](../installation/), the sample service is available at
`http://localhost:3000`. Replace that address with your image hostname when deploying.
These examples use the public sample configuration.

## Request an image

The path identifies a source file. With local storage it is relative to
`SOURCE_DIR`; with S3 it is the object key in your bucket.

```text
/photos/example.jpg
```

Without transform parameters, Pixerve returns the original bytes.

## Resize and convert

Set `w` for the width and `f` for the output format:

```text
/photos/example.jpg?w=640&f=webp
```

Providing only one dimension preserves the source aspect ratio. Choose `jpeg`,
`png`, `webp`, or `avif`. Use `f=auto` to select a supported format from the
browser's `Accept` header.

## Crop to a fixed size

Provide both dimensions and use `fit=cover`:

```text
/photos/example.jpg?w=640&h=360&fit=cover&fo=attention&f=webp
```

`fo=attention` chooses a crop based on image content. Use `fo=center` for a centered
crop. Other resize modes include `contain` for padding and `inside` for fitting
within the requested dimensions. Ordinary transforms do not enlarge small sources.

## Use a preset

Presets cover common image sizes:

| Preset | Output |
|---|---|
| `thumbnail` | 320 × 180, cropped |
| `card` | 640 × 360, cropped |
| `article` | Up to 960 px wide, original aspect ratio |
| `wide` | Up to 1280 px wide, original aspect ratio |

```text
/photos/example.jpg?p=card&f=webp
```

Explicit parameters override preset values, such as `q=80` for a different quality.

## Display an image

```html
<img
  src="https://img.example.com/tr:f-webp,w-640,h-360,q-75,fit-cover/photos/example.jpg"
  width="640"
  height="360"
  alt="Describe the image"
>
```

Query URLs redirect to a canonical path like the one above. Generate canonical
URLs with the CLI or SDK to avoid that redirect. See [CMS integration](../../guides/cms/).

## Social images

Use `/og/photos/example.jpg` for a 1200 × 630 JPEG with the configured watermark.
Set `OG_WATERMARK` to configure the overlay. See
[static files and social images](../../guides/assets/).

## Access and caching

The public sample allows five transform requests per IP per minute. If you
configure `SIGNING_KEY`, ordinary transforms require signed URLs. For restricted
images, also enable private mode. See [signing and access](../../guides/signing/).

Public results can be cached by the browser and CDN. Use new source keys when
replacing published images; see [caching and delivery](../../guides/delivery/).
