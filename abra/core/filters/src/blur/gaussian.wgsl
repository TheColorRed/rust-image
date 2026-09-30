struct Params {
  radius: i32,
  // 0 blurs horizontally, 1 blurs vertically.
  direction: u32,
  pad0: u32,
  pad1: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

// One pass of a separable Gaussian blur with the same kernel as the CPU path: sigma = radius / 2, normalized so the
// weights sum to 1, and edge pixels clamped.
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) {
    return;
  }

  let radius = params.radius;
  let sigma = f32(radius) / 2.0;
  let two_sigma_sq = 2.0 * sigma * sigma;
  var step = vec2<i32>(1, 0);
  if (params.direction == 1u) {
    step = vec2<i32>(0, 1);
  }

  var sum = vec4<f32>(0.0);
  var total = 0.0;
  for (var k = -radius; k <= radius; k = k + 1) {
    let sample_pos = clamp(pos + step * k, vec2<i32>(0), dims - vec2<i32>(1));
    let weight = exp(-f32(k * k) / two_sigma_sq);
    sum = sum + textureLoad(input_tex, sample_pos, 0) * weight;
    total = total + weight;
  }
  textureStore(output_tex, pos, sum / total);
}
