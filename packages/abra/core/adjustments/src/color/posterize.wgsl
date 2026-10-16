@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> levels: u32;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let pos = vec2<i32>(gid.xy);
  let pixel = textureLoad(input_tex, pos, 0);
  let count = levels - 1u;
  let source = vec3<u32>(round(pixel.rgb * 255.0));
  // The CPU's float formula is this exact integer operation: round to a level, then truncate the level's byte value.
  // 255 is odd, so adding 127 implements the same nearest-level rounding without a tie case.
  let level = (source * count + vec3<u32>(127u)) / vec3<u32>(255u);
  let rgb = vec3<f32>(level * vec3<u32>(255u) / vec3<u32>(count)) / 255.0;
  textureStore(output_tex, pos, vec4<f32>(rgb, pixel.a));
}
