````markdown
# Box Blur (Core)

Decision: Core primitive under `filters/blur`.

Rationale
- Fundamental separable blur used in many pipelines; useful as a fast baseline.

API
```rust
pub fn box_blur(image: &mut Image, radius: u32)
```
- `radius`: kernel half-width (>=1). Uses two-pass separable averaging.

Behavior
- Applies a horizontal then vertical uniform blur (separable box kernel). RGBA channels blurred equally; edge pixels clamp.

Performance
- Complexity: O(r) per pixel per pass (separable), total ~O(2r).
- Very fast and cache-friendly; parallelized per scanline with `rayon`.

Example
```rust
use abra::{image::Image, filters::blur};
let mut img = Image::new_from_path("assets/bikini.jpg");
blur::box_blur(&mut img, 16);
img.save("out/box_blur.png", None);
```

Notes
- Produces characteristic boxy bokeh; for smoother blur use `gaussian_blur`.
````