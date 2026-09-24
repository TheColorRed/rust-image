# Testing Changes

When making changes to the Abra library, it is important to test them thoroughly to ensure that they work as expected.

## Using Examples for Testing

This physically tests the public API by running the examples located in the `/apps/examples/<feature>` directory. Each example demonstrates how to use different parts of the Abra library, allowing you to verify that your changes work correctly in real-world scenarios.

The code runs from the root of the repository, so input files are referenced from the execution point. Input images are located in the `/assets` directory and they are to be output into the `/out` directory.

Always read the `/out/<image-name>` file after running an example to verify that the output is as expected. For example if you made a gradient make sure the gradient appears correctly in the output image. Or if you placed two images on top of each other, ensure that the resulting image reflects the intended composition.

If the result is not as expected, review the library changes and the example code to identify any issues. Make necessary adjustments and re-run the example to verify the corrections.

## Example folder structure

An example is structured as follows (use an existing example as a template for creating new ones):

```text
assets
out
apps/examples/<feature>/
├── Cargo.toml
└── main.rs
```
