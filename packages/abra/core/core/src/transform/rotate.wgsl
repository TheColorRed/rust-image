// Rotates an image. Each pixel of the output reads the spot of the input that turns onto it, with the samplers in sampling.wgsl, which
// goes in front of this file. The spot is a straight function of the output pixel's center, so the pass is given its two rows.
//
// The pass is dispatched over the output, which is a different size from the input when the canvas grows to hold the corners or
// is cropped (see `GpuPass::output`).

struct Params {
  // 0 nearest, 1 bilinear, 2 bicubic, 3 lanczos, 4 edge-directed NEDI, 5 edge-directed EDI.
  mode: u32,
  pad0: u32,
  pad1: u32,
  pad2: u32,
  // The input spot, in pixels with the first pixel's center at 0, of an output pixel whose center is at (x + 0.5, y + 0.5):
  // `spot.x = row_x.x * (x + 0.5) + row_x.y * (y + 0.5) + row_x.z`, and the same with `row_y` for `spot.y`.
  row_x: vec4<f32>,
  row_y: vec4<f32>,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let out_dims = textureDimensions(output_tex);
  if (gid.x >= out_dims.x || gid.y >= out_dims.y) {
    return;
  }
  let dims = vec2<i32>(textureDimensions(input_tex));
  let center = vec2<f32>(gid.xy) + vec2<f32>(0.5);
  let spot = vec2<f32>(
    params.row_x.x * center.x + params.row_x.y * center.y + params.row_x.z,
    params.row_y.x * center.x + params.row_y.y * center.y + params.row_y.z,
  );
  let below = floor(spot);
  let base = vec2<i32>(below);
  let fraction = spot - below;

  var result = vec4<f32>(0.0);
  if (params.mode == 0u) {
    // The nearest pixel, a spot exactly halfway going away from zero as Rust's `round` does.
    let nearest = select(ceil(spot - vec2<f32>(0.5)), floor(spot + vec2<f32>(0.5)), spot >= vec2<f32>(0.0));
    result = texel(vec2<i32>(nearest), dims);
  } else if (params.mode <= 3u) {
    result = sample_kernel(base, fraction, params.mode, dims);
  } else if (params.mode == 4u) {
    result = sample_nedi(base, fraction, dims);
  } else {
    result = sample_edi(base, fraction, dims);
  }
  textureStore(output_tex, vec2<i32>(gid.xy), result / 255.0);
}
