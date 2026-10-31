// Resizes an image. Each pixel of the output reads a window of the input around the spot it maps back to. The samplers are in
// sampling.wgsl, which goes in front of this file.
//
// The pass is dispatched over the output, which is a different size from the input (see `GpuPass::output`).

struct Params {
  // 0 nearest, 1 bilinear, 2 bicubic, 3 lanczos, 4 edge-directed NEDI, 5 edge-directed EDI.
  mode: u32,
  pad0: u32,
  pad1: u32,
  pad2: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

// Where an output pixel reads the input along one axis: the pixel's center, `(pixel + 0.5) * old / new - 0.5`, as the whole number at or
// below it and how far past that it is. It is worked out in whole numbers, so a pixel that sits exactly on a spot is exactly on it
// (a 32-bit float is off by a little there, and the edge-directed samplers choose differently on either side of a spot), and so it
// does not drift on a large image.
struct Axis {
  base: i32,
  fraction: f32,
  // Whether the pixel the spot is nearest to is the one after `base`. A spot exactly halfway goes away from zero, as Rust's `round` does,
  // which is decided on the whole numbers, because a GPU's division can make a half a little less than a half.
  past_half: bool,
}

fn axis_spot(pixel: u32, old_size: u32, new_size: u32) -> Axis {
  if (f32(old_size) * f32(new_size) > 1.0e9) {
    // Too big for whole numbers to hold, which needs an image of more than 30,000 pixels across.
    let spot = (f32(pixel) + 0.5) * (f32(old_size) / f32(new_size)) - 0.5;
    let below = floor(spot);
    let fraction = spot - below;
    return Axis(i32(below), fraction, select(fraction >= 0.5, fraction > 0.5, below < 0.0));
  }
  let numerator = (2 * i32(pixel) + 1) * i32(old_size) - i32(new_size);
  let denominator = 2 * i32(new_size);
  var base = numerator / denominator;
  if (numerator < 0 && base * denominator != numerator) {
    base = base - 1;
  }
  let remainder = numerator - base * denominator;
  return Axis(base, f32(remainder) / f32(denominator), select(2 * remainder >= denominator, 2 * remainder > denominator, base < 0));
}


@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let out_dims = textureDimensions(output_tex);
  if (gid.x >= out_dims.x || gid.y >= out_dims.y) {
    return;
  }
  let dims = vec2<i32>(textureDimensions(input_tex));
  let x = axis_spot(gid.x, u32(dims.x), out_dims.x);
  let y = axis_spot(gid.y, u32(dims.y), out_dims.y);
  let base = vec2<i32>(x.base, y.base);
  let fraction = vec2<f32>(x.fraction, y.fraction);

  var result = vec4<f32>(0.0);
  if (params.mode == 0u) {
    result = texel(base + select(vec2<i32>(0), vec2<i32>(1), vec2<bool>(x.past_half, y.past_half)), dims);
  } else if (params.mode <= 3u) {
    result = sample_kernel(base, fraction, params.mode, dims);
  } else if (params.mode == 4u) {
    result = sample_nedi(base, fraction, dims);
  } else {
    result = sample_edi(base, fraction, dims);
  }
  textureStore(output_tex, vec2<i32>(gid.xy), result / 255.0);
}
