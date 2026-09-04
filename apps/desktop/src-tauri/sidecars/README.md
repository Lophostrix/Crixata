# llama.cpp Sidecar Binaries

This directory holds the `llama-server` sidecar binaries for each supported platform.

Do **not** commit the actual binaries to Git. They are downloaded by the build script or CI pipeline.

## Supported target triples

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`
- `aarch64-pc-windows-msvc`

## Download source

Official llama.cpp releases:
https://github.com/ggerganov/llama.cpp/releases

## Expected filename

`llama-server-<target-triple>`

For example:

```text
llama-server-x86_64-unknown-linux-gnu
```

## Build-time behavior

If the binary for the current target is missing, `build.rs` will download the latest compatible release automatically.
