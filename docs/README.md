# Pixerve documentation

These guides describe the Rust + libvips HTTP image service in this checkout.

| Goal | Guide |
|---|---|
| Understand features and try a local image | [Project README](../README.md) |
| Install native dependencies and run the service | [Installation](installation.md) |
| Build portable Windows ZIPs and Linux Docker artifacts | [Distribution and CI](distribution.md) |
| Use the default OG, cover, icon, or placeholder | [Brand assets](branding.md) |
| Resize, crop, convert, or generate a social image | [HTTP API](api.md) |
| Look up an environment variable | [Configuration](configuration.md) |
| Generate public, private, or expiring URLs | [CLI and signing](signing.md) |
| Configure CDN caching and responsive images | [CDN delivery](../CDN.md) |
| Run behind a proxy or migrate a service | [Deployment](deployment.md) |
| Inspect metrics, diagnose failures, or benchmark | [Operations](operations.md) |
| Understand scheduling and cache behavior | [Architecture](architecture.md) |
| Contribute or prepare an independent repository | [Contributing](../CONTRIBUTING.md) |

## Conventions

- Shell examples use a POSIX shell unless labeled PowerShell. Windows users can
  invoke `pixerve.exe` instead of `pixerve`.
- Runtime, CLI, development, and Docker commands run from the standalone Pixerve
  repository root unless stated otherwise.
- Relative paths resolve from the process working directory. Only its `.env`
  file is loaded; existing environment variables take precedence.
- `<full-sha256>`, example hosts, and example secrets are placeholders.
- The product is Pixerve. Its executable and Rust log target are `pixerve`;
  its public asset signing domain is `pixerve-public-asset-v1`.

## Scope

The application reads sources from a local directory or one S3-compatible bucket.
There is no upload API, management UI, cache purge API, distributed result cache,
or arbitrary user-supplied remote URL fetcher. It is currently an executable
service rather than an independently published Rust SDK.

Deployment templates do not indicate that a live CDN has been configured or a
public release has been published.
