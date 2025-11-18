# Lens Blur (Core)

Decision: Core primitive under `filters/blur`.

Rationale (Feature Creation Decision Tree):
- New sampling kernel and pixel math (aperture-shaped convolution) → Core
- Broad applicability across photo editing, compositing, effects → Core
- Clear primitive with minimal parameters; composes with layers/masks → Core
- Enables future bokeh-related effects and consistent specular handling → Core

API Summary
- Module: `abra::filters::blur`
- Function: `lens_blur(image: &mut Image, options: LensBlurOptions)`
- Types:
  - `ApertureShape = Triangle | Square | Pentagon | Hexagon | Heptagon | Octagon`
  - `IrisOptions { shape, radius, blade_curvature, rotation }`
  - `SpecularOptions { brightness, threshold }`
  - `NoiseDistribution = Uniform | Gaussian`
  - `NoiseOptions { amount, distribution }`
  - `LensBlurOptions { iris, specular: Option, noise: Option, samples }`

Parameters
- Iris
  - Shape: triangle, square through octagon
  - Radius: kernel radius in pixels
  - Blade curvature: 0.0 (polygonal) → 1.0 (circular)
  - Rotation: aperture rotation in radians
- Specular Highlights
  - Brightness: multiplier (>=1.0) to boost bright samples
  - Threshold: luminance threshold [0.0,1.0]
- Noise
  - Amount: [0.0,1.0] relative to 255 range
  - Distribution: `Uniform` or `Gaussian`
- Sampling
  - `samples`: number of samples per pixel (default 32). Higher is smoother.

Implementation Notes
- Area-uniform sampling via low-discrepancy sequence, mapped to blended polygon/circle aperture boundary.
- Bilinear sampling of source with clamped edges.
- Specular boost applied per-sample above threshold.
- Optional post-blur noise (uniform or Gaussian) to reduce banding.

Example
```rust
use abra::{filters::blur::{lens_blur, LensBlurOptions, IrisOptions, ApertureShape, SpecularOptions, NoiseOptions, NoiseDistribution}, image::Image};

let mut img = Image::new_from_path("assets/bikini.jpg");
let opts = LensBlurOptions {
  iris: IrisOptions { shape: ApertureShape::Hexagon, radius: 12, blade_curvature: 0.4, rotation: 0.2 },
  specular: Some(SpecularOptions { brightness: 1.6, threshold: 0.8 }),
  noise: Some(NoiseOptions { amount: 0.02, distribution: NoiseDistribution::Gaussian }),
  samples: 48,
};
lens_blur(&mut img, opts);
img.save("out/lens_blur.png", None);
```

Performance
- Complexity: O(samples) per pixel; parallelized across pixels with Rayon.
- Recommended `samples` 32–64 for quality/speed balance; adjust per radius/content.
