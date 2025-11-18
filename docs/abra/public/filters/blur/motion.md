# Motion Blur Filter

## Decision Tree
1. New primitive pixel sampling technique (directional accumulation) → Core.
2. Cannot be composed from existing primitives (requires directional sampling kernel) → Core.
3. Broad applicability (photo editing, compositing, effects) → Core.
4. Enables future features (e.g. velocity-based blur, streak effects) → Core.

Result: Implemented as core filter `motion_blur` in `filters/blur/`.

## API
```rust
pub fn motion_blur(image: &mut Image, p_angle_degrees: f32, p_distance: u32)
```
- `p_angle_degrees`: Direction of blur in degrees (0° = right, 90° = down).
- `p_distance`: Length of streak in pixels (>=1). Zero is a no-op.

## Behavior
Creates a symmetric linear blur by sampling along a line oriented by `p_angle_degrees` spanning `p_distance` pixels. Uses bilinear sampling for smoother sub-pixel results. Alpha is blurred equally.

## Performance Notes
- Complexity: O(p_distance * width * height). Each extra pixel of distance adds one sample per output pixel.
- Memory: One output buffer + source snapshot (cloned RGBA). ~8 * width * height bytes.
- Parallelization: Per-pixel accumulation parallelized via `rayon` (`par_chunks_mut`).
- SIMD/GPU: Future optimization could batch sample fetches or port to GPU path by converting the loop to a compute kernel.
- Early exit for `p_distance == 0` keeps overhead minimal when disabled.

## Extensibility
Potential future optional parameters (not yet added to keep surface minimal):
- Sample spacing (>1 for sparser fast approximation)
- Directional falloff (e.g. exponential weighting)
- Center bias (leading/trailing smear only)

## Example Usage
```rust
use abra::{image::Image, filters::blur};
let mut img = Image::new_from_path("assets/bikini.jpg");
blur::motion_blur(&mut img, 30.0, 40);
img.save("out/motion_blur.png", None);
```

## Rationale
A common photographic / compositing primitive absent from current blur set (box, average, gaussian, lens). Adds directional streak capability without resorting to manual multi-pass transforms.
