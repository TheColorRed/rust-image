---
name: abra-library
description: The Abra Library is a rust library that is the core of the image manipulation. It provides functions for loading, saving, and manipulating images with or without using a canvas and layers.
metadata:
  keywords:
    - abra
    - rust
    - image
    - manipulation
    - library
    - WebAssembly
    - wasm
---

# Abra Library

The Abra library is located in the `/abra` directory of the repository. It is broken down into several modules, each responsible for different aspects of image manipulation and processing.

## Reuse First (Read Before Writing Code)

**IMPORTANT:** Before creating any new function, struct, or module for a tool, filter, adjustment, effect, or other feature, **search the whole workspace** to see if that functionality already exists.

- If it exists, use it.
- If something close exists, update it to cover your use case, keeping it general-purpose and reusable (not tailored to your feature).
- Keep the library general. A feature specific to one subject or task is a composition of general operations (select where, test which pixels count, change them with an existing adjustment), not a new library function named after the feature.
- Only write new code when nothing suitable exists, and place reusable logic in a core/shared crate rather than inside the feature.

See the [reuse first document](./references/reuse-first.md) for the full workflow, where to search, and how to extend existing code.

## Root Crate

The root crate of the Abra library is located in the root of the workspace. It is broken down into several workspaces, some are for the library and others are for the examples (see below).

## Modules

- `/packages/abra/abra`: This is the main entry point of the library. It doesn't contain much logic itself but re-exports the other modules for easier access.
- `/packages/abra/ai`: This module contains functions and types related to AI-powered image processing tasks, such as image enhancement. Models describe themselves with a `.yml` manifest beside the `.onnx` file, so adding a model of a kind the library can already read needs no code. See `ai/core` (`AiModel`, `Manifest`). The library never downloads or locates models; the app passes a path.
- `/packages/abra/core`: This directory contains the core functionality of the library and is further divided into several sub-modules:
  - `adjustments`: Contains functions for adjusting image properties such as brightness, contrast, and saturation.
  - `canvas`: A canvas is a group of layers, and this module contains functions for creating and managing canvases and their layers.
  - `core`: Contains common types and functions used throughout the library and is used by other modules. It is commonly used as the middle-man for passing data between modules. Includes geometry, transforms, color utilities, blending, file IO, and area/GPU application helpers.
  - `debug`: Contains debug console output helpers for inspecting canvases, layers, and effects.
  - `drawing`: Contains functions for drawing shapes and text onto images.
  - `filters`: Contains functions for applying various filters to images, such as blur, sharpen, and color adjustments.
  - `gpu`: The adapter that plugs the standalone `gpu-passes` runtime into abra: it creates the shared GPU context, follows the `gpu.enabled` setting and registers as the GPU provider that `apply_in_area` uses. Effects here only describe themselves as GPU passes.
  - `mask`: Contains functions for creating and applying masks to images.
  - `options`: Contains types and functions for configuring options for various image processing tasks.
  - `primitives`: Contains the lightweight base types (`Image`, `Channels`, `Color`, `Resolution`) with no heavy dependencies.
  - `tools`: Contains interactive image tools (perspective, straighten, remover). Tools are thin wrappers that compose library functions.
  - `typography`: Contains font loading, glyph metrics, and text rendering support.
- `/packages/vessel/events`: A standalone event system in the style of RxJS (no abra dependency, so it can be published on its own): `Subject<T>`, `BehaviorSubject<T>` and the `Observable` and `Observer` traits. Import them with `use pub_sub::prelude::*`.
- `/packages/vessel/vessel`: The standalone view engine, package `vessel` (no abra dependency, so it can be published on its own). `Engine` runs `View`s on its own thread and decides when to draw them; each view shows a `MediaSource` (a canvas, a video, a camera, a game...). It uses `pub-sub`.
- `/packages/vessel/gpu-passes`: Compute shader passes over RGBA pixels (`GpuPass`, `GpuAux`, `GpuSession`) and, with its `runtime` feature, the wgpu runtime that runs them (`GpuContext`, `LiveRenderer`, `Presenter`). No abra dependency. Abra supplies passes as data; it never runs the GPU itself.
- `/packages/vessel/surface`: Surfaces that frames are drawn on, looked up by id (`Surface` trait; a native Android window is the first kind). No abra dependency.
- `/packages/abra/plugins`: This module contains functions and types related to plugins that can extend the functionality of the Abra library.

## Images That Wait, and Parts of Images

An image can stand for work that has not run. `effect.apply(deferred(&mut image))` (see `abra-core`, `image/effect.rs`) makes the image remember the effect instead of applying it, for any effect (a shader is not needed), unless the effect has an area or a mask or is set to the CPU. Nothing is made until somebody reads the pixels, or asks for just a part:

- `ImageExt::region(level, left, top, width, height)` (`abra-core`, `image/image_ext`) returns a rectangle of the image at a zoom-out `level` (level 0 is the image, each level above it is the one before halved; see `image/levels.rs`). A pixel past an edge is the pixel at that edge. A waiting CPU effect makes only that part, plus the `padding()` around it that the effect reads (`image/recipe.rs`); an effect says how it works at a zoomed-out level with `Effect::for_level`, and one that does not is made whole once and cut from that.
- `Image::version()` (`primitives`) is a number that names the pixels: clones share it and any change makes a new one, so a caller can tell whether it has to draw an image again with one comparison. `Image::derived(..)` keeps something worked out from the pixels (the zoom-out levels) until they change.
- `recipe::gpu_run(&image)` gives the image the shader passes start from and the passes, for a surface that can draw them itself. A CPU-only effect in the chain is where that image starts.

## Examples

Examples are mini applications that implement specific features of the Abra library. They can be found in the `/apps/examples` directory of the repository. Each example demonstrates how to use different parts of the Abra library to achieve specific image manipulation tasks.

Some examples include:

- **adjustments:** Demonstrates how to use an adjustment (brightness) on an image.
- **ai:** Demonstrates how to use AI-powered image enhancement.
- **blend:** Demonstrates how to blend two images together using different blend modes (like multiply, screen, overlay, etc.).
- **blur:** Demonstrates how to apply a blur filter to an image.
- **brush:** Demonstrates how to use a brush to draw on an image.
- **drawing:** Demonstrates how to draw shapes and text onto an image.
- **gradient:** Demonstrates how to create and apply gradients to images.

## Applications and Bindings

**Read the [applications document](./references/applications.md) before writing or reviewing any code that uses the library from outside it:** Rust examples, the Electron app, the React Native app, bindings, or any application added later. The rule is that the library does the work and applications are thin callers. A missing operation is added to the library first.

## Code Management

See the [code management document](./references/code-management.md) for guidelines on handling code modifications within the Abra library.

## Code Placement

See the [code placement document](./references/code-placement.md) for guidelines on where to place new code or refactor existing code.

## Testing Changes

Use the examples folder in the `/apps/examples/<feature>` directory to test changes. See the [testing document](./references/testing-changes.md) for more information.

## Public API

See the [public API document](./references/public-api.md) for guidelines on designing and using the public-facing API of the Abra library.

## Naming Conventions

See the [naming conventions document](./references/naming-conventions.md) for guidelines on naming structs, functions, and parameters within the Abra library.

## Patterns

Defines patterns for writing new code or updating existing code. See the [patterns document](./references/patterns.md) for more information.
It includes the rule that application code owns control ranges and validation,
while library builders preserve supplied parameters and only bound physical output
representations.

## Tools

Instructions for using tools in the abra library. See the [tools document](./references/tools.md) for more information. This is used for researching information about image processing techniques, algorithms, and best practices.

## Documentation

If features significantly change or are added to the Abra library, the documentation should be updated accordingly to reflect the new functionality. Update the docs in the `/apps/docs` and look at the documentation skill for guidance on maintaining accurate and up-to-date documentation.
