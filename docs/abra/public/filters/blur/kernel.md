````markdown
# Blur (3×3 Kernel) (Core)

Decision: Core primitive under `filters/blur`.

Rationale
- Minimal fixed-kernel blur useful for quick softening and as a building block.

API
```rust
pub fn blur(image: &mut Image)
```
- No parameters; uses a fixed 3×3 kernel equivalent to a small Gaussian-like blur.

Behavior
- Applies a single 3×3 convolution with weights:
  ```text
  1/16  2/16  1/16
  2/16  4/16  2/16
  1/16  2/16  1/16
  ```
  RGBA channels are processed equally. Edges clamp.

Performance
- Single pass O(1) per pixel; very fast.

Example
```rust
use abra::{image::Image, filters::blur};
let mut img = Image::new_from_path("assets/bikini.jpg");
blur::blur(&mut img);
img.save("out/blur_3x3.png", None);
```

Notes
- For stronger blur, run multiple passes or use `gaussian_blur` with larger radius.
````