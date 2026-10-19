// Matches apply_contrast in contrast.rs. params is the contrast factor, computed on the CPU, and the pivot is mid gray.
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: f32;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let p = textureLoad(input_tex, vec2<i32>(i32(gid.x), i32(gid.y)), 0);

  // The CPU pivots on 128 of 255.
  let pivot = vec3<f32>(128.0 / 255.0);
  let out_rgb = clamp((p.rgb - pivot) * params + pivot, vec3<f32>(0.0), vec3<f32>(1.0));
  let out = vec4<f32>(out_rgb, p.a);

  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), out);
}
