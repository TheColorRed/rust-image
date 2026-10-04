@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

// The surface blur of `apply_surface_blur` in surface.rs, done with the same whole-number steps so the GPU and the CPU
// give the same pixels. For each color channel on its own, it averages the values in the square window around the pixel
// that are within `threshold` of the pixel's own value, and ignores the rest. Flat areas are blurred, and a value across
// an edge is too far away to be mixed in, so edges stay sharp.
//
// The uniform is three u32s, 12 bytes:
//   radius:    how many pixels the window reaches on each side. The window is (2 * radius + 1) pixels wide and tall.
//   threshold: how far, 0 to 255, a neighbor's value can be from the pixel's value and still be averaged in.
//   step:      the distance between the pixels it looks at, 1 for every pixel. With a step above 1 the same number of
//              pixels reaches `step` times as far, which blurs a large photo as widely as a small one for the same cost.
// Pixels past the border use the nearest pixel inside, as on the CPU.
struct Params {
  radius: u32,
  threshold: u32,
  step: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let last = vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1);
  let r = i32(params.radius);
  let threshold = i32(params.threshold);
  let step = i32(params.step);

  let center = textureLoad(input_tex, pos, 0);
  let c = vec3<i32>(round(center.rgb * 255.0));

  var sum = vec3<u32>(0u);
  var count = vec3<u32>(0u);
  for (var dy = -r; dy <= r; dy++) {
    for (var dx = -r; dx <= r; dx++) {
      let at = clamp(pos + vec2<i32>(dx, dy) * step, vec2<i32>(0), last);
      let v = vec3<i32>(round(textureLoad(input_tex, at, 0).rgb * 255.0));
      // Counted only where this channel's value is close enough to the pixel's own.
      let near = abs(v - c) <= vec3<i32>(threshold);
      sum += select(vec3<u32>(0u), vec3<u32>(v), near);
      count += select(vec3<u32>(0u), vec3<u32>(1u), near);
    }
  }
  // The pixel itself is always in its own window, so every count is at least 1. Whole-number division, as on the CPU.
  let averaged = vec3<f32>(sum / count) / 255.0;
  textureStore(output_tex, pos, vec4<f32>(averaged, center.a));
}
