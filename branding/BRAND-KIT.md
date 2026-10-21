# Pixerve brand kit

The downloadable ZIP contains these original PNG masters:

| File | Native size | Use |
|---|---|---|
| `pixerve-logo.png` | 2172 × 724 | Transparent horizontal logo for light surfaces |
| `default-og.png` | 1730 × 909 | Default link preview on a navy background |
| `readme-cover.png` | 2172 × 724 | Light README hero/banner |
| `pixerve-icon.png` | 1254 × 1254 | Transparent app-icon/avatar master |
| `placeholder.png` | 1672 × 940 | Image-unavailable fallback |

All artwork uses the Pixerve teal/navy identity. English copy is embedded in
the opaque banner/card/placeholder images. Original raster dimensions and
transparent alpha channels are preserved; these are not SVG or ICO files.

In the source checkout, companion images live in `static/` and are served
by `/static/<filename>` when the bundled static directory is configured. The
project README already uses the cover. For exact 1200 × 630 JPEG output, the
existing `/og/default-og.png` route works when the main source contains that
file; the bundled local demo does. S3 deployments must upload it to their bucket
or use the static master URL directly. Set metadata dimensions to match the URL
you actually use.

Fallback selection is controlled by the consuming application; the placeholder
does not silently replace Pixerve's HTTP errors. The transparent OG watermark
used by the service is separate from this opaque default preview card.

Generated with the built-in image generation tool using the original Pixerve
logo as a reference. Companion prompts are included in `asset-prompts.md` in
the ZIP. Full project integration examples are in `docs/branding.md` in the
source checkout.
