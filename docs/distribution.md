# Portable packages and CI builds

[Documentation index](README.md)

Pixerve's initial distribution targets are Windows x64 portable ZIPs and Linux
x64 Docker images. These include native runtime dependencies so end users do not
need a Rust compiler or libvips development SDK. Builds are currently local/CI
artifacts on branch/PR builds. Publishing a GitHub Release additionally publishes
its tested Docker image to GHCR after Windows and Linux checks succeed.

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

The local package passed isolated SDK-path tests and all 137 HTTP checks. This
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
requests, main-branch changes, manual dispatch, and published GitHub Releases.
Build jobs have read-only repository access. Only the release publisher receives
`packages: write` and authenticates to GHCR with the built-in `GITHUB_TOKEN`.
PRs, branch pushes, and manual build dispatches do not publish images.

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

## Publish a release image to GHCR

1. Commit and push this workflow and the release sources to GitHub before
   creating the release tag. Keep Cargo.toml/Cargo.lock versioning aligned with
   the version being released.
2. Create and publish a GitHub Release using `vMAJOR.MINOR.PATCH` (for example,
   `v0.1.0`) or the equivalent unprefixed version. Pre-release suffixes such as
   `v0.2.0-rc.1` are supported. Build metadata (`+build`) is rejected because it
   is not a Docker tag character. Drafts and tag pushes alone do not publish.
3. The workflow runs Windows and Linux checks. The Linux job builds, tests, and
   saves the Docker image with source/revision/version OCI labels.
4. After both jobs succeed, the publisher downloads the same tested image from
   that workflow run, verifies its SHA-256, loads it, and pushes version tags.
   The publisher does not rebuild an untested image.

For `xviidev/pixerve`, a stable `v0.1.0` release publishes:

```text
ghcr.io/xviidev/pixerve:v0.1.0
ghcr.io/xviidev/pixerve:0.1.0
ghcr.io/xviidev/pixerve:latest
```

`latest` is updated only for a stable version that is also GitHub's current latest
release. Pre-releases and older/backport releases keep version tags without
replacing it. The release publisher is serialized per repository. Names are
derived from the GitHub repository and lowercased, so forks use their own GHCR path.

Once publication succeeds:

```sh
docker pull ghcr.io/xviidev/pixerve:v0.1.0
docker run --rm -p 127.0.0.1:3000:3000 ghcr.io/xviidev/pixerve:v0.1.0
```

No custom registry token is required for the workflow. Repository/package policy
must allow its GITHUB_TOKEN to publish. A first GHCR package may default to
private visibility; set its package visibility to Public if you want anonymous
pulls. For an existing package, grant this repository Actions access if needed.
See [GitHub's Container registry guide](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry).

A release publishes the Linux amd64 image. Windows ZIPs remain workflow artifacts;
this workflow does not attach them as release downloads. No release or registry
push has been executed by this local configuration change.

## Rebuilding assets

`static/example.png` and `assets/og_wm.png` are deterministic,
project-generated fixtures. To regenerate them:

```sh
node scripts/generate-runtime-assets.mjs
```

The generator uses only Node built-ins, with no external images or fonts. It
replaces those two sample files. Their current purpose is startup/demo/test
portability, not a finished brand identity.
