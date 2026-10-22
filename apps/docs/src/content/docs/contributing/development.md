---
title: Development
description: Build and test the Rust server and documentation.
---

The repository contains the server, documentation, and delivery examples:

```text
src/                  Rust server
apps/docs/            Astro + Starlight documentation
scripts/              HTTP checks and release tooling
deploy/               CDN and monitoring templates
```

## JavaScript and TypeScript

Use Node.js 22.18 or newer and install from the root:

```sh
npm ci
npm run check
npm test
npm run build
```

`npm run check` checks documentation types. `npm test` runs server release-tag
tests. `npm run build` emits the documentation site at `apps/docs/dist`.

Use `npm run dev:docs` while editing documentation. Set `DOCS_SITE` to the public
site origin when deploying. Set `DOCS_BASE` if hosting under a path prefix.
The generated files can be hosted on a static web server; no runtime backend is required.

The public site is deployed to [GitHub Pages](https://pixerve.xvii.dev/)
by `.github/workflows/docs.yml`. Changes to the documentation on `main` trigger
a build, link checks, and deployment. The workflow obtains the site origin and
base path from GitHub Pages; local development remains at `/`.

## Rust and image checks

Install the native prerequisites below, then run:

```sh
cargo fmt --check
cargo test --locked
cargo build --release --locked
npm run test:http
```

`test:http` starts local servers and a mock S3 endpoint. It uses `PIXERVE_BIN`
when set, otherwise it uses the release executable. Windows requires the
libvips DLL directory on `PATH` for the CLI and server.

## Native source builds

Source builds require Rust 1.88 or newer, a C linker, and libvips. CI uses Rust
1.92 and the Windows libvips 8.18.2 web SDK.

On Linux:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libvips-dev
cargo build --release --locked
```

For AVIF on Ubuntu 24.04, install `libheif-plugin-aomenc` and
`libheif-plugin-dav1d`. Debian Bookworm provides these codecs through its libheif
dependencies instead. To test the Ubuntu encoder:

```sh
sudo apt-get install libvips-tools libheif-plugin-aomenc libheif-plugin-dav1d
mkdir -p target/codec-check
vips heifsave static/example.png target/codec-check/preflight.avif --compression av1 --Q 70 --effort 2 --bitdepth 8 --strip
```

On Windows, install Rust with the MSVC toolchain and a C linker, then extract the
libvips x64 web SDK:

```powershell
$env:VIPS_LIB_DIR='C:\tools\vips-sdk\lib'
$env:PATH="C:\tools\vips-sdk\bin;$env:PATH"
cargo build --release --locked
```

Keep the DLL directory on `PATH` when running source-build executables. To build
a development Docker image, run `docker build -t pixerve:local .` from the root.

## JavaScript SDK

The SDK has its own repository, version, and npm publication workflow:
[pixerve-js](https://github.com/xviidev/pixerve-js). See its
[contribution guide](https://github.com/xviidev/pixerve-js/blob/main/CONTRIBUTING.md)
for development and compatibility tests. End-user setup is covered in
[CMS integration](../../guides/cms/).

## Changing the URL protocol

Keep canonical encoding, presets, and signature normalization aligned across
Rust and TypeScript. Signing normalizes only present fields; result cache keys
also normalize rendering defaults. Those are separate contracts.

Changing encoded bytes, watermark, or preset semantics requires a pipeline
version change and [retention planning](../../guides/delivery/#pipeline-upgrades-and-retention).
Add regression cases to the compatibility suite for URL changes.

Server CI checks a pinned `pixerve-js` commit in `.github/workflows/pixerve-build.yml`.
Update both platform pins when adopting a new SDK contract. The SDK's own CI
also checks compatibility against the server's `main` branch.

Write documentation in English. Keep source content in this site rather than
copying guides into the README. Avoid build output, credentials, and private
source keys in commits.
