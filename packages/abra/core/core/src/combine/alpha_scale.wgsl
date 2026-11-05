// Multiplies the alpha by a factor and drops the fraction, as the CPU does (`(alpha as f32 * factor) as u8`).
struct Params {
  factor: f32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let pixel = textureLoad(input_tex, pos, 0);
  let alpha = floor(round(pixel.a * 255.0) * params.factor);
  textureStore(output_tex, pos, vec4<f32>(pixel.rgb, clamp(alpha, 0.0, 255.0) / 255.0));
}
