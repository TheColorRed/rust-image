@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: f32;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  // Factor is the same formula used by the CPU/Rust implementation.
  let factor = (259.0 * (params + 255.0)) / (255.0 * (259.0 - params));

  let p = textureLoad(input_tex, vec2<i32>(i32(gid.x), i32(gid.y)), 0);

  // Convert to normalized pivot (0.5) instead of 128.0 (which is for 0..255).
  let pivot = vec3<f32>(0.5);
  let out_rgb = clamp((p.rgb - pivot) * factor + pivot, vec3<f32>(0.0), vec3<f32>(1.0));
  let out = vec4<f32>(out_rgb, p.a);

  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), out);
}
