// The same whole-number steps as `draw` in linear_gradient.rs, so the GPU gives exactly the CPU's pixels: the position
// along the gradient is the integer line `a * x + b * y + c` (in units of 2^-shift table texels), and the color is put
// over the pixel with source-over and straight alpha in whole numbers.
struct Params {
  a: i32,
  b: i32,
  c: i32,
  shift: u32,
  // The overall opacity, 0 to 255.
  opacity: u32,
  pad0: u32,
  pad1: u32,
  pad2: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var lut_tex: texture_2d<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));

  let lut_size = i32(textureDimensions(lut_tex).x);
  let half = (1i << params.shift) >> 1u;
  let position = params.a * pos.x + params.b * pos.y + params.c;
  let lut_index = clamp((position + half) >> params.shift, 0, lut_size - 1);
  let gradient = vec4<u32>(round(textureLoad(lut_tex, vec2<i32>(lut_index, 0), 0) * 255.0));
  let base = vec4<u32>(round(textureLoad(input_tex, pos, 0) * 255.0));

  let source_alpha = (gradient.a * params.opacity + 127u) / 255u;
  let out_alpha = source_alpha + (base.a * (255u - source_alpha) + 127u) / 255u;
  var out = vec4<u32>(0u);
  if (out_alpha > 0u) {
    let divisor = out_alpha * 255u;
    let sum = gradient.rgb * source_alpha * 255u + base.rgb * base.a * (255u - source_alpha);
    out = vec4<u32>(min((sum + vec3<u32>(divisor / 2u)) / vec3<u32>(divisor), vec3<u32>(255u)), out_alpha);
  }
  textureStore(output_tex, pos, vec4<f32>(out) / 255.0);
}
