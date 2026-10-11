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
// The weights, 2 * radius + 1 texels, three bytes each (red is the low byte). They are whole numbers that add up to
// exactly 65536, built on the CPU by `gaussian_weights` in gaussian.rs.
@group(0) @binding(3) var weights_tex: texture_2d<f32>;

// One pass of a separable Gaussian blur in whole numbers, the same steps as the CPU path: the weights times the pixels
// around it (edge pixels clamped), rounded. Nothing here is floating point except reading and writing 8-bit values,
// so the GPU gives exactly the CPU's pixels.
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) {
    return;
  }

  let radius = params.radius;
  var step = vec2<i32>(1, 0);
  if (params.direction == 1u) {
    step = vec2<i32>(0, 1);
  }

  var sums = vec4<u32>(0u);
  for (var k = -radius; k <= radius; k = k + 1) {
    let sample_pos = clamp(pos + step * k, vec2<i32>(0), dims - vec2<i32>(1));
    let bytes = vec3<u32>(round(textureLoad(weights_tex, vec2<i32>(k + radius, 0), 0).rgb * 255.0));
    let weight = bytes.r | (bytes.g << 8u) | (bytes.b << 16u);
    let pixel = vec4<u32>(round(textureLoad(input_tex, sample_pos, 0) * 255.0));
    sums = sums + pixel * weight;
  }
  let result = (sums + vec4<u32>(32768u)) >> vec4<u32>(16u);
  textureStore(output_tex, pos, vec4<f32>(result) / 255.0);
}
