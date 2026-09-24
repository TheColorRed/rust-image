---
title: Loading and saving images
order: 2
outline: deep
---

# Loading and saving images

Abra selects the file reader or writer from the file extension. The high-level, fallible `Image` methods are the usual choice.

## Open and save one image

```rust
use abra::prelude::*;

let image = Image::read("assets/input.png")?;
image.write("out/output.webp", None)?;
```

Supported high-level input formats are PNG, JPEG, GIF, WebP, and SVG. High-level output currently supports PNG, JPEG, GIF, and WebP.

Use `ImageFormat` when an output name does not contain an extension:

```rust
let image = Image::read("assets/photo.jpg")?;
image.write_as("out/photo", ImageFormat::Webp, None)?;
```

Supported input formats are PNG, JPEG, GIF, WebP, and SVG. Output supports PNG, JPEG, GIF, and WebP. Unsupported formats return an error.

## Writer options

`WriterOptions` currently exposes a quality value for writers that support quality settings:

```rust
use abra::abra_core::{Image, WriterOptions};

let image = Image::read("assets/photo.jpg")?;
let options = WriterOptions { quality: 90 };
image.write("out/photo.jpg", Some(options))?;
```

Pass `None` to use the writer default. PNG and GIF also accept the options value even though quality has a format-specific effect.

## Load multiple images

Use `ImageLoader` for paths, existing images, glob patterns, or folders:

```rust
use abra::prelude::*;

let loaded = ImageLoader::FromPaths(vec![
  "assets/a.png",
  "assets/b.jpg",
]).load(LoadMode::Parallel { threads: 4 });
```

Other sources include:

```rust
let from_glob = ImageLoader::FromGlob(vec!["assets/**/*.png"]).load(LoadMode::Parallel { threads: 4 });
let from_folder = ImageLoader::FromFolders(vec!["assets/photos"], true).load(LoadMode::Parallel { threads: 4 });
let from_images = ImageLoader::FromImages(vec![Image::read("assets/a.png")?]).load(LoadMode::Sync);
```

Choose parallel or deterministic single-threaded loading with `LoadMode`:

```rust
let mut images = ImageLoader::FromGlob(vec!["assets/**/*.jpg"]).load(LoadMode::Sync);
```

## Manage loaded images

`LoadedImages` stores loaded images behind `Arc<Image>` values:

```rust
if let Some(first) = images.at(0) {
  println!("first image: {:?}", first.dimensions::<u32>());
}

images.add("assets/extra.png");
let first = images.shift();
let last = images.pop();
images.drop(0);
```

Use `at` to read without removing an image, `shift` or `pop` to consume from either end, and `drop` to remove an index.

## API summary

| API                                               | Purpose                                           |
| ------------------------------------------------- | ------------------------------------------------- |
| `Image::read(path)`                               | Fallibly construct an image from a file.          |
| `Image::write(path, options)`                     | Fallibly write based on the path extension.       |
| `Image::write_as(path, format, options)`          | Fallibly write with an explicit format.           |
| `WriterOptions { quality }`                       | Configure supported writer quality.               |
| `ImageLoader::FromPaths`                          | Load a list of paths.                             |
| `ImageLoader::FromGlob`                           | Load paths matching glob patterns.                |
| `ImageLoader::FromFolders`                        | Load images from folders, optionally recursively. |
| `ImageLoader::FromImages`                         | Wrap existing images.                             |
| `load(LoadMode)`                                  | Load in parallel or synchronously.                |
| `LoadedImages::add`, `at`, `shift`, `pop`, `drop` | Manage a loaded image collection.                 |
