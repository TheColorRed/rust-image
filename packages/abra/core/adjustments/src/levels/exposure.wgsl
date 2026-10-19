// Matches apply_exposure in exposure.rs: sRGB to linear, exposure and offset, gamma correction, then back to sRGB.
// params: x = exposure factor (1.4^stops), y = offset, z = 1 / gamma correction, w = unused.
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let p = textureLoad(input_tex, vec2<i32>(i32(gid.x), i32(gid.y)), 0);

  let gamma = 2.2;
  let linear = pow(p.rgb, vec3<f32>(gamma));
  let adjusted = max(linear * params.x + params.y, vec3<f32>(0.0));
  let corrected = pow(adjusted, vec3<f32>(params.z));
  let out_rgb = clamp(pow(corrected, vec3<f32>(1.0 / gamma)), vec3<f32>(0.0), vec3<f32>(1.0));

  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(out_rgb, p.a));
}
