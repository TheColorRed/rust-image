// Blends the texture at binding 3 over the input with the mode in the uniform. This file is joined after
// `blend_modes.wgsl` at compile time, which holds `composite` and every mode.
//
// `params.opacity` is the opacity from 0 to 1, on top of the top texture's own alpha, and `params.mode` is the mode, its index in
// `BlendMode::ALL`. The top texture's top left corner is at (`offset_x`, `offset_y`) in the photo, and it can be any size: where it does not
// reach, the input is left as it is.
struct Params {
  opacity: f32,
  mode: u32,
  offset_x: i32,
  offset_y: i32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var top_tex: texture_2d<f32>;
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
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let dst = round(textureLoad(input_tex, pos, 0) * 255.0);
  // The top is read where the photo is, so a zoomed view of the photo shows the same blend as the whole of it.
  let top_size = vec2<i32>(textureDimensions(top_tex));
  let photo = window.origin + (vec2<f32>(pos) + 0.5) * window.scale;
  let top_at = vec2<i32>(floor(photo)) - vec2<i32>(params.offset_x, params.offset_y);
  if (top_at.x < 0 || top_at.y < 0 || top_at.x >= top_size.x || top_at.y >= top_size.y) {
    textureStore(output_tex, pos, dst / 255.0);
    return;
  }
  let src = round(textureLoad(top_tex, top_at, 0) * 255.0);
  textureStore(output_tex, pos, composite(dst, src, params.opacity, params.mode) / 255.0);
}
