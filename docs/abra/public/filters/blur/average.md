````markdown
# Average Blur (Core)

Decision: Core primitive under `filters/blur`.

Rationale (Feature Creation Decision Tree):
- Basic pixel kernel primitive used widely for smoothing → Core
- Minimal parameters; composes with layers/masks → Core

API
```rust
pub fn average_blur(image: &mut Image, radius: u32)
```
- `radius`: half-size of square window in pixels (>=1). Window side is `2·radius+1`.

Behavior
- Each output pixel is the mean of all RGBA samples in its `square(radius)` neighborhood (uniform weights). Edges clamp sampling.

Performance
- Complexity: O((2r+1)^2) per pixel; parallelized via `rayon`.
- Memory: Single output buffer; in-place via `image.set_rgba`.

Example
```rust
use abra::{image::Image, filters::blur};
let mut img = Image::new_from_path("assets/bikini.jpg");
blur::average_blur(&mut img, 8);
img.save("out/average_blur.png", None);
```

Notes
- Produces a uniform blur that can look boxy; prefer `gaussian_blur` for natural falloff.
````