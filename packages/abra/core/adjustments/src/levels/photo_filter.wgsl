@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

// The lens-filter tint of `apply_photo_filter` in photo_filter.rs, the mode that does not preserve luminosity: each
// channel is multiplied by the filter color in linear light, as if shot through a colored lens filter. The custom-color
// mode that keeps the photo's brightness works in hue and saturation and has no shader.
//
// The uniform is four f32s, 16 bytes:
//   color:   the filter color in LINEAR light, `srgb_u8_to_linear_f32` of each 0-255 channel. It is converted on the CPU
//            once so every pixel does not do it again, and so the GPU and the CPU start from the very same numbers.
//   density: how strongly the filter is applied, 0 to 1.
struct Params {
  color: vec3<f32>,
  density: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

// `srgb_to_linear` in primitives/src/color/to_lab.rs.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
  let low = c / 12.92;
  let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
  return select(high, low, c <= vec3<f32>(0.04045));
}

// `linear_to_srgb` in primitives/src/color/to_rgb.rs. A negative value would make `pow` undefined, so it is held at 0.
fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
  let c = max(linear, vec3<f32>(0.0));
  let low = c * 12.92;
  let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
  return select(high, low, c <= vec3<f32>(0.0031308));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);
  // The exact 0-255 value, as the CPU reads it.
  let srgb = round(p.rgb * 255.0) / 255.0;
  let tinted = srgb_to_linear(srgb) * ((1.0 - params.density) + params.density * params.color);
  // Rounded to 0-255 the way `linear_f32_to_srgb_u8` does.
  let out_rgb = round(clamp(linear_to_srgb(tinted), vec3<f32>(0.0), vec3<f32>(1.0)) * 255.0) / 255.0;
  textureStore(output_tex, pos, vec4<f32>(out_rgb, p.a));
}
