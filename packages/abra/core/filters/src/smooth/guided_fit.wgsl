struct Params {
  epsilon: f32,
  pad0: u32,
  pad1: u32,
  pad2: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> params: Params;

// The middle pass of the guided filter. It reads the window means of brightness, mask, brightness * mask and brightness squared,
// and writes the straight line that fits the mask to the brightness in that window: mask = a * brightness + b.
//   a = covariance(brightness, mask) / (variance(brightness) + epsilon),  b = mean(mask) - a * mean(brightness).
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) {
    return;
  }
  let mean = textureLoad(input_tex, pos, 0);
  let variance = mean.w - mean.x * mean.x;
  let covariance = mean.z - mean.x * mean.y;
  let a = covariance / (variance + params.epsilon);
  let b = mean.y - a * mean.x;
  textureStore(output_tex, pos, vec4<f32>(a, b, 0.0, 1.0));
}
