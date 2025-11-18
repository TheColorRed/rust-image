# ViewBox System Documentation

## Overview

The ViewBox system provides SVG-style resolution-independent path rendering. Paths are defined in an abstract coordinate space and can be rendered at any size while preserving their shape and proportions.

## Core Concept

Similar to SVG's `viewBox` attribute, paths are defined in a normalized coordinate system (e.g., 0-100 space) and then transformed to fit any target viewport size. This enables true vector graphics behavior where the same path definition can render perfectly at 50×50 pixels or 5000×5000 pixels.

## Types

### ViewBox

Defines the abstract coordinate system for path definitions.

```rust
pub struct ViewBox {
  pub x: f32,       // Minimum x coordinate
  pub y: f32,       // Minimum y coordinate
  pub width: f32,   // Width of the viewBox
  pub height: f32,  // Height of the viewBox
}
```

**Constructors:**
- `ViewBox::new(x, y, width, height)` - Create with explicit coordinates
- `ViewBox::from_dimensions(width, height)` - Create from dimensions (origin at 0,0)
- `ViewBox::square(size)` - Create square viewBox from 0,0 to size×size
- `ViewBox::unit()` - Create 1×1 unit square (0,0 to 1,1)

**Methods:**
- `scale_to_fit(viewport_width, viewport_height)` - Calculate non-uniform scale factors
- `uniform_scale_to_fit(viewport_width, viewport_height)` - Calculate uniform scale (preserves aspect ratio)
- `map_point(point, viewport_width, viewport_height, aspect_ratio)` - Transform a single point
- `aspect_ratio()` - Get the aspect ratio of the viewBox

### PreserveAspectRatio

Controls how content is scaled when viewBox and viewport have different aspect ratios.

```rust
pub enum PreserveAspectRatio {
  None,   // Stretch to fill, don't preserve aspect ratio
  Meet,   // Scale to fit within viewport (may have empty space)
  Slice,  // Scale to cover entire viewport (may crop content)
}
```

### Alignment

Controls where content is positioned when aspect ratio is preserved.

```rust
pub enum Alignment {
  Min,  // Align to left/top
  Mid,  // Center
  Max,  // Align to right/bottom
}
```

### AspectRatio

Complete aspect ratio specification combining mode and alignment.

```rust
pub struct AspectRatio {
  pub mode: PreserveAspectRatio,
  pub align_x: Alignment,
  pub align_y: Alignment,
}
```

**Constructors:**
- `AspectRatio::new(mode, align_x, align_y)` - Full control
- `AspectRatio::none()` - Stretch to fill (no preservation)
- `AspectRatio::meet()` - Fit within viewport, centered
- `AspectRatio::slice()` - Cover entire viewport, centered

## Path Integration

### transform_to_viewport

Transforms a path from viewBox coordinates to viewport coordinates.

```rust
pub fn transform_to_viewport(
  &self,
  viewbox: &ViewBox,
  viewport_width: f32,
  viewport_height: f32,
  aspect_ratio: AspectRatio,
) -> Path
```

**Returns:** A new Path with all points transformed to viewport coordinates.

### to_viewbox

Creates a ViewBox from a path's bounding box.

```rust
pub fn to_viewbox(&self) -> ViewBox
```

Useful for normalizing a path - define it at any scale, then create a matching ViewBox to enable flexible rendering.

## Usage Examples

### Basic Scaling

```rust
use abra::geometry::{Path, ViewBox, AspectRatio};

// Define heart shape in 0-100 space
let mut heart = Path::new();
heart
  .with_move_to((50.0, 20.0))
  .with_cubic_to((20.0, 0.0), (0.0, 20.0), (25.0, 50.0))
  .with_cubic_to((50.0, 80.0), (75.0, 50.0), (75.0, 50.0))
  .with_cubic_to((100.0, 20.0), (80.0, 0.0), (50.0, 20.0))
  .with_close();

let viewbox = ViewBox::new(0.0, 0.0, 100.0, 100.0);

// Render at different sizes
let heart_100 = heart.transform_to_viewport(&viewbox, 100.0, 100.0, AspectRatio::meet());
let heart_500 = heart.transform_to_viewport(&viewbox, 500.0, 500.0, AspectRatio::meet());
```

### Non-Uniform Scaling

```rust
// Stretch to fill a wide viewport
let stretched = heart.transform_to_viewport(
  &viewbox,
  800.0,
  400.0,
  AspectRatio::none()
);
```

### Alignment Control

```rust
// Fit within viewport, align to top-left
let aligned = heart.transform_to_viewport(
  &viewbox,
  500.0,
  500.0,
  AspectRatio::new(PreserveAspectRatio::Meet, Alignment::Min, Alignment::Min)
);
```

### Path Normalization

```rust
// Define path at any scale
let mut icon = Path::new();
icon
  .with_move_to((10.0, 50.0))
  .with_line_to((90.0, 50.0));

// Create viewBox from bounds
let viewbox = icon.to_viewbox();

// Now can render at any size
let icon_32 = icon.transform_to_viewport(&viewbox, 32.0, 32.0, AspectRatio::meet());
let icon_256 = icon.transform_to_viewport(&viewbox, 256.0, 256.0, AspectRatio::meet());
```

## Design Patterns

### Icon Libraries

Define icons in normalized 0-100 or 0-1000 space:

```rust
const ICON_VIEWBOX: ViewBox = ViewBox::new(0.0, 0.0, 100.0, 100.0);

fn render_icon(path: &Path, size: f32) -> Path {
  path.transform_to_viewport(&ICON_VIEWBOX, size, size, AspectRatio::meet())
}
```

### Responsive Graphics

Define shapes once, render at multiple resolutions:

```rust
let logo = create_logo_path();
let viewbox = ViewBox::square(1000.0);

// Render for different screen densities
let logo_1x = logo.transform_to_viewport(&viewbox, 100.0, 100.0, aspect);
let logo_2x = logo.transform_to_viewport(&viewbox, 200.0, 200.0, aspect);
let logo_3x = logo.transform_to_viewport(&viewbox, 300.0, 300.0, aspect);
```

### Dynamic Sizing

```rust
fn render_at_size(path: &Path, viewbox: &ViewBox, size: (f32, f32)) -> Path {
  path.transform_to_viewport(viewbox, size.0, size.1, AspectRatio::meet())
}
```

## Implementation Details

### Coordinate Transformation

The transformation process:

1. Calculate scale factors based on viewBox and viewport dimensions
2. Apply aspect ratio preservation if needed (Meet/Slice/None)
3. Calculate alignment offsets for preserved aspect ratios
4. Transform each point: `(point - viewbox_origin) * scale + offset`

### Segment Handling

All segment types are transformed:
- **Line segments:** Transform the endpoint
- **Quadratic curves:** Transform control point and endpoint
- **Cubic curves:** Transform both control points and endpoint

The start point is also transformed, maintaining the path's structure while adapting to the viewport.

### Performance Considerations

- `transform_to_viewport` creates a new Path with transformed coordinates
- Original path is immutable and can be reused
- For animation or repeated rendering, cache transformed paths if viewport size is constant
- Transformation is O(n) where n is the number of segments

## Comparison to SVG

The ViewBox system closely mirrors SVG's behavior:

| SVG                                    | Abra                                            |
| -------------------------------------- | ----------------------------------------------- |
| `viewBox="0 0 100 100"`                | `ViewBox::new(0.0, 0.0, 100.0, 100.0)`          |
| `width="500" height="500"`             | `viewport_width: 500.0, viewport_height: 500.0` |
| `preserveAspectRatio="xMidYMid meet"`  | `AspectRatio::meet()`                           |
| `preserveAspectRatio="xMinYMin slice"` | `AspectRatio::new(Slice, Min, Min)`             |
| `preserveAspectRatio="none"`           | `AspectRatio::none()`                           |

## Future Enhancements

Potential additions to the ViewBox system:

- [ ] Automatic viewport calculation from image/canvas dimensions
- [ ] ViewBox animation/interpolation for smooth transitions
- [ ] Helper methods for common icon sizes (16, 24, 32, 48, etc.)
- [ ] Integration with drawing functions to accept ViewBox directly
- [ ] ViewBox serialization for file formats
