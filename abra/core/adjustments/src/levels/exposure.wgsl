@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
struct Params {
  exposure: f32,
  offset: f32,
  gamma_correction: f32,
  _pad: f32,
}

@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let p = textureLoad(input_tex, vec2<i32>(i32(gid.x), i32(gid.y)), 0);

  // guard gamma correction
  let gamma_correction_safe = select(params.gamma_correction, 0.01, params.gamma_correction <= 0.0);
  // fixed gamma for sRGB conversion
  let gamma = 2.2;
  let exposure_factor = pow(1.4, params.exposure);

  // sRGB -> linear (approximate with pow)
  let lin = pow(p.rgb, vec3<f32>(gamma));

  // apply exposure in linear space, add offset
  let adjusted = max(lin * exposure_factor + params.offset, vec3<f32>(0.0));

  // apply gamma correction in linear space
  let corrected = pow(adjusted, vec3<f32>(1.0 / gamma_correction_safe));

  // convert back to sRGB
  let encoded = clamp(pow(corrected, vec3<f32>(1.0 / gamma)), vec3<f32>(0.0), vec3<f32>(1.0));

  let out = vec4<f32>(encoded, p.a);
  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), out);
}