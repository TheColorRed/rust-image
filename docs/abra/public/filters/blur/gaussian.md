````markdown
# Gaussian Blur (Core)

Decision: Core primitive under `filters/blur`.

Rationale
- Ubiquitous photographic blur with natural falloff; enabling primitive for many effects.

API
```rust
pub fn gaussian_blur(image: &mut Image, radius: u32)
```
- `radius`: kernel radius in pixels (>=1). Internally uses separable 1D Gaussian passes.

Behavior
- Two-pass separable convolution (horizontal, vertical) with a radius-derived Gaussian kernel. RGBA blurred; edges clamp.

Performance
- Complexity: ~O(r) per pass per pixel (separable) vs O(r²) naive.
- Parallelized with `rayon`. Suitable for larger radii compared to box/average.

Example
```rust
use abra::{image::Image, filters::blur};
let mut img = Image::new_from_path("assets/bikini.jpg");
blur::gaussian_blur(&mut img, 12);
img.save("out/gaussian_blur.png", None);
```

Notes
- Produces smooth, natural blur; typically preferred over box/average for quality.
````