---
title: Pixerve
description: Install Pixerve and serve resized images from your own storage.
---

Pixerve serves images from a local directory or an S3-compatible bucket. Request
a size and format in the URL; Pixerve processes the image with libvips and caches
the result in RAM and on disk.

```text
https://img.example.com/tr:f-webp,w-640,q-75/photos/example.jpg
```

Use it for thumbnails, article images, responsive variants, and watermarked
social cards. Supported output formats are JPEG, PNG, WebP, and AVIF when the
native codecs are available.

## Install and try it

[Install the Docker image or Windows package](./start/installation/). Both include
the executable, native libraries, and a sample image. With Docker:

```sh
docker run -d --name pixerve -p 127.0.0.1:3000:3000 ghcr.io/xviidev/pixerve:v0.1.2
curl 'http://localhost:3000/tr:f-webp,w-320/example.png' -o example.webp
```

## Resize, crop, and convert

Set the size and format in the image URL, or use a preset such as `card`:

```text
/photos/example.jpg?w=640&f=webp
/photos/example.jpg?p=card&f=avif
```

Follow [Use Pixerve](./start/usage/) for image URLs, crop options, and HTML
examples. For application integration, see [Connect a CMS](./guides/cms/).

## Configure delivery

Choose [public or private access](./guides/signing/), then put a CDN in front of
the service. Public cache hits can be served without reaching Pixerve. See
[caching and delivery](./guides/delivery/) for TTLs and versioned assets.

Pixerve reads images from your storage. Uploads, media records, and publication
remain in your application.
