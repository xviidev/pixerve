---
title: Installation
description: Install the Docker image or Windows portable package and start serving images.
---

Choose the Docker image for a server deployment or the portable Windows x64
package. Both include Pixerve and its native image libraries.

## Docker

Install Docker, then start the published Linux amd64 image:

```sh
docker pull ghcr.io/xviidev/pixerve:v0.1.2
docker run -d --name pixerve \
  -p 127.0.0.1:3000:3000 \
  -v pixerve-cache:/app/.cache \
  ghcr.io/xviidev/pixerve:v0.1.2
```

Open [the sample image](http://localhost:3000/tr:f-webp,w-320/example.png), or save it:

```sh
curl 'http://localhost:3000/tr:f-webp,w-320/example.png' -o example.webp
```

Check logs with `docker logs pixerve`. Stop the service with `docker stop pixerve`;
use `docker start pixerve` to start the same container again.

The default image serves bundled samples from `/app/static`. The cache volume
keeps usable image results across container replacements.

## Windows

Download the [Windows x64 package for v0.1.2](https://github.com/xviidev/pixerve/releases/download/v0.1.2/pixerve-v0.1.2-windows-x64.zip).

1. Extract `pixerve-v0.1.2-windows-x64.zip`.
2. Open PowerShell in the extracted package directory. Keep the DLLs and asset
   folders beside `pixerve.exe`.
3. Create the configuration and start Pixerve:

```powershell
Copy-Item .env.example .env
./pixerve.exe
```

The package configuration serves bundled samples. Open
[the sample image](http://localhost:3000/tr:f-webp,w-320/example.png) to verify it.
Run the executable from this directory so `.env` and relative paths resolve correctly.

## Local storage

For Windows, set `SOURCE_DIR` in `.env` to the directory containing your images:

```dotenv
SOURCE_DIR=C:/images
```

A file at `C:/images/photos/example.jpg` is available as `/photos/example.jpg`.
Restart Pixerve after changing configuration.

For Docker, mount your image directory read-only and set its container path:

```sh
docker run -d --name pixerve-images \
  -p 127.0.0.1:3000:3000 \
  -e SOURCE_DIR=/data/images \
  -v /absolute/path/to/images:/data/images:ro \
  -v pixerve-cache:/app/.cache \
  ghcr.io/xviidev/pixerve:v0.1.2
```

Stop the sample container before starting another container on the same port.
`STATIC_DIR` configures the separate `/static/` route. Changing `SOURCE_DIR`
does not change where static files are read.

## S3-compatible storage

Set these values in `.env`, using your provider's endpoint and credentials:

```dotenv
SOURCE_DIR=
S3_ENDPOINT=https://s3.example.com
S3_REGION=auto
S3_BUCKET=images
S3_ACCESS_KEY_ID=your-access-key
S3_ACCESS_KEY_SECRET=your-secret-key
```

For Docker, pass the file and explicitly override the image's local-source default:

```sh
docker run -d --name pixerve-s3 \
  -p 127.0.0.1:3000:3000 \
  --env-file .env \
  -e SOURCE_DIR= \
  -v pixerve-cache:/app/.cache \
  ghcr.io/xviidev/pixerve:v0.1.2
```

Upload source images through your CMS or storage tools. Pixerve reads existing
objects using path-style bucket addressing.

## Next steps

Start with [image URLs and presets](../usage/), then configure
[signing and access](../../guides/signing/) and
[HTTPS/CDN delivery](../../guides/deployment/).
All settings are listed in the [configuration reference](../../reference/configuration/).
