# Releasing Pixerve

Maintainer procedures for the official Pixerve repository.

The release workflow builds Windows x64 portable ZIPs and Linux amd64 Docker
images. Branch and pull-request runs produce workflow artifacts. Publishing a
GitHub Release also publishes the tested Docker image to GHCR after both jobs pass.

## Windows portable package

Build a release executable and supply a matching libvips SDK:

```powershell
./scripts/package-windows.ps1 -VipsDir 'C:\tools\vips-sdk'
```

The package name uses the version in `Cargo.toml`. The archive includes the
executable, native DLLs/modules, sample assets, configuration, documentation
source, and dependency notices. `manifest.json` lists file sizes and SHA-256
hashes; a sidecar checksum covers the ZIP.

Existing output directories are not overwritten. Use `-OutputDir` for a rebuild
or `-Binary` to select a different executable. Test an extracted package with:

```powershell
node scripts/check-package.mjs 'C:\downloads\pixerve-v0.1.2-windows-x64'
```

The checker verifies file hashes and runs the packaged binary without developer
SDK paths, exercising codecs, conditional responses, OG, and static routes.

## Publish a server release

1. Align `Cargo.toml` and the Pixerve entry in `Cargo.lock` with the release version.
2. Run checks and push the release sources and workflow to GitHub.
3. Publish a GitHub Release with a tag such as `v0.1.2`. Tag pushes alone and drafts do not publish images.
4. Inspect the release workflow. After Windows and Linux pass, it verifies and
   publishes the same Linux image that was tested, without rebuilding it.

A stable `v0.1.2` release publishes `ghcr.io/xviidev/pixerve:v0.1.2` and
`ghcr.io/xviidev/pixerve:0.1.2`. Only a stable release that is also GitHub's latest
release updates `:latest`. Prereleases keep their version tags.

Windows ZIPs and their SHA-256 checksums are attached to the GitHub Release.
Branch and pull-request builds retain them as workflow artifacts.

Review the application license and native dependency notices before distributing
artifacts. The application currently has no assigned license.

## JavaScript SDK

The SDK is maintained and published from
[pixerve-js](https://github.com/xviidev/pixerve-js). Its npm configuration and
release procedures are in that repository's
[release guide](https://github.com/xviidev/pixerve-js/blob/main/.github/RELEASING.md).
Server releases do not publish the SDK.