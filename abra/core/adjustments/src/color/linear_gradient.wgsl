struct Params {
  start: vec2<f32>,
  end: vec2<f32>,
  opacity: f32,
  lut_size: f32,
  pad: vec2<f32>,
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

  // Project the pixel center onto the gradient line to get the position along the gradient.
  let axis = params.end - params.start;
  let axis_len_sq = dot(axis, axis);
  var t = 0.0;
  if (axis_len_sq > 0.0) {
    t = clamp(dot(vec2<f32>(pos) + vec2<f32>(0.5) - params.start, axis) / axis_len_sq, 0.0, 1.0);
  }
  let lut_index = i32(round(t * (params.lut_size - 1.0)));
  let gradient = textureLoad(lut_tex, vec2<i32>(lut_index, 0), 0);

  // Source-over with straight (non-premultiplied) alpha.
  let base = textureLoad(input_tex, pos, 0);
  let src_a = gradient.a * params.opacity;
  let out_a = src_a + base.a * (1.0 - src_a);
  var out_rgb = vec3<f32>(0.0);
  if (out_a > 0.0) {
    out_rgb = (gradient.rgb * src_a + base.rgb * base.a * (1.0 - src_a)) / out_a;
  }
  textureStore(output_tex, pos, vec4<f32>(out_rgb, out_a));
}
