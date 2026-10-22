# Contributing

Use Node.js 22.18 or newer for documentation and HTTP checks. Server development also
requires Rust, a C linker, and libvips with the codecs under test.

```sh
npm ci
npm run check
npm test
npm run build
cargo fmt --check
cargo test --locked
cargo build --release --locked
npm run test:http
```

On Windows, put the libvips DLL directory on `PATH` and set `VIPS_LIB_DIR` for
builds. Set `PIXERVE_BIN` to test a binary other than `target/release/pixerve`.

Keep source reads behind cache lookup and bounded admission. Keep authentication
before redirects and cache lookup. Changes to URL encoding, presets, or signing
must pass the SDK/Rust compatibility tests. The JavaScript SDK and its development
instructions are maintained in [pixerve-js](https://github.com/xviidev/pixerve-js).

Write documentation in English. Keep examples executable and explain limits
where they affect a user's decision. Do not commit credentials, generated build
output, or copies of documentation in a second location.

See [development](apps/docs/src/content/docs/contributing/development.md) and
[architecture](apps/docs/src/content/docs/contributing/architecture.md) for source
setup and implementation details.

Official server publishing procedures are documented in the
[maintainer release guide](.github/RELEASING.md).
