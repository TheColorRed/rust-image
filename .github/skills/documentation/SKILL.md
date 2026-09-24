---
name: documentation
description: Generating documentation for the project
---

We use VitePress for generating documentation for the project.
The root of the documentation is located in the `apps/docs` directory.

You can use `npm run docs:dev -w apps/docs` to start the development server for the documentation.

The source files for the documentation are located in the `apps/docs/public` directory.

The `index.md` file should always have an order value of `order: 0` in the front matter.

When documenting shared API patterns, keep the pattern consistent within the same section. If a concept such as `ApplyOptions` is relevant to multiple entries in a section, do not mix entries that include a full example with entries that omit it. This includes cases like one blur section showing `ApplyOptions` while another blur section omits it even though the API supports it. Use a single canonical example for that concept and keep the rest of the section aligned to that same pattern instead of showing the example in some sections and not others.

Don't run `npm run docs:build`, especially when you are modifying only markdown files. There is no need to make sure it builds.
