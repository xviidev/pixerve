# Installation

[Documentation index](README.md)

## Requirements

- Rust 1.88 or newer; the tested toolchain is Rust 1.92.
- A C linker and compatible native libvips libraries.
- Codec support for every format you intend to serve.
- A readable static directory and watermark, plus local sources or S3 credentials.

The tested Windows SDK is libvips 8.18.2. Other platform/codec combinations need
validation; JPEG decoding support does not imply AVIF encoding support.

## Linux: Debian or Ubuntu

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libvips-dev
cargo build --release --locked
```

The runtime also needs libvips and its codec dependencies. The Dockerfile uses
Debian Bookworm and installs `libvips42` in the runtime image. The Linux/container
recipe has not yet been validated locally.

## Windows

Install a Rust toolchain with a compatible C linker and extract a matching
libvips SDK. Replace the SDK location below:

```powershell
$env:VIPS_LIB_DIR='C:\tools\vips-sdk\lib'
$env:PATH="C:\tools\vips-sdk\bin;$env:PATH"
cargo build --release --locked
Copy-Item .env.example .env
```

`VIPS_LIB_DIR` supplies a build-time linker search path. The DLL directory must
also be on `PATH` when running the server, CLI, or Rust tests.

## Local sources

Copy `.env.example` to `.env`. For the bundled local samples, use:

```dotenv
SOURCE_DIR=static
STATIC_DIR=static
OG_WATERMARK=assets/og_wm.png
SIGNING_KEY=
```

For your own files, point `SOURCE_DIR` at a directory containing, for example,
`photos/example.jpg`. The URL `/photos/example.jpg` maps to that relative key.
`/static/logo.png` always reads from `STATIC_DIR`, including when main sources
use S3. Local paths are canonicalized and must remain inside their root.

The watermark is required at startup even if OG is unused. The project supplies
a transparent 1200 × 630 starter overlay; replace it to add visible branding.
See the [social image API](api.md#social-images). Portable ZIP users can follow
the [distribution guide](distribution.md) without installing a development SDK.

## S3-compatible storage

Leave `SOURCE_DIR` unset or empty and supply:

```dotenv
S3_ENDPOINT=https://s3.example.com
S3_REGION=auto
S3_BUCKET=your-image-bucket
S3_ACCESS_KEY_ID=your-access-key
S3_ACCESS_KEY_SECRET=your-secret-key
STATIC_DIR=static
OG_WATERMARK=assets/og_wm.png
```

Requests use path-style bucket addressing. Use your provider's endpoint and
region. An `http://` endpoint permits HTTP for an isolated local emulator.
Source reads have a 30-second timeout and a configured byte limit. The server
does not upload objects.

## Start and verify

```sh
./target/release/pixerve
```

On Windows, use `./target/release/pixerve.exe`. From another terminal:

```sh
curl http://localhost:3000/health
curl -L 'http://localhost:3000/photos/example.jpg?w=640&f=webp' -o example.webp
```

Use an actual source key; the second command assumes unsigned open mode.
Health returns `{"ok":true}` after initialization. It does not probe storage or
test every codec. Startup fails for unavailable required paths or invalid settings.

Set a signing key and follow [deployment](deployment.md) before routing production
traffic. Full defaults are in [configuration](configuration.md).
