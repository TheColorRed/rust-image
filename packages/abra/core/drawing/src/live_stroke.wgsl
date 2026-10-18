// Draws a line through a list of points, matching `stroke_overlay` on the CPU. Every pixel finds how far it is from the nearest
// piece of the line, and that distance (against the brush's radius and how soft its edge is) says how much of the color shows.
// The input is only there for its size. Pixels away from the line are transparent, so the result is made to be blended over a photo.
//
// `points` are in photo pixels. `window` says where this image sits in the photo, so the line is the same however the photo is zoomed.
struct Params {
  // The brush color, straight (not multiplied by its alpha), each from 0 to 1.
  color: vec4<f32>,
  count: u32,
  // Half the brush's width, in photo pixels.
  radius: f32,
  // How wide, in photo pixels, the fade at the edge is. At least one, so the edge is never jagged.
  edge: f32,
  pad: f32,
  points: array<vec4<f32>, 256>,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
struct Window {
  origin: vec2<f32>,
  scale: f32,
  pad: f32,
  full: vec2<f32>,
  pad2: vec2<f32>,
}
@group(0) @binding(5) var<uniform> window: Window;

fn distance_to_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
  let ab = b - a;
  let length_squared = dot(ab, ab);
  var along = 0.0;
  if (length_squared > 0.0) {
    along = clamp(dot(p - a, ab) / length_squared, 0.0, 1.0);
  }
  return distance(p, a + ab * along);
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
  var nearest = distance(photo, params.points[0].xy);
  for (var i = 1u; i < params.count; i = i + 1u) {
    nearest = min(nearest, distance_to_segment(photo, params.points[i - 1u].xy, params.points[i].xy));
  }
  let coverage = clamp((params.radius - nearest) / params.edge + 0.5, 0.0, 1.0);
  let alpha = coverage * params.color.a;
  if (alpha <= 0.0) {
    textureStore(output_tex, at, vec4<f32>(0.0));
  } else {
    textureStore(output_tex, at, vec4<f32>(params.color.rgb, alpha));
  }
}
