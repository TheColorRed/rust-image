@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

// Pass 4 of the GPU version of `SmoothSkin` (see `passes` in skin.rs). The alpha channel holds how much each pixel is
// protected as an edge, already widened. This turns it into the mask: how skin-colored the pixel is, times the part that
// is not protected. The color channels are passed along unchanged.
//
// Skin color is judged in YCbCr as `cpu_processor` does: 1 when Cb and Cr are inside their skin ranges, falling to 0 over
// `margin` outside them, and 0 for pixels too dark to have a color (below `y_dark`, complete by `y_full`).
//
// The uniform is eight f32s, 32 bytes:
//   cb_low, cb_high, cr_low, cr_high: the skin ranges of Cb and Cr.
//   margin: how far outside the ranges the likeness falls to 0.
//   y_dark, y_full: the brightness range over which a dark pixel counts more and more.
//   gain: how much the widened protection is multiplied by, so the middle of its zone stays fully protected.
struct Params {
  cb_low: f32,
  cb_high: f32,
  cr_low: f32,
  cr_high: f32,
  margin: f32,
  y_dark: f32,
  y_full: f32,
  gain: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);
  let rgb = round(p.rgb * 255.0);
  // `rgb_to_ycbcr` in primitives: full range ITU-R BT.601.
  let y = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
  let cb = 128.0 + 0.564 * (rgb.b - y);
  let cr = 128.0 + 0.713 * (rgb.r - y);
  let cb_likeness = clamp(min((cb - (params.cb_low - params.margin)) / params.margin, ((params.cb_high + params.margin) - cb) / params.margin), 0.0, 1.0);
  let cr_likeness = clamp(min((cr - (params.cr_low - params.margin)) / params.margin, ((params.cr_high + params.margin) - cr) / params.margin), 0.0, 1.0);
  let bright_enough = clamp((y - params.y_dark) / (params.y_full - params.y_dark), 0.0, 1.0);
  let kept_out = min(p.a * params.gain, 1.0);
  textureStore(output_tex, pos, vec4<f32>(p.rgb, cb_likeness * cr_likeness * bright_enough * (1.0 - kept_out)));
}
