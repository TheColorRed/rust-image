struct Params {
  amount: f32,
  // 0 is uniform and 1 is Gaussian (Box-Muller), matching NoiseDistribution.
  distribution: u32,
  pad0: u32,
  pad1: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

// Thomas Wang's integer mix, kept in lockstep with add_noise.rs so a pixel's noise stays stable between renders.
fn hash3(u: u32, v: u32, w: u32) -> u32 {
  var value = u * 374761393u ^ v * 668265263u ^ w * 2246822519u;
  value = value ^ (value >> 13u);
  value = value * 1274126177u;
  return value ^ (value >> 16u);
}

fn rand01(seed: u32) -> f32 {
  return f32(seed) / 4294967295.0;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let first = hash3(gid.x, gid.y, 0u);
  let second = hash3(gid.x ^ 0x9E3779B9u, gid.y ^ 0x85EBCA6Bu, 0xC2B2AE35u);
  var noise = (rand01(first) * 2.0 - 1.0) * params.amount;
  if (params.distribution == 1u) {
    let radius = sqrt(-2.0 * log(max(rand01(first), 0.0000001)));
    noise = radius * cos(6.28318530718 * rand01(second)) * params.amount;
  }
  let input = textureLoad(input_tex, vec2<i32>(gid.xy), 0);
  let rgb = clamp(input.rgb + vec3<f32>(noise * 3.0 / 255.0), vec3<f32>(0.0), vec3<f32>(1.0));
  textureStore(output_tex, vec2<i32>(gid.xy), vec4<f32>(rgb, input.a));
}
