# Pixerve brand images

[Documentation index](README.md)

The brand uses a teal pixel P symbol, a navy wordmark, and light or midnight-navy
surfaces. Artwork and text are in English. These are PNG raster masters, not
SVG/vector files or an ICO favicon bundle.

## Asset catalog

| Asset | Source checkout file | Native size | Purpose |
|---|---|---|---|
| Original horizontal logo | `branding/pixerve-logo.png` | 2172 × 724 | Logo lockup on light surfaces; transparent |
| Default OG | `static/default-og.png` | 1730 × 909 | Default page/link preview, opaque navy background |
| README cover | `static/readme-cover.png` | 2172 × 724 | Light hero banner, already used in the project README |
| Icon master | `static/pixerve-icon.png` | 1254 × 1254 | Avatar/app icon source; transparent |
| Placeholder | `static/placeholder.png` | 1672 × 940 | Explicit image-unavailable fallback |

The generated originals are preserved without resizing. The OG master is close
to 1.91:1; its native dimensions are not 1200 × 630. Use its actual dimensions
in metadata, or generate an exact-size response with the existing OG endpoint
as described below. Keep the icon's pixel cluster visible when cropping avatars.
Create appropriately sized exports for favicon/app-manifest requirements through
your asset pipeline rather than treating the large icon as an ICO file.

## Serve the images

With the example `STATIC_DIR=static`, Docker's bundled static directory,
or a newly built portable package, these existing routes serve the assets:

```text
/static/default-og.png
/static/readme-cover.png
/static/pixerve-icon.png
/static/placeholder.png
```

When replacing STATIC_DIR with your own directory, copy the desired files there.
Static routes do not read the S3 source bucket. Public mode allows these routes;
private mode still requires expiring signatures and uses private/no-store.

With main `SOURCE_DIR` also pointing at the bundled static directory, the
existing `/og/default-og.png` route returns a 1200 × 630 quality-70 JPEG.
It applies the configured watermark. The bundled starter watermark is transparent.
If main sources use S3, that OG route instead expects `default-og.png` in the
bucket; use the static OG master URL directly or upload the source there.

There is no new fallback route or silent image replacement. Missing source
objects still return the documented error. Applications choose when to use the
default card or placeholder.

## Open Graph metadata

Replace the example image hostname with your deployment's public HTTPS hostname:

```html
<meta property="og:title" content="Pixerve — Image processing and delivery">
<meta property="og:description" content="Fast images. Small footprint.">
<meta property="og:type" content="website">
<meta property="og:image" content="https://img.example.com/static/default-og.png">
<meta property="og:image:type" content="image/png">
<meta property="og:image:width" content="1730">
<meta property="og:image:height" content="909">
<meta property="og:image:alt" content="Pixerve: Fast images. Small footprint.">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:image" content="https://img.example.com/static/default-og.png">
<meta name="twitter:image:alt" content="Pixerve image processing and delivery">
```

For the exact-size OG endpoint, use its JPEG URL, MIME type, width 1200, and height
630 instead. Image assets do not insert metadata into a website automatically.

## Placeholder example

```html
<img src="https://img.example.com/static/placeholder.png"
     width="1672" height="940" alt="Image unavailable">
```

Your application can select this image when there is no usable source. Avoid
unconditional error handlers that repeatedly retry a failed placeholder URL.

## Packaging, provenance, and updates

Docker and the Windows packager copy `static/`, so these images are
included on the next image/ZIP build. Existing downloaded archives are unchanged.
The asset generator only recreates the neutral test PNG and transparent overlay;
it does not replace these generated brand images.

The built-in image generation tool created all companion images using the
original Pixerve logo as a reference. Prompts are recorded in
`branding/asset-prompts.md` in the source checkout. Keep filenames/versioned
publication URLs distinct when replacing artwork; follow the content-hash and
pipeline policy in [CDN delivery](../CDN.md) for long-lived cached assets.
