# Code Management

This explains how to handle code modifications.

**IMPORTANT:** Library execution speed is number 1 priority. Anything slowing down the library should be optimized or rewritten for better performance; image result quality should not be compromised in the process.

## Guidelines

- Do full function/module rewrites.
- If the function/module can be written more efficiently, do so.
- Execution speed is priority. The library is meant to be as **FAST** as possible.
  - If anything is slowing down the speed of the library fix it without changing the external behavior or final result.
  - Profile and benchmark code to identify performance bottlenecks.
  - Optimize algorithms and data structures for better performance.
- Do not preserve legacy code.
- Keep code simple and maintainable.
- Avoid introducing unnecessary complexity.
- Use versatility for example the `Fill` `enum`, and `fill()` function to handle different features using the same interface.
- Avoid duplicating code unnecessarily.
- Reuse existing functions as much as possible; this will require modifying the parameters of existing functions most of the time. 100% okay to do.
  - Use `impl Into<XXX>` for function parameters to allow flexible input types.
  - Use `enum` types for function parameters when there are a limited set of valid options.

## Performance Considerations

Potentially allow for "options" that can be used to configure performance-related aspects of functions.

Use "balanced" default values for the performance settings, which aims to provide a good compromise between execution speed and image result quality.

### Option 1: Performance Modes

Consider providing different performance modes for functions, such as an option for "fast mode" that prioritizes execution, "balanced" mode that balances speed and quality, and "high-quality" mode that prioritizes image result quality over execution speed.

### Option 2: Algorithm Selection

Consider allowing users to select different algorithms for certain operations, where some algorithms may be faster but less precise, and others may be slower but produce higher-quality results.

### Option 3: Builder Pattern

Consider using the builder pattern to allow users to configure performance-related options in a flexible and readable manner. This can help in setting multiple options at once and make the code more maintainable.

## Recommendations for Implementation

- Consider providing benchmarks or guidelines for when to use each mode.
- Ensure that switching between modes does not introduce significant overhead.
- Document the expected trade-offs for each mode clearly for users of the library.
- Regularly review and update performance considerations as the library evolves to ensure optimal performance without compromising image quality.
