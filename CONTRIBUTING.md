# Contributing to Pixerve

Thanks for helping improve image processing and delivery. Contributions should
preserve predictable resource use, accurate cache policy, and signature behavior.

## Current layout

```text
pixerve/
├── .github/workflows/      # Windows/Linux CI
├── Cargo.toml              # Pixerve Cargo package
├── package.json            # Development-only Node test dependencies
├── package-lock.json       # Reproducible npm ci installation
├── Dockerfile              # Build context is this repository root
├── src/                    # HTTP, native processing, storage, cache, metrics
├── static/                 # Brand images and sample source
├── assets/                 # Transparent starter OG overlay
├── branding/               # Logo and generation prompts
├── scripts/                # HTTP suite and packaging tools
├── deploy/                 # CDN and Prometheus templates
├── docs/                   # Detailed English documentation
├── CDN.md                  # CDN integration guide
└── README.md               # Project introduction
```

The Cargo package and executable are named `pixerve`. This is a standalone
repository; no sibling application, asset directory, or npm installation is required.

## Development setup

Follow [installation](docs/installation.md), including native libraries and
asset paths. Run from the repository root:

```sh
cargo fmt --check
cargo test --locked
cargo build --release --locked
npm ci
node scripts/smoke.mjs
```

Windows tests and CLI invocations also need libvips DLLs on `PATH`. The HTTP suite
requires Node.js 22 or newer and its own Sharp dev dependency installed with
`npm ci`. Sharp is used only by tests, not by the Rust service. The
suite also reads `static/example.png` and `assets/og_wm.png`; it generates its
other fixtures under `target/`, starts local service processes, and mocks S3.

Set `PIXERVE_BIN` to an absolute executable path to test a different build. Without
it, the suite runs `target/release/pixerve` (`.exe` on Windows). It uses temporary
sources and test credentials rather than production storage.

Recorded validation: 10 Rust tests and 134 HTTP/image checks passed on Windows
with Rust 1.92 and libvips 8.18.2. Docker/Linux and live-CDN validation are pending.
Re-run relevant checks after implementation changes; documentation-only edits
need reference/example validation rather than another native release build.

## Change guidelines

- Keep source reads after result-cache lookup and under bounded admission.
- Do not release native-task permits early when a client disconnects.
- Keep authentication before redirects and result lookup.
- Preserve signing normalization independently of cache normalization.
- Change pipeline version and document retention when output behavior changes.
- Keep secrets and source keys out of metric labels; bound new metadata.
- Add regression coverage for behavioral fixes rather than tests that merely
  repeat the implementation. Describe validation and limitations in the PR.
- Write documentation, examples, and new explanatory comments in English.

For a reproducible issue report, include platform, Rust/libvips versions, codec
support, sanitized configuration, request shape, status, and expected behavior.
Use a redistributable fixture. Never paste real credentials or signed private URLs.

## Publication checklist

Before publishing a release:

1. Validate this repository in isolation, using its own Docker context, static
   assets, package lockfile, and CI workflow.
2. Keep the project-owned runtime samples and neutral overlay, or supply your
   own redistributable replacements rather than internal branding.
3. Keep the Pixerve package, binary, scripts, Docker command, log target, and
   documentation names aligned. Version signing protocol changes and document
   regeneration of stored URLs when their payload domain changes.
4. Select a distribution license, add its LICENSE file, and review native and
   dependency redistribution requirements. No application license is assigned yet.
5. Validate a clean checkout, Linux/container codecs, secret exclusions, and
   the complete test workflow before publishing release artifacts.
6. Configure the GitHub repository and run the supplied workflow. Release/registry
   publication is a separate action; no badge or
   published package is claimed here.

See [distribution and CI](docs/distribution.md) for the portable packager,
isolated package checker, and the Windows/Linux artifact workflow.

## Documentation map

The [documentation index](docs/README.md) links every guide. Keep environment
defaults aligned with `src/config.rs`, `src/cdn.rs`, and `.env.example`. Keep HTTP
examples aligned with `src/params.rs`, `src/service.rs`, and the CLI in `src/main.rs`.
