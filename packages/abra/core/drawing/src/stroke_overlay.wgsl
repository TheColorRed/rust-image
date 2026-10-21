// Draws a line through a list of points, matching `stroke_overlay` on the CPU. Every pixel finds how far it is from the pieces of the
// line near it, and that distance (against how far the brush reaches there, which can change along the line, and how soft its edge is)
// says how much of the color shows; the most of any piece is kept. The input is only there for its size. Pixels away from the line are
// transparent, so the result is made to be blended over a photo.
//
// `points` are in photo pixels, and each one's `z` is its radius (half the brush's width there) and its `w` its opacity, which change
// evenly between two points.
// A grid is laid over the box around the line (`grid`: its top left corner and the size of a cell, with `cols` and `rows` cells), and the
// texture at binding 3 has a row for each cell: a count of the pieces of the line that reach into it (255 for every piece), then which
// pieces they are, a byte each. A pixel only tests the pieces in its cell, so what it costs does not grow with the length of the line.
// `window` says where this image sits in the photo, so the line is the same however the photo is zoomed.
struct Params {
  // The brush color, straight (not multiplied by its alpha), each from 0 to 1.
  color: vec4<f32>,
  count: u32,
  // How much of the radius the fade at the edge covers, from 0 (a sharp edge) to 1. The fade is never narrower than a pixel, so the
  // edge is never jagged.
  softness: f32,
  cols: u32,
  rows: u32,
  // The top left corner of the grid in x and y, and the size of a cell in z.
  grid: vec4<f32>,
  points: array<vec4<f32>, 256>,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var cells_tex: texture_2d<f32>;
struct Window {
  origin: vec2<f32>,
  scale: f32,
  pad: f32,
  full: vec2<f32>,
  pad2: vec2<f32>,
}
@group(0) @binding(5) var<uniform> window: Window;

// A byte of a cell's row: the first is how many pieces it lists, and the ones after are which.
fn cell_byte(cell: u32, at: u32) -> u32 {
  let texel = textureLoad(cells_tex, vec2<i32>(i32(at / 4u), i32(cell)), 0);
  return u32(texel[at % 4u] * 255.0 + 0.5);
}

// How much of the color shows at a distance from the line where it reaches this far and is this opaque. A radius of nothing covers nothing.
fn covered(distance_from_line: f32, radius: f32, opacity: f32) -> f32 {
  if (radius <= 0.0) {
    return 0.0;
  }
  return clamp((radius - distance_from_line) / max(params.softness * radius, 1.0) + 0.5, 0.0, 1.0) * opacity;
}

// How much of the color shows at `p` from one piece of the line: the one from point `piece` to the next, or the dot of a single point.
fn covered_by_piece(p: vec2<f32>, piece: u32) -> f32 {
  let a = params.points[piece];
  let b = params.points[min(piece + 1u, params.count - 1u)];
  let ab = b.xy - a.xy;
  let length_squared = dot(ab, ab);
  var along = 0.0;
  if (length_squared > 0.0) {
    along = clamp(dot(p - a.xy, ab) / length_squared, 0.0, 1.0);
  }
  return covered(distance(p, a.xy + ab * along), mix(a.z, b.z, along), mix(a.w, b.w, along));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let at = vec2<i32>(i32(gid.x), i32(gid.y));
  if (params.count == 0u) {
    textureStore(output_tex, at, vec4<f32>(0.0));
    return;
  }
  let photo = window.origin + vec2<f32>(f32(gid.x) + 0.5, f32(gid.y) + 0.5) * window.scale;
  // Off the grid, which is the box around the line as wide as the brush and its fade at their widest, there is no color.
  let on_grid = (photo - params.grid.xy) / params.grid.z;
  if (on_grid.x < 0.0 || on_grid.y < 0.0 || on_grid.x >= f32(params.cols) || on_grid.y >= f32(params.rows)) {
    textureStore(output_tex, at, vec4<f32>(0.0));
    return;
  }
  let cell = u32(on_grid.y) * params.cols + u32(on_grid.x);
  let listed = cell_byte(cell, 0u);
  var coverage = 0.0;
  if (listed == 255u) {
    // A cell with too many pieces to list checks them all.
    let pieces = max(params.count, 2u) - 1u;
    for (var piece = 0u; piece < pieces; piece = piece + 1u) {
      coverage = max(coverage, covered_by_piece(photo, piece));
    }
  } else {
    for (var k = 0u; k < listed; k = k + 1u) {
      coverage = max(coverage, covered_by_piece(photo, cell_byte(cell, 1u + k)));
    }
  }
  let alpha = coverage * params.color.a;
  if (alpha <= 0.0) {
    textureStore(output_tex, at, vec4<f32>(0.0));
  } else {
    textureStore(output_tex, at, vec4<f32>(params.color.rgb, alpha));
  }
}
