---
title: Examples and development
order: 5
outline: deep
---

# Examples and development

The repository includes small Rust applications under `apps/examples`. Each example is a workspace member with its own `Cargo.toml` and demonstrates one focused Abra workflow.

## Run an example

From the repository root, run an example by selecting its package and binary:

```sh
cargo run -p gradient-test --bin gradient
```

Most examples write generated images to an `out` directory relative to the example's working directory. Check the example source for its input assets and output filename.

## Useful examples

| Example | What it demonstrates |
| --- | --- |
| `gradient` | Multi-stop gradients and canvas overlays. |
| `viewbox-test` | Normalized geometry and viewport sizing. |
| `layers-test` | Loading and composing image layers. |
| `blend-test` | Image blend modes. |
| `blur-test` | Blur filters. |
| `adjustments-test` | Color and tonal adjustments. |
| `plugins-test` | Plugin-driven image composition. |
| `super-resolution` | AI image enhancement. |

Package and binary names can differ. Inspect the example directory's `Cargo.toml` when a command needs a different name.

## Build the workspace

Build the default workspace members:

```sh
cargo build
```

Build the release artifacts:

```sh
cargo build --release
```

Run tests for the workspace:

```sh
cargo test
```

Some AI and GPU examples require additional native runtimes, models, or hardware. Start with the core examples when verifying a local installation.

## Develop the documentation site

The documentation site lives in `apps/docs` and uses VitePress. Install JavaScript dependencies once, then start the development server:

```sh
cd apps/docs
npm install
npm run docs:dev
```

Build the static documentation site for validation:

```sh
npm run docs:build
```

Preview the production build locally:

```sh
npm run docs:preview
```

The source Markdown is under `apps/docs/public`. VitePress generates the sidebar from folders and frontmatter such as `title` and `order`.

## Add a documentation page

Create a Markdown file under the relevant `apps/docs/public` section with frontmatter:

```md
---
title: My guide
order: 10
outline: deep
---

# My guide

Explain the API and include a runnable example.
```

Use relative links between pages and validate the full site with `npm run docs:build` before publishing.

## Debug common issues

### Asset path errors

Paths such as `assets/photo.jpg` are resolved relative to the process working directory. Run the example from the directory expected by its source, or use an explicit path.

### Missing output directory

Create the output directory before saving generated files:

```rust
std::fs::create_dir_all("out").unwrap();
```

### Wrong package name

Use `cargo metadata --no-deps` or inspect the nearest `Cargo.toml` to find the package and binary names:

```sh
cargo metadata --no-deps --format-version 1
```
