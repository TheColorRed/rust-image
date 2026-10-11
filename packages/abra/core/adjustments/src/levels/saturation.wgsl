@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
// The saturation factor in 256ths, 0 to 512. See `apply_saturation` in saturation.rs: this does the same whole-number
// steps, so the GPU and the CPU give exactly the same pixels.
@group(0) @binding(2) var<uniform> factor: u32;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let p = textureLoad(input_tex, vec2<i32>(i32(gid.x), i32(gid.y)), 0);
  let rgb = vec3<i32>(round(p.rgb * 255.0));
  let gray = (77 * rgb.r + 150 * rgb.g + 29 * rgb.b + 128) >> 8u;
  let moved = vec3<i32>(gray) + (((rgb - vec3<i32>(gray)) * i32(factor) + vec3<i32>(128)) >> vec3<u32>(8u));
  let out_rgb = vec3<f32>(clamp(moved, vec3<i32>(0), vec3<i32>(255))) / 255.0;
  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(out_rgb, p.a));
}
