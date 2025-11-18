---
description: Defines patterns for writing new code or updating existing code.
---

# Coding Patterns

This document outlines recommended patterns for writing new code or updating existing code in the project. Adhering to these patterns helps maintain consistency, readability, and maintainability across the codebase.

## Builder

The code uses the Builder pattern for constructing complex objects. This pattern allows for step-by-step construction and is particularly useful for objects with many optional parameters.

The builder should contain a `new()` method or `new_*` function to initialize the builder with default values. Each method in the builder should return `self` to allow for method chaining. All functions should be prefixed with `with_` to indicate that they set a property.