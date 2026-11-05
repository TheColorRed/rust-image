// Mixes what an adjustment layer's effects made (the input) into what was there before them (binding 4) by a strength, as the CPU does:
// `result = round(before + (after - before) * strength)` for each channel, where the strength is the opacity times the mask at that pixel,
// from 0 to 1. The mask (binding 3, its value in the red channel) has its top left corner at (`origin_x`, `origin_y`) in the photo, and where it
// does not reach the strength is 0. When `has_mask` is 0 the mask texture is only a placeholder and the strength is the opacity everywhere.
struct Params {
  opacity: f32,
  has_mask: u32,
  origin_x: i32,
  origin_y: i32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var mask_tex: texture_2d<f32>;
@group(0) @binding(4) var before_tex: texture_2d<f32>;
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
  let after = round(textureLoad(input_tex, pos, 0) * 255.0);
  let before = round(textureLoad(before_tex, pos, 0) * 255.0);
  var mask = 255.0;
  if (params.has_mask != 0u) {
    let photo = window.origin + (vec2<f32>(pos) + 0.5) * window.scale;
    let at = vec2<i32>(floor(photo)) - vec2<i32>(params.origin_x, params.origin_y);
    let size = vec2<i32>(textureDimensions(mask_tex));
    if (at.x < 0 || at.y < 0 || at.x >= size.x || at.y >= size.y) {
      mask = 0.0;
    } else {
      mask = round(textureLoad(mask_tex, at, 0).r * 255.0);
    }
  }
  let strength = mask / 255.0 * params.opacity;
  // Half a unit up and rounded down, which is how the CPU rounds a value that is never negative.
  let mixed = floor(before + (after - before) * strength + vec4<f32>(0.5));
  textureStore(output_tex, pos, clamp(mixed, vec4<f32>(0.0), vec4<f32>(255.0)) / 255.0);
}
