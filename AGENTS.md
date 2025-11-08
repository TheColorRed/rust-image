# Working with this project

This project uses Cargo as its build system and package manager.

## Abra

Abra is an image processing library written in Rust. It provides functionality for loading, manipulating, and saving images, as well as working with layers and colors, blending, etc.

The library is located in the `packages/abra` directory.

We don't need to deprecate, make fallbacks, support legacy features, etc., as this isn't a public library yet. We can freely refactor and improve the codebase as we see fit.

### Mod.rs

`mod.rs` files should only contain module declarations and re-exports. All other code should be placed in separate files.

### Public API

Use clean simple names for public API items. Avoid abbreviations unless they are widely recognized. For internal structs use `Inner` such as `CanvasInner` to differentiate from the public API struct `Canvas`.

### Naming Conventions

Use the [naming conventions](./.github/instructions/naming-conventions.instructions.md) for public API items and internal structs.

## Alakazam

Alakazam is a GUI application built using the Abra library. It provides a user interface for loading images, applying layers and effects, and saving the results. It's goal is to work similarly to Photoshop or GIMP.

The application is located in the `packages/alakazam` directory.

## Test Programs

The `packages/tests` directory contains various test programs that demonstrate the functionality of the Abra library. These tests can be run to verify that the library is working correctly, and for a playground to experiment with the library's features.

Test program file structure:

```
packages/tests
 ├── example 1
 │    ├── main.rs
 │    └── Cargo.toml
 ├── example 2
 │    ├── main.rs
 │    └── Cargo.toml
 └── ...
```

### Test Program Execution

We can run the test programs using Cargo. For example, to run a test program located in `packages/tests/layers`, we can use the following command (The package name is found in the `Cargo.toml` file in that directory):

```sh
# Git Bash
/c/Users/untun/.cargo/bin/cargo.EXE run --package <package-name> --bin <package-name>

# Windows Command Prompt
C:\Users\untun\.cargo\bin\cargo.EXE run --package <package-name> --bin <package-name>
```

## Documentation

All documentation files are located in the `docs` directory. These files provide information about the design, architecture, and usage of the Abra library and Alakazam application. Also any API reviews and improvement tracking documents are stored here.