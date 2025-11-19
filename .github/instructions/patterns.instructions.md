---
description: Defines patterns for writing new code or updating existing code.
applyTo: '**'
---

# Coding Patterns

This document outlines recommended patterns for writing new code or updating existing code in the project. Adhering to these patterns helps maintain consistency, readability, and maintainability across the codebase.

## Builder

The code uses the Builder pattern for constructing complex objects. This pattern allows for step-by-step construction and is particularly useful for objects with many optional parameters.

The builder should contain a `new()` method or `new_*` function to initialize the builder with default values. Each method in the builder should return `self` to allow for method chaining. All functions should be prefixed with `with_` to indicate that they set a property.

## Doc Comments

Doc comments should be used to describe the purpose and usage of structs, enums, functions, and methods. They should provide clear and concise explanations to help other developers understand the code. Use triple slashes (`///`) for doc comments.

### Outer Doc Comments

This should provide a good explanation of the file/module's purpose and functionality. Things such as: what the module contains, what is the point of the module, and any other relevant context. It should contain a high-level overview of the module's contents and how the module fits into the library.

### Description

The first lines of a doc comment should provide a good detailed description of the item being documented. This should not be overly verbose but should give enough context for someone unfamiliar with the code to understand its purpose.

### Parameters

When documenting parameters in doc comments, use the following format:

```rust
/// - `param_name`: Description of the parameter.
```

### Examples

When applicable, include examples in the doc comments to illustrate how to use the item being documented. Use code blocks to format examples clearly.

Use three ticks with a `ignore` tag for code examples that should not be tested or compiled.

```rust
/// ```ignore
/// // Example code here
/// ```
```