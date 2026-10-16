@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

// The fixed 3 by 3 kernel in blur.rs, with clamped edges. Its integer weights add to 16.
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) { return; }

  var sum = vec4<u32>(0u);
  for (var y = -1; y <= 1; y = y + 1) {
    for (var x = -1; x <= 1; x = x + 1) {
      let weight = select(1u, 2u, x == 0) * select(1u, 2u, y == 0);
      let sample_pos = clamp(pos + vec2<i32>(x, y), vec2<i32>(0), dims - vec2<i32>(1));
      sum += vec4<u32>(round(textureLoad(input_tex, sample_pos, 0) * 255.0)) * weight;
    }
  }
  textureStore(output_tex, pos, vec4<f32>((sum + vec4<u32>(8u)) / vec4<u32>(16u)) / 255.0);
}
