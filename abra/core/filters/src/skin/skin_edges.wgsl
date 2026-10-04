@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

// Pass 1 of the GPU version of `SmoothSkin` (see `passes` in skin.rs). Measures how strong the edges are at each pixel,
// as `sobel_magnitude` does on the CPU, and writes how much to keep the pixel out of the smoothing into the alpha
// channel, as `smoothstep(start, full, edge)`. The color channels are passed along unchanged.
//
// The uniform is one u32 and two f32s, 12 bytes:
//   step:  the distance between the kernel's taps, in pixels, so a large photo is measured as if it were the reference
//          size, without its grain being multiplied too.
//   start: the edge strength where the protection starts.
//   full:  the edge strength where the pixel is fully protected.
struct Params {
  step: u32,
  start: f32,
  full: f32,
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
  let step = i32(params.step);

  // The Sobel kernels over the 3x3 around the pixel, on brightness, with the nearest pixel inside past the border.
  var gx = 0.0;
  var gy = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      let rgb = round(textureLoad(input_tex, clamp(pos + vec2<i32>(dx, dy) * step, vec2<i32>(0), last), 0).rgb * 255.0);
      let brightness = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
      gx += brightness * f32(dx) * (2.0 - f32(abs(dy)));
      gy += brightness * f32(dy) * (2.0 - f32(abs(dx)));
    }
  }
  // The kernels add up to 4 across an edge, so dividing by 4 gives the size of the jump in brightness levels.
  let edge = sqrt(gx * gx + gy * gy) / 4.0;
  let protection = smoothstep(params.start, params.full, edge);
  let center = textureLoad(input_tex, pos, 0);
  textureStore(output_tex, pos, vec4<f32>(center.rgb, protection));
}
