// Multiplies the input's alpha by the mask texture at binding 3, as `Mask::multiply_alpha` does on the CPU, in whole numbers:
// `alpha = (alpha * mask + 127) / 255`. The mask is the size of the photo and its value is in the red channel.
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var mask_tex: texture_2d<f32>;
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
  let pixel = round(textureLoad(input_tex, pos, 0) * 255.0);
  let size = vec2<i32>(textureDimensions(mask_tex));
  let photo = window.origin + (vec2<f32>(pos) + 0.5) * window.scale;
  let at = clamp(vec2<i32>(floor(photo)), vec2<i32>(0), size - vec2<i32>(1));
  let mask = round(textureLoad(mask_tex, at, 0).r * 255.0);
  let alpha = floor((pixel.a * mask + 127.0) / 255.0);
  textureStore(output_tex, pos, vec4<f32>(pixel.rgb, alpha) / 255.0);
}
