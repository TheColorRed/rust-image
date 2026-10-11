@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> vibrance: f32;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);

  // Compute average
  let avg = (p.r + p.g + p.b) / 3.0;

  // Compute vibrance factor
  let max_channel = max(p.r, max(p.g, p.b));
  let amt = clamp((max_channel - avg) * 3.0, 0.0, 1.0);
  let vibrance_factor = 1.0 + (vibrance / 100.0) * amt;

  // Apply vibrance adjustment
  let result = mix(vec3<f32>(avg), p.rgb, vibrance_factor);

  // Apply saturation adjustment

  textureStore(output_tex, pos, vec4<f32>(vec3<f32>(result), p.a));
}