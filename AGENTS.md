# Working with this project

This project uses Cargo as its build system and package manager.

## Features

⚠️ **No deprecations, no fallbacks, no legacy features** ⚠️

We **MUST** remove legacy features or features that will become legacy due to updates or new features we add. This keeps the codebase clean and maintainable.

This is not a public library. We can freely refactor and improve the codebase as we see fit.

## Abra

Abra is an image processing library written in Rust. It provides functionality for loading, manipulating, and saving images, as well as working with layers and colors, blending, etc.

The library is located in the `packages/abra` directory.


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

## No Deprecations Policy

This is a new project, any changes done should directly improve the codebase without adding deprecated features or fallbacks. This keeps the codebase clean and maintainable.

For example, if we are adding a new feature that replaces an old one, we should remove the old feature instead of marking it as deprecated.

We do not care if a change is a large code modification if it is better in the long run. Minimal changes are the worst possible approach as they lead to code rot and technical debt. **DO NOT BE AFRAID TO MAKE LARGE CHANGES IF THEY IMPROVE THE CODEBASE.**

### Example of Bad Practice

This example shows a basic example of adding a new feature while keeping the old one, which is against our no deprecations policy.
```rust
// Bad: Adding a new feature while keeping the old one
pub fn old_feature() {
    // Old implementation
}
pub fn new_feature() {
    // New implementation
}
// Usage of the features
pub fn use_feature() {
  if use_old {
    old_feature();
  } else {
    new_feature();
  }
}
```

### Example of Good Practice

This example shows a basic example of adding a new feature while removing the old one, which follows our no deprecations policy.
```rust
// Good: Removing the old feature when adding a new one
pub fn new_feature() {
    // New implementation
}
// Usage of the new feature
pub fn use_feature() {
    new_feature();
}
```

## Unification of Similar Features

There are features that are similar in functionality but have different implementations. These features should be analyzed and unified into a single implementation.

For every new and existing feature we need to think about how this can be implemented so that when we expand upon it we don't write redundant code.

## Edition 2024

This project uses Rust Edition 2024. All new code should follow the conventions and features of this edition.

## no_mangle Attribute

When exposing functions to FFI, we use the `#[unsafe(no_mangle)]` in the 2024 edition, as opposed to `#[no_mangle]` in previous editions. This is to ensure that the function names are not mangled by the Rust compiler, allowing them to be called from other programming languages.