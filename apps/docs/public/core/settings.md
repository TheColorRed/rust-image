---
title: Settings and GPU
order: 8
outline: deep
---

# Settings and GPU

`Settings` controls process-wide Abra configuration. GPU integration is optional and depends on the features and providers enabled by the build.

## Initialize settings

Abra initializes its settings during crate startup. Most applications do not need to call initialization manually, but the type is available through the core prelude:

```rust
use abra::abra_core::Settings;

Settings::init();
```

Call initialization before querying or changing global configuration in an application that uses `abra_core` directly.

## GPU behavior

When GPU support is compiled and enabled in settings, Abra attempts to register a GPU provider during initialization. If no provider is available, image operations continue through the CPU implementation.

Treat GPU acceleration as an optimization rather than a required capability. Applications should produce correct results with GPU support disabled.

## API guidance

The exact settings methods depend on the enabled crate features and provider integration. Keep application-level configuration close to startup, and avoid changing global settings while image operations are running concurrently.

## Related APIs

- Use [Image operations](./image/operations) for CPU/GPU-independent image workflows.
- Use [Image transforms](./transform/image-transforms) for resize and rotation algorithms.
- Use [Rasterization primitives](./drawing/rasterization) for custom CPU drawing pipelines.
