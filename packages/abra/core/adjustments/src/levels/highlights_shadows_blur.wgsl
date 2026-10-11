@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
struct Parameters { shadow_radius: u32, highlight_radius: u32, direction: u32, maximum: u32 }
@group(0) @binding(2) var<uniform> parameters: Parameters;
@group(0) @binding(3) var weights_tex: texture_2d<f32>;
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if gid.x >= dims.x || gid.y >= dims.y { return; }
  let pos = vec2<i32>(gid.xy);
  var result = vec4<f32>(0.0, 0.0, 0.0, 1.0);
  for (var channel = 0u; channel < 2u; channel++) {
    let radius = i32(select(parameters.shadow_radius, parameters.highlight_radius, channel == 1u));
    var sum = 0u;
    for (var offset = -radius; offset <= radius; offset++) {
      let table = vec3<u32>(round(textureLoad(weights_tex, vec2<i32>(offset + i32(parameters.maximum), i32(channel)), 0).rgb * 255.0));
      let weight = table.r + (table.g << 8u) + (table.b << 16u);
      let delta = select(vec2<i32>(offset, 0), vec2<i32>(0, offset), parameters.direction == 1u);
      let at = clamp(pos + delta, vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
      let value = u32(round(textureLoad(input_tex, at, 0)[channel] * 255.0));
      sum += value * weight;
    }
    result[channel] = f32((sum + 32768u) >> 16u) / 255.0;
  }
  textureStore(output_tex, pos, result);
}
