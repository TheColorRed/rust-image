@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

// x is the amount. The 3 by 3 cross kernel in sharpen.rs: (1 + 4 * amount) times the center minus amount times each cardinal neighbour.
@group(0) @binding(2) var<uniform> params: vec4<f32>;

// Out-of-bounds neighbours clamp to the edge, exactly as apply_kernel does on the CPU.
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) { return; }
  let last = dims - vec2<i32>(1);
  let center = textureLoad(input_tex, pos, 0);
  let left = textureLoad(input_tex, clamp(pos + vec2<i32>(-1, 0), vec2<i32>(0), last), 0);
  let right = textureLoad(input_tex, clamp(pos + vec2<i32>(1, 0), vec2<i32>(0), last), 0);
  let above = textureLoad(input_tex, clamp(pos + vec2<i32>(0, -1), vec2<i32>(0), last), 0);
  let below = textureLoad(input_tex, clamp(pos + vec2<i32>(0, 1), vec2<i32>(0), last), 0);
  textureStore(output_tex, pos, clamp(center * (1.0 + 4.0 * params.x) - (left + right + above + below) * params.x, vec4<f32>(0.0), vec4<f32>(1.0)));
}
