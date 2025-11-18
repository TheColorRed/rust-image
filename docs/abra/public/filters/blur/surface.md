# Surface Blur (Core)

Decision: Core primitive under `filters/blur`.

Rationale (Feature Creation Decision Tree):
- New edge-aware pixel math (conditional neighborhood averaging) → Core
- Broad utility for photo retouching, denoising, and effects → Core
- Clear primitive with minimal parameters; composes cleanly → Core

API Summary
- Module: `abra::filters::blur`
- Function: `surface_blur(image: &mut Image, radius: u32, threshold: u8)`

Parameters
- `radius` (pixels): neighborhood half-size (>= 1). Uses a square window of side `2·radius+1`.
- `threshold` (1–255): edge sensitivity. Neighbor pixels whose per-channel max absolute difference from the center exceeds `threshold` are excluded from the average. Lower values preserve edges more strongly; higher values approach a box blur.

Implementation Notes
- For each pixel, iterate its neighborhood and accumulate only samples where `max(|nr-cr|, |ng-cg|, |nb-cb|) ≤ threshold`.
- The alpha channel is averaged over the accepted samples to keep compositing behavior consistent.
- Clamp sampling at image boundaries; parallelized across pixels with Rayon.

Example
```rust
use abra::{filters::blur::surface_blur, image::Image};

let mut img = Image::new_from_path("assets/bikini.jpg");
surface_blur(&mut img, 16, 25); // gentle smoothing while preserving edges
img.save("out/surface_blur.png", None);
```

Performance
- Complexity: O((2r+1)^2) per pixel; parallelized across pixels.
- Guidance: start with `radius=8..16`, `threshold=15..35`. Increase `threshold` to smooth more; increase `radius` for broader, slower smoothing.

Notes
- If no neighbors pass the threshold (rare with reasonable values), the center pixel is kept.
- This is a simple, fast edge-preserving blur; future variants may add Gaussian weighting or bilateral approximations as separate primitives.
