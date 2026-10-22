---
title: Static files and social images
description: Serve static files and generate social previews with a watermark.
---

## Serve static files

`/static/<key>` serves files from `STATIC_DIR`, independently of the main image
source. This route is useful for site icons, fixed social previews, and other
files that do not need image processing.

For example, with `STATIC_DIR=/data/static`, a file at
`/data/static/site-card.png` is available at:

```text
https://img.example.com/static/site-card.png
```

Pixerve returns the original bytes. Transform parameters do not change the
response. In private mode, static files require an expiring signed URL.

The `/favicon.ico` route serves Pixerve's built-in icon. To serve a site-specific
icon, place it in `STATIC_DIR` and reference `/static/favicon.ico` in the site's
HTML.

## Generate a social preview

`/og/<key>` reads an image from the main source and produces a 1200 × 630 JPEG
with a watermark:

```text
https://img.example.com/og/photos/example.jpg
```

The image is cropped to fill the canvas using entropy-based crop selection.
This route can enlarge small sources. Query transform parameters do not change
its output dimensions or format.

### Configure the watermark

Set `OG_WATERMARK` to a readable image file and restart Pixerve:

```dotenv
OG_WATERMARK=/data/social-overlay.png
```

Pixerve composites the overlay at the top-left corner at its original size.
A transparent 1200 × 630 PNG allows a logo or text to be positioned anywhere
within the social card. The overlay must fit inside the output canvas.
For Docker, mount the overlay file read-only at the configured container path.

Watermark changes affect generated images. Follow the
[pipeline upgrade procedure](../delivery/#pipeline-upgrades-and-retention)
when updating a watermark used by published URLs.

## Add social metadata

For a generated preview:

```html
<meta property="og:image" content="https://img.example.com/og/photos/example.jpg">
<meta property="og:image:type" content="image/jpeg">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="Article preview">
<meta name="twitter:card" content="summary_large_image">
```

For a fixed preview, use `/static/site-card.png` and set the MIME type and
dimensions to match that file. Public social previews must be accessible to the
platform fetching them.

## Error responses

Failed image requests return a PNG showing an error message and HTTP status.
The response retains the error status and uses `Cache-Control: no-store`.
`X-Img-Error` contains a more specific diagnostic message.

`HEAD` returns headers without an image body. `/health` and `/metrics` retain
their operational response formats. See the [HTTP reference](../../reference/http/#status-codes)
for status codes and [operations](../operations/#troubleshooting) for diagnosis.
