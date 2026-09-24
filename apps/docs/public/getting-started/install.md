---
title: Install Abra
order: 2
outline: deep
---

# Install Abra

Abra is currently organized as a Cargo workspace. The main public crate is `abra`, which re-exports the core image, canvas, drawing, adjustment, filter, mask, and plugin APIs.

## Add Abra to a Rust project

Add the crate to your application's `Cargo.toml`:

```toml
[dependencies]
abra = "1.0.0"
```

For a local checkout or workspace member, use a path dependency instead:

```toml
[dependencies]
abra = { path = "../image/abra/abra" }
```

The workspace currently targets Rust edition 2024. Use a recent stable Rust toolchain capable of compiling edition 2024 crates.

> The package name is `abra`; the Rust import is also `abra`.

## Import the prelude

The prelude exposes the types most applications need for first steps:

```rust
use abra::prelude::*;
```

This includes `Image`, `Color`, `ImageLoader`, `Area`, `Path`, `Gradient`, transform traits, and common plugin types. For canvas and drawing helpers, import their module preludes:

```rust
use abra::canvas::prelude::*;
use abra::drawing::prelude::*;
```

Prefer explicit module imports when an application grows and the source needs to make dependencies clearer.

## Create a project

Create a binary project with Cargo:

```sh
cargo new abra-example
cd abra-example
```

Add the dependency, create an `assets` directory for source images, and put generated files in an `out` directory:

```sh
mkdir assets out
cargo run
```

Abra creates output files when you call `save`; it does not create arbitrary parent directories automatically, so create `out` before saving when it may not exist.

## Verify the installation

Use a small program that creates an image without requiring an input asset:

```rust
use abra::prelude::*;

fn main() {
  let image = Image::new_from_color(320, 180, Color::from_hex(0x203040));
  image.write("out/installation-check.png", None)?;
}
```

Run it with:

```sh
cargo run
```

If `out/installation-check.png` is created, the crate is linked and image writing is working.

## Optional GPU support

The `abra` crate exposes an optional `gpu` feature:

```toml
[dependencies]
abra = { version = "1.0.0", features = ["gpu"] }
```

Start without the feature while learning the API. Enable it only when the relevant GPU integration is available in the target environment.
