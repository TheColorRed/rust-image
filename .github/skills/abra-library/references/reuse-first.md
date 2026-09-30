# Reuse First

**IMPORTANT:** Before writing any new function, struct, enum, trait, or module, search the workspace to see if the functionality already exists. The library should have **one** implementation of each capability, shared by every tool, filter, adjustment, effect, and feature that needs it.

Duplicate implementations make the library slower to improve (every optimization has to be done twice), harder to learn, and inconsistent in behavior.

## Workflow

Follow these steps every time you add a new tool, filter, adjustment, effect, or other feature:

1. **Break the feature into building blocks.** List the lower-level operations it needs (e.g. "sample a pixel with bilinear interpolation", "map a quad to a rectangle", "blur a mask", "blend two images", "iterate pixels in parallel within an area").
2. **Search for each building block** before writing it. See [Where to Search](#where-to-search) and [How to Search](#how-to-search).
3. **Use what exists.** If a function already does the job, call it.
4. **Extend what almost exists.** If a function does *nearly* what you need, update it rather than writing a sibling. See [Updating Existing Code](#updating-existing-code).
5. **Only then write new code**, and put it in the crate where it is most reusable, not in the crate of the feature you are building. See [Where New Code Belongs](#where-new-code-belongs).

## Where to Search

Search the whole workspace, not just the crate you are working in. Common locations for building blocks:

| Looking for...                                                                        | Search here                                                      |
| ------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Image type, channels, color type, resolution                                          | `abra/core/primitives`                                           |
| Color math: HSL/HSV/Lab conversion, `luma`, harmonies, histograms and `Bins`, pixel stats | `abra/core/primitives/src/color`                              |
| Geometry (`PointF`, `Size`, `Rect`, lines, areas, paths, quads, homography, shapes, point-in-polygon) | `abra/core/core/src/geometry`                     |
| Transforms (resize, rotate, crop, flip, warp, zoom, fit, interpolation)               | `abra/core/core/src/transform`                                   |
| Mapping each output pixel back to a source position and sampling it (`remap`)         | `abra/core/core/src/transform/interpolation.rs`                  |
| Fills and gradients                                                                   | `abra/core/core/src/color`                                       |
| Blending / compositing (`BlendMode`)                                                  | `abra/core/core/src/combine`                                     |
| Applying an operation to an area, image extensions, GPU dispatch                      | `abra/core/core/src/image`                                       |
| Loading / saving files                                                                | `abra/core/core/src/fs`                                          |
| Drawing shapes, brushes, fills                                                        | `abra/core/drawing`                                              |
| Masks                                                                                 | `abra/core/mask`                                                 |
| `ApplyOptions` and shared operation options                                           | `abra/core/options`                                              |
| Brightness, contrast, levels, color adjustments                                       | `abra/core/adjustments`                                          |
| Blur, sharpen, distort, edges, noise, smoothing, repair, kernels, sobel               | `abra/core/filters`                                              |
| Layer effects (drop shadow, stroke)                                                   | `abra/core/canvas/src/effects`                                   |
| Fonts and text                                                                        | `abra/core/typography`                                           |
| GPU context and GPU images                                                            | `abra/core/gpu`                                                  |
| Existing tools (perspective, straighten, remover)                                     | `abra/core/tools`                                                |

## How to Search

Search by **concept**, not only by the name you would give the function. The same idea is often named differently, so try several terms.

- Search for the operation: e.g. `bilinear`, `interpolat`, `sample`, `lerp`.
- Search for the math or algorithm: e.g. `homography`, `perspective`, `gaussian`, `kernel`, `convolve`, `luminance`.
- Search for the types involved: e.g. functions taking or returning `Quad`, `Area`, `PointF`, `Mask`.
- Search `pub fn` and `impl` blocks of the relevant crate to see what it already offers.
- Check `abra/abra/src/prelude.rs` and each crate's `lib.rs` to see what is exported.
- Check the docs in `apps/docs/public` for existing features.

If you find two or more existing functions that already do the same thing, point it out to the user as a cleanup candidate rather than adding a third.

## Updating Existing Code

Changing an existing function to fit a new use is expected and preferred over writing a near-copy. It is 100% okay to change parameters and update the callers.

When updating, keep the function **general-purpose**:

- Generalize, don't specialize. Add a parameter, an `enum` option, or an `impl Into<...>` input so the function covers both the old and the new use case. Do not add logic that only makes sense for the one feature you are building.
- Do not hard-code values that belong to your feature (thresholds, sizes, colors, modes). Pass them in as parameters or options with sensible defaults.
- Keep the name describing *what the function does*, not *who calls it* (e.g. `warp_quad_to_rect`, not `perspective_tool_warp`).
- Preserve existing behavior for existing callers unless changing it is the goal; update every caller if the signature changes.
- Do not degrade performance for existing callers. Execution speed is the top priority (see [code management](./code-management.md)).
- Update doc comments, tests, examples, and `apps/docs` to match the new behavior.

If the change would make the function confusing or overloaded, split it into a small shared core function plus thin callers, rather than duplicating the core logic.

### Make It Extensible While You Are There

If you need to add parameters to a function that is not very flexible, do not just append another parameter. Refactor it so the **next** feature can be added without changing its signature and without touching every caller in the codebase again.

Signs a function needs this refactor:

- You are adding a parameter to an already long parameter list.
- You are adding a `bool` flag or a magic value that switches behavior.
- Callers pass the same defaults over and over.
- Adding the feature forces edits in many unrelated callers.

Pick the tool that fits:

**Use an `enum` to support different types or modes.** New variants can be added later without changing the signature.

```ignore
// Before: every new mode means another flag or function
fn sample(p_image: &Image, p_x: f32, p_y: f32, p_bilinear: bool) -> Color

// After: add variants (e.g. Bicubic, Lanczos) without touching callers
pub enum Interpolation { Nearest, Bilinear, Bicubic }
fn sample(p_image: &Image, p_x: f32, p_y: f32, p_interpolation: Interpolation) -> Color
```

**Use a builder or options struct for optional settings.** New options get a default value, so existing callers do not change. Follow the builder rules in [patterns](./patterns.md) (`new()`, `with_*` methods returning `self`, sensible defaults).

```ignore
// Before: each new setting adds a parameter to every call site
fn drop_shadow(p_image: &mut Image, p_distance: f32, p_angle: f32, p_blur: f32, p_color: Color)

// After: new settings are added to the options with a default
let shadow = DropShadow::new().with_distance(10.0).with_blur(4.0);
drop_shadow(p_image, shadow);
```

**Use traits when different types need the same behavior.** Accept `impl Trait` (or a generic) so new types work by implementing the trait, instead of adding a new function per type.

```ignore
// Before: one function per input type
fn fill_color(p_image: &mut Image, p_color: Color)
fn fill_gradient(p_image: &mut Image, p_gradient: &Gradient)

// After: new fill sources only need to implement the trait (or become an enum variant, like `Fill`)
fn fill(p_image: &mut Image, p_fill: impl Into<Fill>)
```

**Use `impl Into<T>` for flexible inputs.** Callers can pass any type that converts, and adding a new input type only needs a new `From` impl.

```ignore
fn resize(p_image: &mut Image, p_size: impl Into<Size>)

resize(&mut image, (800, 600));
resize(&mut image, Size::new(800, 600));
```

These can be combined, e.g. an options struct whose fields are enums, taken as `impl Into<Options>` so simple callers can pass a single value.

Keep performance in mind: prefer enums and generics (`impl Trait`) in hot paths over `dyn Trait`, and resolve options once outside per-pixel loops.

## Where New Code Belongs

When new code really is required, place it where it is most reusable:

- **Generic building blocks** (math, geometry, sampling, transforms, pixel iteration, color math) belong in `abra-core` (or `primitives` for the most basic types), even if only one feature needs them today.
- **Features** (tools, filters, adjustments, effects) should be thin: they compose library functions and hold only feature-specific glue. Move reusable logic out of a tool, filter, or effect into the appropriate core or feature crate.
- Put code in the lowest crate that has everything it depends on, so more crates can use it.

## Checklist

Before finishing, confirm:

- [ ] I searched the workspace for every building block of the feature.
- [ ] I used or extended existing functions instead of writing duplicates.
- [ ] Any function I extended is still general-purpose and not tailored to my feature.
- [ ] Any function I added parameters to now uses an enum, builder/options struct, trait, or `impl Into` so future features won't require another signature change.
- [ ] New reusable logic lives in a core/shared crate, not inside the feature.
- [ ] Callers, docs, and examples were updated for any changed signatures.
