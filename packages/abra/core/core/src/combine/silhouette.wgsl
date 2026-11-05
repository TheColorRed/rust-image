// Turns the input into a flat color that keeps its shape: every pixel gets the color, and its alpha is kept, except that an alpha above
// `solid_above` becomes fully opaque, as the CPU does (`if alpha > solid_above { 255 } else { alpha }`).
struct Params {
  r: u32,
  g: u32,
  b: u32,
  solid_above: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  var alpha = u32(round(textureLoad(input_tex, pos, 0).a * 255.0));
  if (alpha > params.solid_above) {
    alpha = 255u;
  }
  let color = vec3<f32>(f32(params.r), f32(params.g), f32(params.b)) / 255.0;
  textureStore(output_tex, pos, vec4<f32>(color, f32(alpha) / 255.0));
}
