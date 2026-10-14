// Writes a radial gradient, matching `radial_gradient` on the CPU: the position is the distance from the center in units of
// the radius, capped at 1, and the color is the nearest entry of the lookup table at binding 3 (halves rounded up, like
// Rust's `round`). The input is only there for its size; the gradient does not look at it.
struct Params {
  center: vec2<f32>,
  inverse_radius: vec2<f32>,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var lut_tex: texture_2d<f32>;
// Where this image sits in the photo it is a view of: pixel `p` is the photo's pixel `origin + (p + 0.5) * scale`, in a photo
// `full` pixels across. For an image that is the whole photo, `origin` is zero and `scale` is one.
struct Window {
  origin: vec2<f32>,
  scale: f32,
  pad: f32,
  full: vec2<f32>,
  pad2: vec2<f32>,
}
@group(0) @binding(5) var<uniform> window: Window;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let photo = window.origin + (vec2<f32>(f32(gid.x) + 0.5, f32(gid.y) + 0.5)) * window.scale;
  let offset = (photo - params.center) * params.inverse_radius;
  let position = min(sqrt(dot(offset, offset)), 1.0);
  let last = i32(textureDimensions(lut_tex).x) - 1;
  let index = i32(floor(position * f32(last) + 0.5));
  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), textureLoad(lut_tex, vec2<i32>(index, 0), 0));
}
