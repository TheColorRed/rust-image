---
title: Apply options
order: 7
outline: deep
---

# Apply options

`ApplyOptions` controls where an operation runs. Builder-based adjustments and filters apply to the complete image by default. Call `with_options` before `apply` to restrict one with an `Area`, a mask, or both.

```rust
use abra::adjustments::prelude::levels;
use abra::options::prelude::ApplyOptions;
use abra::prelude::*;

let area = Area::rect((100, 100), (300, 220));
let mut image = Image::read("assets/photo.jpg")?;
let adjustment = levels::brightness(35)
  .with_options(ApplyOptions::new().with_area(area));

adjustment.apply(&mut image);
```

`apply` borrows the configured builder, so the same adjustment can be reused for more than one image.

## Apply to an area

Create an `Area` and pass it through `ApplyOptions::with_area`:

```rust
use abra::adjustments::prelude::levels::brightness;
use abra::options::prelude::ApplyOptions;
use abra::prelude::*;

let mut image = Image::read("assets/photo.jpg")?;
let area = Area::rect((100, 100), (300, 220)).with_feather(40);
let options = ApplyOptions::new().with_area(area);

brightness(35).with_options(options).apply(&mut image);
image.write("out/targeted-brightness.png", None)?;
```

The area limits the operation to its closed shape. `with_feather` softens the boundary in pixels so the adjusted and untouched regions transition smoothly.

## Apply to the whole image

Call `apply` directly when an adjustment should affect the complete image:

```rust
use abra::adjustments::prelude::color;

let mut image = Image::read("assets/photo.jpg")?;
color::grayscale().apply(&mut image);
```

For example, a configured adjustment can be applied to several images without rebuilding its options:

```rust
let exposure = levels::exposure(1.0);
exposure.apply(&mut first_image);
exposure.apply(&mut second_image);
```

Filters follow the shared builder pattern. For example, `gaussian_blur(12).with_options(ApplyOptions::new().with_area(area)).apply(&mut image);` scopes the operation without changing the rest of the API.

## Use a mask

An `ApplyOptions` value can also carry a mask. Masks control the per-pixel strength of the operation and can be combined with an area:

```rust
use abra::adjustments::prelude::color;
use abra::mask::prelude::Mask;
use abra::options::prelude::ApplyOptions;
use abra::prelude::*;

let mask_image = Image::read("assets/mask.png")?;
let mask = Mask::from_image(mask_image);
let options = ApplyOptions::new().with_mask(mask);
let mut image = Image::read("assets/photo.jpg")?;

color::grayscale().with_options(options).apply(&mut image);
```

Use a grayscale mask when the operation should vary gradually across the image. The exact mask conversion and combination behavior depends on the adjustment or filter.

## Supported operation families

Regional options are supported by all current adjustment builders:

- Levels: brightness, contrast, exposure, saturation, vibrance, hue, and photo filters.
- Color: grayscale, invert, threshold, posterize, opacity, auto tone, auto color, and gradient maps.

Filters implement the same `Apply` lifecycle as the adjustments: configure the builder, then call `apply(&mut image)`. `with_options` is supported whenever the filter needs an area, mask, or both.

## API summary

| API                                   | Purpose                                  |
| ------------------------------------- | ---------------------------------------- |
| `ApplyOptions::new()`                 | Create default application options.      |
| `ApplyOptions::new().with_area(area)` | Restrict processing to an `Area`.        |
| `ApplyOptions::new().with_mask(mask)` | Modulate processing with a mask.         |
| `adjustment.with_options(options)`    | Configure a reusable adjustment builder. |
| `adjustment.apply(&mut image)`        | Apply that builder to an image.          |
