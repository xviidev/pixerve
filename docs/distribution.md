# Portable packages and CI builds

[Documentation index](README.md)

Pixerve's initial distribution targets are Windows x64 portable ZIPs and Linux
x64 Docker images. These include native runtime dependencies so end users do not
need a Rust compiler or libvips development SDK. Builds are currently local/CI
artifacts; no GitHub release or registry publication is performed automatically.

## Windows portable ZIP

From a source checkout with a built release executable and compatible libvips SDK:

```powershell
./scripts/package-windows.ps1 -VipsDir 'C:\tools\vips-sdk'
```

The packager reads the version from Cargo.toml and creates:

```text
dist/
  pixerve-v0.1.0-windows-x64/
  pixerve-v0.1.0-windows-x64.zip
  pixerve-v0.1.0-windows-x64.zip.sha256
```

Use `-Binary` for another release executable or `-OutputDir` for a different
destination/rebuild. Existing package directories are not overwritten. Build
the executable for `x86_64-pc-windows-msvc` with the matching SDK architecture;
the packager is not a cross-compiler.

The ZIP includes the executable, SDK runtime DLLs, optional native module/resource
directories, upstream SDK license/version files, English documentation, sample
configuration, a neutral PNG, and a transparent OG overlay. It omits SDK headers,
import libraries, development command-line tools, and real `.env` credentials.
`manifest.json` records SHA-256/size for bundled files and native dependency versions.
The sidecar checksum covers the final ZIP; it is not a release signature.

Extract the complete directory. Keep DLLs alongside `pixerve.exe`, then:

```powershell
Copy-Item .env.example .env
./pixerve.exe
```

The supplied configuration serves local samples. Open `/health`, then
`/example.png?w=640&f=webp`, following canonical redirects. For S3, clear
`SOURCE_DIR` and fill the S3 settings. Configure signing, HTTPS proxying, and
source publication before production use. Replace the transparent OG overlay
with your own artwork to add visible branding.

## Validate an extracted package

```powershell
node scripts/check-package.mjs 'C:\downloads\pixerve-v0.1.0-windows-x64'
```

The checker verifies manifest hashes, launches the packaged executable with only
Windows system directories on its child PATH, and tests JPEG/PNG/WebP/AVIF,
conditional responses, OG, and static files. It explicitly enables PNG in its
test open-format policy; the product default remains AVIF/WebP/JPEG.

For the full HTTP suite, point `PIXERVE_BIN` at the extracted executable and run
`node scripts/smoke.mjs` with the developer SDK removed from PATH. This suite
requires Node.js and Sharp for fixture generation/inspection; those are developer
test dependencies, not runtime requirements for the ZIP.

The local package passed isolated SDK-path tests and all 134 HTTP checks. This
is not a claim that every Windows version or clean virtual machine was tested.

## Linux Docker image

Build from the Pixerve repository root:

```sh
docker build --platform linux/amd64 -t pixerve:local .
docker run --rm -p 127.0.0.1:3000:3000 pixerve:local
```

The image includes Debian's libvips runtime, a non-root `pixerve` user, neutral
sample sources, and a transparent OG overlay. It starts in local sample mode.
For S3, override `SOURCE_DIR` to empty, supply credentials/settings, and persist
the writable cache volume. See [deployment](deployment.md).

The local Docker daemon was unavailable, so Linux/container validation must run
in CI or on a host with Docker. Native libvips/codec versions can differ between
Windows and the Debian container; inspect native output before production rollout.

## GitHub Actions

The repository's `.github/workflows/pixerve-build.yml` runs on pull
requests, main-branch changes, or manual dispatch with read-only repository access.
It does not publish releases, log into a registry, or push images.

Windows jobs install Rust 1.92.0 and the official libvips 8.18.2 x64 web SDK.
The SDK archive is verified against a pinned SHA-256. Jobs run format checks,
Rust tests, the full HTTP suite, ZIP packaging, and extracted-package checks,
then upload the ZIP/checksum as a workflow artifact.

Linux jobs run native format/Rust/HTTP checks, build the x64 image, test its
startup and output formats, confirm the non-root user, and upload a compressed
`docker save` archive with checksum. To load that artifact:

```sh
gzip -dc pixerve-linux-amd64-docker.tar.gz | docker load
docker run --rm -p 127.0.0.1:3000:3000 pixerve:ci
```

Workflow artifacts do not mean the workflow has already run. All workflow paths
and the Docker build context resolve from the standalone repository root.

## Rebuilding assets

`static/example.png` and `assets/og_wm.png` are deterministic,
project-generated fixtures. To regenerate them:

```sh
node scripts/generate-runtime-assets.mjs
```

The generator uses only Node built-ins, with no external images or fonts. It
replaces those two sample files. Their current purpose is startup/demo/test
portability, not a finished brand identity.
